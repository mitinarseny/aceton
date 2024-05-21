use core::{
    cmp,
    fmt::Debug,
    hash::{self, Hash},
};

use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{self, AtomicU64},
        Arc,
    },
    time::Duration,
};

use aceton_core::{
    ton_utils::{contract::TonContract, wallet::WalletI},
    Asset, Dex, DexBody, DexPool, SwapPath,
};
use anyhow::{anyhow, Context};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{Local, TimeDelta, Utc};
use futures::{future, stream::FuturesUnordered, try_join, TryStreamExt};
use lazy_static::lazy_static;
use num::{rational::Ratio, BigUint, One, ToPrimitive};
use petgraph::{
    graph::{EdgeIndex, NodeIndex},
    visit::{
        DfsPostOrder, EdgeFiltered, EdgeRef, FilterEdge, GraphBase, IntoEdgeReferences, IntoEdges,
    },
    Directed, Graph,
};
use tlb::CellSerializeExt;
use tlb_ton::{
    BagOfCells, BoC, CommonMsgInfo, CurrencyCollection, ExtraCurrencyCollection, InternalMsgInfo,
    Message, MsgAddress,
};
use ton_contracts::wallet::{v4r2::V4R2, Wallet, WalletOpSendMessage};
use tonlibjson_client::ton::TonClient;
use tracing::{debug, info, instrument, warn};

use aceton_graph_utils::NegativeCycles;

use crate::ArbitragerConfig;

lazy_static! {
    static ref KEEP_MIN_TON: BigUint = 2_000_000_000u64.into(); // 2 TON
}

type G = Graph<Asset, f64, Directed>;

pub struct Arbitrager<D>
where
    D: Dex,
{
    cfg: ArbitragerConfig,
    dex: D,
    g: G,

    /// asset -> node_index
    asset2node: HashMap<Asset, NodeIndex>,

    /// edge_index -> pool_id
    edge2pool: Vec<<D::Pool as DexPool>::ID>,

    /// pool_id -> (pool, edge_indexes)
    pools: HashMap<<D::Pool as DexPool>::ID, (D::Pool, [EdgeIndex; 2])>,

    query_id: AtomicU64,

    ton: TonClient,
    wallet: Wallet<V4R2>,
}

impl<D> Arbitrager<D>
where
    D: Dex,
    D::Pool: Clone,
{
    #[instrument(skip_all)]
    pub async fn new(
        cfg: ArbitragerConfig,
        ton: TonClient,
        dex: D,
        wallet: Wallet<V4R2>,
    ) -> anyhow::Result<Self> {
        let base_asset = cfg.base_asset;
        info!("resolving DEX pools...");
        let pools = dex.get_pools().await.context("DEX")?;

        let mut s = Self {
            cfg,
            dex,
            ton,
            wallet,
            g: Graph::new(),
            asset2node: Default::default(),
            edge2pool: Default::default(),
            pools: Default::default(),
            query_id: Default::default(),
        };
        info!(pools_count = pools.len(), "building DEX graph...");
        s.add_asset(base_asset);
        s.add_pools(pools);
        // info!("removing branches...");
        // s.remove_branches();
        s.g.shrink_to_fit();
        info!(
            asset_count = s.asset_count(),
            pool_count = s.pool_count(),
            "DEX graph ready",
        );
        Ok(s)
    }

    fn add_asset(&mut self, asset: Asset) -> NodeIndex {
        *self
            .asset2node
            .entry(asset)
            .or_insert_with_key(|asset| self.g.add_node(asset.clone()))
    }

    pub fn asset_count(&self) -> usize {
        self.g.node_count()
    }
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }

    fn add_pool(&mut self, pool: D::Pool) {
        if !pool.is_active() {
            return;
        }

        let pool_id = pool.id();
        let assets = pool.assets();
        let nodes = assets.map(|asset| self.add_asset(asset));

        let edges = [(0, 1), (1, 0)].map(|(i_in, i_out)| {
            self.edge2pool.push(pool_id.clone());
            self.g.add_edge(
                nodes[i_in],
                nodes[i_out],
                -pool.rate_with_fees(assets[i_in]).log2(),
            )
        });

        self.pools.insert(pool_id, (pool, edges));
    }

    fn add_pools(&mut self, pools: impl IntoIterator<Item = D::Pool>) {
        for pool in pools {
            self.add_pool(pool);
        }
    }

    // fn remove_branches(&mut self) {
    //     // TODO: this invalidates node & edge indexes
    //     let mut keep = HashSet::new();
    //     let mut nodes = DfsPostOrder::new(&self.g, self.asset2node[&self.base_asset()]);
    //     while let Some(node) = nodes.next(&self.g) {
    //         if self.g.neighbors_undirected(node).count() >= 1 {
    //             keep.insert(node);
    //         }
    //     }
    //     self.g.retain_nodes(|_, node| keep.contains(&node))
    // }

    async fn wallet_seqno(&self) -> anyhow::Result<u32> {
        let wallet = TonContract::new(self.ton.clone(), self.wallet.address());
        wallet.seqno().await
    }

    pub fn base_asset(&self) -> Asset {
        self.cfg.base_asset
    }

    fn base_asset_id(&self) -> NodeIndex {
        self.asset2node[&self.base_asset()]
    }

    pub async fn base_asset_balance(&self) -> anyhow::Result<BigUint> {
        match self.base_asset() {
            Asset::Native => {
                let state = self
                    .ton
                    .get_account_state(&self.wallet.address().to_string())
                    .await?;
                Ok((state.balance as u64).into())
            }
            Asset::Jetton(master) => {
                todo!()
            }
            Asset::ExtraCurrency { .. } => {
                return Err(anyhow!("extra currencies are not supported"))
            }
        }
    }

    fn filter_pools(
        &self,
    ) -> EdgeFiltered<&G, impl FilterEdge<<&G as IntoEdgeReferences>::EdgeRef>> {
        EdgeFiltered::from_fn(&self.g, |edge: <&G as IntoEdgeReferences>::EdgeRef| {
            // check that -log is finite
            edge.weight().is_finite()
        })
    }

    fn profitable_cycles<'a, G1>(&'a self, g: G1) -> impl Iterator<Item = SwapPath<&D::Pool>>
    where
        G1: IntoEdges<
            NodeId = <&'a G as GraphBase>::NodeId,
            EdgeRef = <&'a G as IntoEdgeReferences>::EdgeRef,
        >,
    {
        NegativeCycles::new(
            g,
            self.base_asset_id(),
            |edge| *edge.weight(),
            self.cfg.max_length,
        )
        .map(|pools| {
            let mut p = SwapPath::new(self.base_asset());
            p.extend(
                pools
                    .into_iter()
                    .map(|e| &self.pools[&self.edge2pool[e.id().index()]].0),
            );
            p
        })
    }

    async fn update_pools(&mut self) -> anyhow::Result<()> {
        let mut updated_pools = self
            .pools
            .iter_mut()
            .map(|(pool_id, (pool, edges))| {
                let dex = &self.dex;
                async move {
                    if !dex.update_pool(pool).await? {
                        return Ok(None);
                    }
                    return anyhow::Ok(Some((pool_id, &*pool, *edges)));
                }
            })
            .collect::<FuturesUnordered<_>>()
            .try_filter_map(future::ok);

        while let Some((pool_id, pool, edges)) = updated_pools.try_next().await? {
            for e in edges {
                let (index_in, _index_out) = self.g.edge_endpoints(e).unwrap();
                self.g[e] = -pool.rate_with_fees(self.g[index_in]).log2();
            }
        }
        Ok(())
    }

    fn make_steps(
        &self,
        _amount_in: &BigUint,
        path: &SwapPath<&D::Pool>,
    ) -> anyhow::Result<<D::Pool as DexPool>::Step> {
        let mut pools = path.iter_pools();

        let first = pools.next().context("empty path")?;

        let rest = pools
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .fold(None, |next, pool| Some(pool.make_step(None, next)));

        // TODO: amount_out_min
        let root = first.make_step(None, rest);
        Ok(root)
    }

    async fn make_body(
        &self,
        amount_in: &BigUint,
        path: &SwapPath<&D::Pool>,
    ) -> anyhow::Result<DexBody<D::Body>> {
        let asset_in = path.asset_in();
        if asset_in != self.base_asset() {
            return Err(anyhow!("{asset_in} is not base asset"));
        }
        let steps = self.make_steps(amount_in, path)?;
        self.dex
            .make_body(
                self.query_id.fetch_add(1, atomic::Ordering::SeqCst),
                self.base_asset(),
                amount_in.clone(),
                steps,
            )
            .await
    }

    fn make_message(
        &self,
        dst: MsgAddress,
        grams: BigUint,
        body: D::Body,
    ) -> anyhow::Result<Message<D::Body>> {
        Ok(Message {
            info: CommonMsgInfo::Internal(InternalMsgInfo {
                ihr_disabled: true,
                bounce: true,
                bounced: false,
                src: MsgAddress::NULL,
                dst,
                value: CurrencyCollection {
                    grams,
                    other: ExtraCurrencyCollection,
                },
                ihr_fee: BigUint::ZERO,
                fwd_fee: BigUint::ZERO,
                created_lt: 0,
                created_at: None,
            }),
            init: None,
            body,
        })
    }

    async fn send_external_message(
        &self,
        seqno: u32,
        message: Message<D::Body>,
    ) -> anyhow::Result<()> {
        let now = Local::now().with_timezone(&Utc);
        let expire_at = now + TimeDelta::seconds(60);

        let msg = self.wallet.create_external_message(
            expire_at,
            seqno,
            [WalletOpSendMessage {
                mode: 3,
                message: message.normalize()?,
            }],
            false,
        )?;

        let boc = BagOfCells::from_root(msg.to_cell()?);
        let packed = boc.pack(true)?;

        let tx_hash = self
            .ton
            .send_message_returning_hash(STANDARD.encode(packed).as_str())
            .await?;
        let decoded_tx_hash = STANDARD.decode(tx_hash)?;
        warn!(tx.hash = hex::encode(decoded_tx_hash), "sent tx");
        Ok(())
    }

    pub async fn run(&mut self) -> anyhow::Result<()>
    where
        D::Pool: Debug,
    {
        info!("starting main loop...");
        loop {
            info!("updating pools reserves...");
            self.update_pools().await?;
            info!("pools reserves updated");

            let (seqno, base_asset_balance) =
                try_join!(self.wallet_seqno(), self.base_asset_balance())?;
            info!(
                seqno,
                base_asset = %self.base_asset(),
                base_asset.balance = %base_asset_balance,
            );

            let amount_in = (self.cfg.amount_in_balance_coef()
                * if self.base_asset() == Asset::Native {
                    if &base_asset_balance < &*KEEP_MIN_TON {
                        warn!("too small balance");
                        continue;
                    }
                    base_asset_balance - &*KEEP_MIN_TON
                } else {
                    base_asset_balance
                })
            .to_integer();

            info!(%amount_in, "looking for profitable cycles...");
            let filtered_pools = self.filter_pools();
            let profitable_cycles = self.profitable_cycles(&filtered_pools);

            let Some(cycle) =
                profitable_cycles.max_by_key(|cycle| cycle.estimate_swap_out(amount_in.clone()))
            else {
                info!("no profitable cycles");
                continue;
            };

            let amount_out = cycle.estimate_swap_out(amount_in.clone());
            if amount_out < amount_in {
                info!(%amount_in, "the best cycle is unprofitable");
                continue;
            }

            info!("found profitable cycle!");

            let DexBody { dst, gas, body } = self.make_body(&amount_in, &cycle).await?;

            let mut profit = &amount_out - &amount_in;
            if profit <= &gas + BigUint::from(100_000_000u64) {
                info!(%profit, %gas, "profit does not cover gas");
                continue;
            }
            profit -= &gas;

            let profit_rate = Ratio::new(profit, amount_in.clone()).to_f64().unwrap();
            if profit_rate.to_f64().unwrap() < 0.05 {
                info!(
                    profit_rate_percent = format!("{:.2}", profit_rate * 100.0),
                    "too small profit percent"
                );
                continue;
            }
            info!(
                %amount_in,
                %amount_out,
                profit_rate_percent = format!("{:.2}", profit_rate * 100.0),
                %cycle,
                "found most profitable cycle",
            );

            self.send_external_message(
                seqno,
                self.make_message(
                    dst,
                    gas + if matches!(self.base_asset(), Asset::Native) {
                        amount_in
                    } else {
                        BigUint::ZERO
                    },
                    body,
                )?,
            )
            .await?;
            info!("sleeping for 60 seconds...");
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}

#[derive(Debug, Clone, Copy, Eq)]
pub struct EdgeKey(Asset, Asset);

impl EdgeKey {
    fn sorted(self) -> (Asset, Asset) {
        if self.0 > self.1 {
            return (self.1, self.0);
        }
        (self.0, self.1)
    }
}

impl PartialEq for EdgeKey {
    fn eq(&self, other: &Self) -> bool {
        self.sorted() == other.sorted()
    }
}

impl PartialOrd for EdgeKey {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.sorted().partial_cmp(&other.sorted())
    }
}

impl Hash for EdgeKey {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.sorted().hash(state)
    }
}

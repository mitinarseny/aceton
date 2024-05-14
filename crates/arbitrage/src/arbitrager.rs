use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use anyhow::{anyhow, Context};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{Local, TimeDelta, Utc};
use futures::{stream::FuturesUnordered, try_join, TryStreamExt};
use lazy_static::lazy_static;
use num::{rational::Ratio, BigUint, One, ToPrimitive};
use petgraph::{
    graph::NodeIndex,
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
use ton_contracts::{v4r2::V4R2, Wallet, WalletOpSendMessage};
use tonlibjson_client::ton::TonClient;
use tracing::{info, instrument, warn};

use aceton_core::{TonContract, WalletI};
use aceton_graph::NegativeCycles;

use crate::{ArbitragerConfig, Asset, Dex, DexBody, DexPool, SwapPath};

lazy_static! {
    static ref KEEP_MIN_TON: BigUint = 2_000_000_000u64.into(); // 2 TON
}

#[allow(type_alias_bounds)]
type G<D: Dex> = Graph<Asset, D::Pool, Directed>;

pub struct Arbitrager<D>
where
    D: Dex,
{
    cfg: ArbitragerConfig,
    dex: D,
    g: G<D>,
    asset2index: HashMap<Asset, NodeIndex>,
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
            g: Graph::new(),
            asset2index: Default::default(),
            query_id: Default::default(),
            ton,
            wallet,
        };
        info!(pools_count = pools.len(), "building DEX graph...");
        s.add_asset(base_asset);
        s.add_pools(pools);
        // info!("removing branches...");
        // s.remove_branches();
        s.g.shrink_to_fit();
        info!(
            asset_count = s.asset_count(),
            directed_pool_count = s.directed_pool_count(),
            "DEX graph ready",
        );
        Ok(s)
    }

    fn add_asset(&mut self, asset: Asset) -> NodeIndex {
        *self
            .asset2index
            .entry(asset)
            .or_insert_with_key(|asset| self.g.add_node(asset.clone()))
    }

    pub fn asset_count(&self) -> usize {
        self.g.node_count()
    }

    pub fn directed_pool_count(&self) -> usize {
        self.g.edge_count()
    }

    fn add_pool(&mut self, pool: D::Pool) {
        if !pool.reserves().into_iter().all(|r| r > &BigUint::one()) {
            return;
        }

        let assets = pool.assets().map(|asset| self.add_asset(asset));

        self.g.add_edge(assets[0], assets[1], pool.clone());
        self.g.add_edge(assets[1], assets[0], pool);
    }

    fn add_pools(&mut self, pools: impl IntoIterator<Item = D::Pool>) {
        for pool in pools {
            self.add_pool(pool);
        }
    }

    fn remove_branches(&mut self) {
        // TODO: this invalidates node & edge indexes
        let mut keep = HashSet::new();
        let mut nodes = DfsPostOrder::new(&self.g, self.asset2index[&self.base_asset()]);
        while let Some(node) = nodes.next(&self.g) {
            if self.g.neighbors_undirected(node).count() >= 1 {
                keep.insert(node);
            }
        }
        self.g.retain_nodes(|_, node| keep.contains(&node))
    }

    async fn wallet_seqno(&self) -> anyhow::Result<u32> {
        let wallet = TonContract::new(self.ton.clone(), self.wallet.address());
        wallet.seqno().await
    }

    pub fn base_asset(&self) -> Asset {
        self.cfg.base_asset
    }

    fn base_asset_id(&self) -> NodeIndex {
        self.asset2index[&self.base_asset()]
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
    ) -> EdgeFiltered<&G<D>, impl FilterEdge<<&G<D> as IntoEdgeReferences>::EdgeRef>> {
        EdgeFiltered::from_fn(&self.g, |edge: <&G<D> as IntoEdgeReferences>::EdgeRef| {
            edge.weight()
                .reserves()
                .iter()
                .all(|r| **r != BigUint::ZERO)
        })
    }

    fn profitable_cycles<'a, G1>(&'a self, g: G1) -> impl Iterator<Item = SwapPath<&D::Pool>>
    where
        G1: IntoEdges<
            NodeId = <&'a G<D> as GraphBase>::NodeId,
            EdgeRef = <&'a G<D> as IntoEdgeReferences>::EdgeRef,
        >,
    {
        NegativeCycles::new(
            g,
            self.base_asset_id(),
            |edge| -edge.weight().rate_with_fees(self.g[edge.source()]).log2(),
            self.cfg.max_length,
        )
        .map(|pools| {
            let mut p = SwapPath::new(self.base_asset());
            p.extend(pools.into_iter().map(|e| e.weight()));
            p
        })
    }

    async fn update_pools(&mut self) -> anyhow::Result<()> {
        self.g
            .edge_weights_mut()
            .map(|pool| self.dex.update_pool(pool))
            .collect::<FuturesUnordered<_>>()
            .try_collect::<()>()
            .await
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
                self.query_id.fetch_add(1, Ordering::SeqCst),
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
        info!(?msg);

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
            info!("sleeping for 90 seconds...");
            tokio::time::sleep(Duration::from_secs(90)).await;
        }
    }
}

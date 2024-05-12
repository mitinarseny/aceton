use std::{
    collections::{HashMap, HashSet},
    f32::consts::PI,
    fmt::Debug,
    sync::Arc,
};

use aceton_graph::NegativeCycles;
use anyhow::Context;
use futures::{stream::FuturesUnordered, TryStreamExt};
use num::{rational::Ratio, BigUint, ToPrimitive};
use petgraph::{
    graph::{EdgeReference, NodeIndex},
    visit::{
        DfsPostOrder, EdgeFiltered, EdgeRef, FilterEdge, GraphBase, IntoEdgeReferences, IntoEdges,
    },
    Directed, Graph,
};
use tlb_ton::Message;
use tracing::{info, instrument, warn};

use crate::{ArbitragerConfig, Asset, Dex, DexPool, SwapPath};

type G<D: Dex> = Graph<Asset, D::Pool, Directed>;

pub struct Arbitrager<D>
where
    D: Dex,
{
    cfg: ArbitragerConfig,
    dex: D,
    g: G<D>,
    asset2index: HashMap<Asset, NodeIndex>,
}

impl<D> Arbitrager<D>
where
    D: Dex,
    D::Pool: Clone,
{
    #[instrument(skip_all)]
    pub async fn new(cfg: ArbitragerConfig, dex: D) -> anyhow::Result<Self> {
        let base_asset = cfg.base_asset;
        info!("getting DEX pools...");
        let pools = dex.get_pools().await.context("DEX")?;
        info!("got {} pools", pools.len());
        let mut s = Self {
            cfg,
            dex,
            g: Graph::new(),
            asset2index: Default::default(),
        };
        info!("building graph from {} pools...", pools.len());
        s.add_asset(base_asset);
        s.g.reserve_exact_nodes(pools.len() * 2);
        s.add_pools(pools);
        info!("removing branches...");
        // s.remove_branches();
        s.g.shrink_to_fit();
        info!(
            "graph ready: {} assets, {} directed pools",
            s.g.node_count(),
            s.g.edge_count(),
        );
        Ok(s)
    }

    fn add_asset(&mut self, asset: Asset) -> NodeIndex {
        *self
            .asset2index
            .entry(asset)
            .or_insert_with_key(|asset| self.g.add_node(asset.clone()))
    }

    fn add_pool(&mut self, pool: D::Pool) {
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
        let mut nodes = DfsPostOrder::new(&self.g, self.asset2index[&self.cfg.base_asset]);
        while let Some(node) = nodes.next(&self.g) {
            if self.g.neighbors_undirected(node).count() >= 1 {
                keep.insert(node);
            }
        }
        self.g.retain_nodes(|_, node| keep.contains(&node))
    }

    // fn iter_profitable_cycles(&self) -> impl Iterator<Item = SwapPath<&DP>> + '_ {
    //     self.g
    //         .profitable_cycles(self.cfg.base_asset, self.cfg.max_length)
    // }

    fn make_message(&self, amount_in: BigUint, path: SwapPath<&D::Pool>) -> Message {
        let pools: Vec<_> = path.iter_pools().collect();
        let mut next = None;
        for pool in pools[1..].into_iter().rev() {
            next = Some(pool.make_step(None, next));
        }
        // TODO: amount_out_min
        let root = pools[0].make_step(None, next);

        self.dex.make_message(self.cfg.base_asset, amount_in, root)
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
            self.asset2index[&self.cfg.base_asset],
            |edge| -edge.weight().rate_with_fees(self.g[edge.source()]).log2(),
            self.cfg.max_length,
        )
        .map(|pools| {
            let mut p = SwapPath::new(self.cfg.base_asset);
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

    pub async fn run(&mut self) -> anyhow::Result<()>
    where
        D::Pool: Debug,
    {
        let TON: BigUint = 1_000_000_000u64.into();
        let amount_in: BigUint = TON.clone() * 100u32;

        info!("looking for profitable cycles...");

        loop {
            info!("updating pools reserves...");
            self.update_pools().await?;
            info!("pools updated!");
            let filtered_pools = self.filter_pools();

            let profitable_cycles = self.profitable_cycles(&filtered_pools);
            let Some(cycle) = profitable_cycles
                // .inspect(|cycle| {
                //     let amount_out = cycle.estimate_swap_out(amount_in.clone());
                //     info!("cycle candidate (out {}): {}", amount_out, cycle);
                // })
                .max_by_key(|cycle| cycle.estimate_swap_out(amount_in.clone()))
            else {
                warn!("no profitable cycles by rate");
                continue;
            };

            let amount_out = cycle.estimate_swap_out(amount_in.clone());
            if amount_out < amount_in {
                warn!("no profitable cycles by amount");
                continue;
            }
            let profit = &amount_out - &amount_in;
            let profit_rate = Ratio::new(profit, amount_in.clone()).to_f64().unwrap();
            info!(
                "most profit: {:.2}%, out: {}, cycle: {}",
                profit_rate * 100.0,
                amount_out,
                cycle
            );
        }
    }

    pub fn find_and_prepare_msg(&self) {}
}

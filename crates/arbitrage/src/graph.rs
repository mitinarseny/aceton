use std::{collections::HashMap, marker::PhantomData};

use aceton_graph::NegativeCycles;
use fraction::{BigUint, Decimal};
use impl_tools::autoimpl;
use petgraph::{
    algo::dijkstra,
    graph::{EdgeReference, NodeIndex},
    visit::{EdgeRef, GraphBase, IntoEdges},
    Directed, Graph,
};

use crate::{Asset, DexPool, SwapPath};

#[autoimpl(Default)]
pub struct DexGraph<DP> {
    g: Graph<Asset, DP, Directed>,
    asset2index: HashMap<Asset, NodeIndex>,
}

impl<DP> DexGraph<DP>
where
    DP: DexPool,
{
    pub fn add_asset(&mut self, asset: Asset) -> NodeIndex {
        *self
            .asset2index
            .entry(asset)
            .or_insert_with_key(|asset| self.g.add_node(asset.clone()))
    }

    pub fn add_pool(&mut self, pool: DP)
    where
        DP: Clone,
    {
        let assets = pool.assets().map(|asset| self.add_asset(asset));
        self.g.add_edge(assets[0], assets[1], pool.clone());
        self.g.add_edge(assets[1], assets[0], pool);
    }

    pub fn from_pools(pools: impl IntoIterator<Item = DP>) -> Self
    where
        DP: Clone,
    {
        let mut g = Self::default();
        for pool in pools {
            g.add_pool(pool);
        }
        g
    }

    pub fn profitable_cycles(
        &self,
        asset_in: Asset,
        max_length: impl Into<Option<usize>>,
    ) -> impl Iterator<Item = SwapPath<&DP>> + '_
    where
        DP: Clone,
    {
        self.asset2index
            .get(&asset_in)
            .copied()
            .map(|asset_in_index| {
                NegativeCycles::new(
                    &self.g,
                    asset_in_index,
                    |edge| -edge.weight().rate_with_fees(self.g[edge.source()]).log2(),
                    max_length,
                )
            })
            .into_iter()
            .flatten()
            .map(move |pools| {
                let mut p = SwapPath::new(asset_in);
                p.extend(pools.into_iter().map(|e| e.weight()));
                p
            })
    }
}

impl<DP> Extend<DP> for DexGraph<DP>
where
    DP: DexPool + Clone,
{
    fn extend<T: IntoIterator<Item = DP>>(&mut self, pools: T) {
        for pool in pools {
            self.add_pool(pool);
        }
    }
}

impl<DP> FromIterator<DP> for DexGraph<DP>
where
    DP: DexPool + Clone,
{
    fn from_iter<T: IntoIterator<Item = DP>>(pools: T) -> Self {
        let mut g = Self::default();
        g.extend(pools);
        g
    }
}

// pub struct ProfitableCycles<'a, DP> {
//     g: &'a G<DP>,
//     path: SwapPath<EdgeDexPool<EdgeReference<'a, DP>>>,
// }

// impl<'a, DP: 'a> Iterator for ProfitableCycles<'a, DP> {
//     type Item = &'a SwapPath<DP>;

//     fn next(&mut self) -> Option<Self::Item> {
//         let last_pool = self.path.pools.last()?;
//         let asset_out = last_pool.target();
//         // TODO: check if already a cycle
//         for e in self.g.g.edges(target) {
//             if e == last_pool {
//                 continue;
//             }

//         }
//     }
// }

// #[derive(Debug, Clone, Copy)]
// #[autoimpl(Deref using self.0)]
// #[autoimpl(DerefMut using self.0)]
// pub struct EdgeDexPool<P>(pub P);

// impl<P> DexPool for EdgeDexPool<P>
// where
//     P: EdgeRef,
//     P::Weight: DexPool,
// {
//     fn assets(&self) -> [Asset; 2] {
//         self.weight().assets()
//     }

//     fn reserves(&self) -> &[BigUint; 2] {
//         self.weight().reserves()
//     }

//     fn trade_fees(&self) -> [Decimal; 2] {
//         self.weight().trade_fees()
//     }
// }

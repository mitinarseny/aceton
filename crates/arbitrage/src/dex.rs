use std::collections::HashMap;

use aceton_dedust::{DedustAsset, DedustPool};
use impl_tools::autoimpl;
use itertools::Itertools;

use num_traits::ToPrimitive;
use petgraph::{graph::NodeIndex, visit::EdgeRef, Graph};

#[derive(Default)]
#[autoimpl(Deref using self.g)]
pub struct DEX {
    // TODO: add Ston.fi pools connecting only last Steps (return to TON)
    pub g: Graph<DedustAsset, f64>,
    asset2id: HashMap<DedustAsset, NodeIndex>,
    // pool2id: HashMap<MsgAddress, EdgeIndex>,
}

impl DEX {
    pub fn add_asset(&mut self, asset: DedustAsset) -> NodeIndex {
        *self
            .asset2id
            .entry(asset)
            .or_insert_with_key(|asset| self.g.add_node(asset.clone()))
    }

    pub fn add_pool(&mut self, pool: DedustPool) {
        let assets = pool.assets.map(|asset| self.add_asset(asset));
        for (i, j) in [(0, 1), (1, 0)] {
            self.g.add_edge(
                assets[i],
                assets[j],
                -(pool.reserves[j].to_f64().unwrap() / pool.reserves[i].to_f64().unwrap()
                    * (1.0 - pool.trade_fee / 100.0))
                    .log2(),
            );
        }
    }

    pub fn index_by_asset(&self, asset: &DedustAsset) -> Option<NodeIndex> {
        self.asset2id.get(asset).copied()
    }

    pub fn asset_by_index(&self, index: NodeIndex) -> Option<&DedustAsset> {
        self.g.node_weight(index)
    }

    pub fn calculate_path_rate(&self, path: impl IntoIterator<Item = NodeIndex>) -> f64 {
        path.into_iter()
            .tuple_windows()
            .map(|(a, b)| *self.g.edges_connecting(a, b).next().unwrap().weight())
            .sum()
    }

    fn do_find_paths(
        &self,
        oppos: &mut Vec<Vec<NodeIndex>>,
        path: &mut Vec<NodeIndex>,
        length: usize,
    ) {
        if path.len() > length {
            return;
        }
        if path.len() == length && path.first() == path.last() {
            if self.calculate_path_rate(path.iter().copied()) < 0.0 {
                oppos.push(path.clone());
            }
        }

        for e in self.g.edges(*path.last().unwrap()) {
            let neighbor = e.target();
            if path.len() == 1 || neighbor != path[path.len() - 2] {
                path.push(neighbor);
                self.do_find_paths(oppos, path, length);
                path.pop();
            }
        }
    }

    fn find_all_paths(&self, start_asset: DedustAsset, max_length: usize) -> Vec<Vec<NodeIndex>> {
        let mut oppos = Vec::new();
        self.do_find_paths(
            &mut oppos,
            &mut [self.index_by_asset(&start_asset).unwrap()].into(),
            max_length,
        );
        oppos
    }

    pub fn find_paths(
        &self,
        start_asset: DedustAsset,
        max_length: usize,
    ) -> Vec<(Vec<DedustAsset>, f64)> {
        let paths = self.find_all_paths(start_asset, max_length);
        paths
            .into_iter()
            .map(|path| {
                let total_log_rate = self.calculate_path_rate(path.iter().copied());
                let path = path
                    .into_iter()
                    .map(|asset_index| self.asset_by_index(asset_index).unwrap().clone())
                    .collect();
                (path, (-total_log_rate).exp2())
            })
            .collect()
    }
}

impl Extend<DedustPool> for DEX {
    fn extend<T: IntoIterator<Item = DedustPool>>(&mut self, pools: T) {
        for pool in pools {
            self.add_pool(pool);
        }
    }
}

impl FromIterator<DedustPool> for DEX {
    fn from_iter<T: IntoIterator<Item = DedustPool>>(pools: T) -> Self {
        let mut g = Self::default();
        g.extend(pools);
        g
    }
}

#[cfg(test)]
mod tests {
    use aceton_dedust::DedustPoolType;
    use petgraph::dot::Dot;
    use tlb_ton::MsgAddress;

    use super::*;
    #[test]
    #[ignore]
    fn finds_best_cycle() {
        const TRADE_FEE: f64 = 0.0;
        let asset0 = DedustAsset::Native;
        println!("asset0: {asset0:?}");
        let asset1 = DedustAsset::Jetton(MsgAddress {
            workchain_id: 0,
            address: [1; 32],
        });
        println!("asset1: {asset1:?}");
        let asset2 = DedustAsset::Jetton(MsgAddress {
            workchain_id: 0,
            address: [2; 32],
        });
        println!("asset2: {asset2:?}");
        let asset3 = DedustAsset::Jetton(MsgAddress {
            workchain_id: 0,
            address: [3; 32],
        });
        println!("asset3: {asset3:?}");

        let pools = [
            DedustPool {
                address: MsgAddress::NULL,
                r#type: DedustPoolType::Volatile,
                assets: [asset0, asset1],
                trade_fee: TRADE_FEE,
                reserves: [7_610_292_159, 10_000_000_000].map(u64::into),
            },
            DedustPool {
                address: MsgAddress::NULL,
                r#type: DedustPoolType::Volatile,
                assets: [asset0, asset2],
                trade_fee: TRADE_FEE,
                reserves: [19_477_989_871, 15_000_000_000].map(u64::into),
            },
            DedustPool {
                address: MsgAddress::NULL,
                r#type: DedustPoolType::Volatile,
                assets: [asset0, asset3],
                trade_fee: TRADE_FEE,
                reserves: [21_581_678_019, 25_000_000_000].map(u64::into),
            },
            DedustPool {
                address: MsgAddress::NULL,
                r#type: DedustPoolType::Volatile,
                assets: [asset1, asset2],
                trade_fee: TRADE_FEE,
                reserves: [32_419_335_574, 19_000_000_000].map(u64::into),
            },
            DedustPool {
                address: MsgAddress::NULL,
                r#type: DedustPoolType::Volatile,
                assets: [asset1, asset3],
                trade_fee: TRADE_FEE,
                reserves: [13_612_078_451, 12_000_000_000].map(u64::into),
            },
            DedustPool {
                address: MsgAddress::NULL,
                r#type: DedustPoolType::Volatile,
                assets: [asset2, asset3],
                trade_fee: TRADE_FEE,
                reserves: [11_966_493_817, 18_000_000_000].map(u64::into),
            },
        ];

        let g = DEX::from_iter(pools);
        println!("{:?}", Dot::new(&g.g));
        let cycle = g.find_all_paths(asset0, 4);
        // assert_eq!(cycle, [[asset0, asset1, asset2]]);
    }

    #[test]
    fn finds_best_cycle_easy() {
        const TRADE_FEE: f64 = 0.4;
        let usdt = DedustAsset::Native;
        println!("usdt: {usdt:?}");
        let eth = DedustAsset::Jetton(MsgAddress {
            workchain_id: 0,
            address: [1; 32],
        });
        println!("eth: {eth:?}");
        let btc = DedustAsset::Jetton(MsgAddress {
            workchain_id: 0,
            address: [2; 32],
        });
        println!("btc: {btc:?}");

        let pools = [
            DedustPool {
                address: MsgAddress::NULL,
                r#type: DedustPoolType::Volatile,
                assets: [usdt, eth],
                trade_fee: TRADE_FEE,
                reserves: [1_000, 2_500_000].map(u64::into),
            },
            DedustPool {
                address: MsgAddress::NULL,
                r#type: DedustPoolType::Volatile,
                assets: [usdt, btc],
                trade_fee: TRADE_FEE,
                reserves: [1_000, 50_000_000].map(u64::into),
            },
            DedustPool {
                address: MsgAddress::NULL,
                r#type: DedustPoolType::Volatile,
                assets: [eth, btc],
                trade_fee: TRADE_FEE,
                reserves: [1_000, 1_000].map(u64::into),
            },
        ];

        let g = DEX::from_iter(pools);
        println!("{:?}", Dot::new(&g.g));
        let mut cycle = g.find_all_paths(usdt, 4);
        cycle.reverse();
        // assert_eq!(cycle, [[usdt, eth, btc]]);
    }
}

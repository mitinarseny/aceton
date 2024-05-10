use std::fmt::Debug;

use tracing::info;

use crate::{ArbitragerConfig, Asset, DexGraph, DexPool, SwapPath};

pub struct Arbitrager<DP> {
    cfg: ArbitragerConfig,
    g: DexGraph<DP>,
}

impl<DP> Arbitrager<DP>
where
    DP: DexPool + Clone,
{
    pub fn new(cfg: ArbitragerConfig) -> Self {
        Self {
            cfg,
            g: Default::default(),
        }
    }

    pub fn add_pools(&mut self, pools: impl IntoIterator<Item = DP>) {
        self.g.extend(pools)
    }

    fn iter_profitable_cycles(&self) -> impl Iterator<Item = SwapPath<&DP>> + '_ {
        self.g
            .profitable_cycles(self.cfg.base_asset, self.cfg.max_length)
    }

    pub fn run(&mut self) -> anyhow::Result<()>
    where
        DP: Debug,
    {
        for cycle in self.iter_profitable_cycles() {
            info!("cycle: {:}", cycle);
        }
        Ok(())
    }

    pub fn find_and_prepare_msg(&self) {}
}

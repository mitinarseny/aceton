use std::{collections::HashMap, fs};

mod dex;
mod asset;

use aceton_core::TonContract;
use aceton_dedust::{DedustAsset, DedustPool, DedustPoolI};
use anyhow::anyhow;
use dex::DEX;
use petgraph::{
    dot::{Config, Dot},
    Graph, Undirected,
};
use tlb_ton::MsgAddress;
use tonlibjson_client::{
    block::{BoxedAccountState, MsgMessage},
    ton::TonClient,
};
use tonlibjson_sys::{emulate_run_method, TransactionEmulator, TvmEmulator};
use tracing::{error, info, warn};

pub struct Arbitrager {
    g: DEX,
    // ton: TonClient,
}

impl Arbitrager {
    pub fn new(
        // ton: TonClient,
        pools: impl IntoIterator<Item = DedustPool>,
    ) -> Self {
        Self {
            // ton,
            g: pools.into_iter().collect(),
        }
    }

    pub fn run(&self) -> anyhow::Result<()> {
        fs::write("/tmp/pools.graph", format!("{:?}", Dot::new(&self.g.g))).unwrap();
        info!(
            "pools: {}, assets: {}",
            self.g.edge_count(),
            self.g.node_count()
        );

        let paths = self.g.find_paths(DedustAsset::Native, 5);
        if paths.is_empty() {
            error!("no abritrage opportunity");
            return Ok(());
        }

        for (path, profit) in paths {
            info!("found path (profit {}): {:?}", profit, path);
        }

        Ok(())
    }
}

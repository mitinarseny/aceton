use aceton_core::TonContract;
use aceton_dedust::{api::DedustPool, DedustAsset, DedustPoolI};
use anyhow::anyhow;
use petgraph::Graph;
use tonlibjson_client::{
    block::{BoxedAccountState, MsgMessage},
    ton::TonClient,
};
use tonlibjson_sys::{emulate_run_method, TransactionEmulator, TvmEmulator};
use tracing::info;

pub struct Arbitrager {
    g: Graph<DedustAsset, TonContract>,
    ton: TonClient,
}

impl Arbitrager {
    pub fn new(ton: TonClient, pools: impl IntoIterator<Item = DedustPool>) -> Self {
        Self { ton, g: todo!() }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let dedust_pool = TonContract::new(
            self.ton.clone(),
            "EQBGXZ9ddZeWypx8EkJieHJX75ct0bpkmu0Y4YoYr3NM0Z9e".parse()?,
        );
        loop {
            let reserves = dedust_pool.get_reserves().await?;
            info!("assets: {reserves:?}");
        }
        Ok(())
    }
}

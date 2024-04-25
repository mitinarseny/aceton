use aceton_core::TonContract;
use aceton_dedust::DedustPool;
use tonlibjson_client::ton::TonClient;
use tracing::info;

pub struct Arbitrager {
    ton: TonClient,
}

impl Arbitrager {
    pub fn new(ton: TonClient) -> Self {
        Self { ton }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let dedust_pool = TonContract::new(
            self.ton.clone(),
            "EQBGXZ9ddZeWypx8EkJieHJX75ct0bpkmu0Y4YoYr3NM0Z9e".parse()?,
        );
        let assets = dedust_pool.get_assets().await?;
        info!("assets: {assets:?}");
        Ok(())
    }
}

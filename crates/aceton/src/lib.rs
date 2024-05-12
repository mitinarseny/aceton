pub mod config;

use std::time::Duration;

use tonlibjson_client::ton::TonClientBuilder;
use tracing::info;

use aceton_arbitrage::Arbitrager;
use aceton_dedust::DeDust;

use self::config::AcetonConfig;

pub struct Aceton {
    arbitrager: Arbitrager<DeDust>,
}

impl Aceton {
    pub async fn new(cfg: AcetonConfig) -> anyhow::Result<Self> {
        let http_client = reqwest::Client::new();

        info!("creating TON client...");
        let mut ton_client =
            TonClientBuilder::from_config_url(cfg.ton.config, Duration::from_secs(60))
                .set_timeout(Duration::from_secs(20))
                .build()
                .await?;
        info!("TON client created, waiting for ready...");
        ton_client.ready().await?;
        info!("TOM client ready");

        let arbitrager =
            Arbitrager::new(cfg.arbitrage, DeDust::new(ton_client, http_client)).await?;

        Ok(Self { arbitrager })
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        info!("running...");
        self.arbitrager.run().await
    }
}

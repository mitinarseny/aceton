pub mod config;

use std::time::Duration;

use anyhow::Context;
use ton_contracts::{mnemonic::Keypair, Wallet};
use tonlibjson_client::ton::TonClientBuilder;
use tracing::info;

use aceton_arbitrage::Arbitrager;
use aceton_dedust::{DeDust, DEDUST_FACTORY_MAINNET_ADDRESS};

use self::config::AcetonConfig;

pub struct Aceton {
    arbitrager: Arbitrager<DeDust>,
}

impl Aceton {
    pub async fn new(cfg: AcetonConfig, key_pair: Keypair) -> anyhow::Result<Self> {
        let wallet = Wallet::derive_default(key_pair).context("wallet")?;

        let http_client = reqwest::Client::new();

        info!("creating TON client...");
        let mut ton_client = cfg.ton.config()?.build().await?;
        info!("TON client created, waiting for ready...");
        ton_client.ready().await?;
        info!("TOM client ready");

        let arbitrager = Arbitrager::new(
            cfg.arbitrage,
            DeDust::new(ton_client, DEDUST_FACTORY_MAINNET_ADDRESS, http_client),
            wallet,
        )
        .await?;

        Ok(Self { arbitrager })
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        info!("running...");
        self.arbitrager.run().await
    }
}

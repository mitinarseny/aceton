pub mod config;

use std::{sync::Arc, time::Duration};

use aceton_arbitrage::Arbitrager;
use aceton_dedust::{api::DedustHTTPClient, DeDust, DedustPool, DedustPoolType};
use anyhow::Context;
use tonlibjson_client::ton::{TonClient, TonClientBuilder};
use tracing::info;

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
        self.arbitrager.run().await
        // let http_client = reqwest::Client::new();
        // let arbitrager = Arbitrager::new(self.cfg.arbitrage, DeDust::new(http_client)).await?;
        // arbitrager.run()

        // let txs = ton
        //     .raw_get_transactions(
        //         "EQCEho8oSvzVneM-q3ALV9GMOoRzlGNwrGtq4p2x3SnInMVA",
        //         &InternalTransactionId {
        //             lt: 43952321000007,
        //             hash: "000822ccb275702a65150712c354f432dba919a0230c80fb43454afa1ab588e3"
        //                 .to_string(),
        //         },
        //     )
        //     .await;
        // info!("{:?}", txs);

        // let mut txs =
        //     pin!(ton.get_account_tx_stream("EQCk6tGPlFoQ_1TgZJjuiulfSJz5aoJgnyy29eLsXtOmeYDw",));
        // info!("subscribed");
        // while let Some(tx) = txs.try_next().await? {
        //     info!(
        //         "tx {}, lt: {}",
        //         tx.transaction_id.hash, tx.transaction_id.lt
        //     );
        // }

        // let state = ton
        //     .get_account_state("EQBGXZ9ddZeWypx8EkJieHJX75ct0bpkmu0Y4YoYr3NM0Z9e")
        //     .await;
        // println!("state: {:?}", state);
        // let reserves = ton
        //     .run_get_method(
        //         "EQBGXZ9ddZeWypx8EkJieHJX75ct0bpkmu0Y4YoYr3NM0Z9e".to_string(),
        //         "get_assets".to_string(),
        //         [].into(),
        //     )
        //     .await?;
        // info!("{:?}", reserves);
    }
}

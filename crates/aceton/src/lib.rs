pub mod config;

use std::sync::Arc;

use aceton_arbitrage::Arbitrager;
use aceton_dedust::{api::DedustHTTPClient, DedustPool};
use anyhow::Context;
use tracing::info;

use self::config::AcetonConfig;

pub struct Aceton {
    arbitrager: Arbitrager<Arc<DedustPool>>,
}

impl Aceton {
    pub async fn new(cfg: AcetonConfig) -> anyhow::Result<Self> {
        let mut arbitrager = Arbitrager::new(cfg.arbitrage);

        info!("getting DeDust pools...");
        let dedust_pools = DedustHTTPClient::default()
            .get_available_pools()
            .await
            .context("DeDust HTTP API")?;
        info!("DeDust pools: {}", dedust_pools.len());
        arbitrager.add_pools(dedust_pools.into_iter().map(Arc::new));

        // let mut ton = args.ton_config()?.build().await?;
        // info!("initializing TON client...");
        // ton.ready().await?;
        // info!("TON client is ready");

        Ok(Self { arbitrager })
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        self.arbitrager.run()
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

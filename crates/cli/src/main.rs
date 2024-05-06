mod args;
mod metrics;

use core::pin::pin;

use aceton_arbitrage::Arbitrager;
use aceton_dedust::api::DedustHTTPClient;
use anyhow::Context;
use args::CliArgs;
use clap::Parser;
use futures::stream::TryStreamExt;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use reqwest::Client;
use tonlibjson_client::{block::InternalTransactionId, ton::TonClient};
use tracing::info;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run(CliArgs::parse()).await
}

async fn run(args: CliArgs) -> anyhow::Result<()> {
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::default());
    args.logging.make_subscriber()?.try_init()?;

    // let mut ton = args.ton_config()?.build().await?;
    // info!("initializing TON client...");
    // ton.ready().await?;
    // info!("TON client is ready");

    let dedust_client = DedustHTTPClient::new(Client::new());

    loop {
        let pools = dedust_client
            .get_available_pools()
            .await
            .context("DeDust HTTP API")?;
        info!("pools count: {}", pools.len());

        let arb = Arbitrager::new(pools);
        info!("initialized graph");

        arb.run()?;
    }

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
    Ok(())
}

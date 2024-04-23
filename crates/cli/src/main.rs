mod args;
mod metrics;

use core::pin::pin;

use args::CliArgs;
use clap::Parser;
use futures::stream::TryStreamExt;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing::info;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run(CliArgs::parse()).await
}

async fn run(args: CliArgs) -> anyhow::Result<()> {
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::default());
    args.logging.make_subscriber()?.try_init()?;

    let mut ton = args.ton_config()?.build().await?;
    info!("client init");
    ton.ready().await?;
    info!("client ready");

    let mut txs =
        pin!(ton.get_account_tx_stream("EQCk6tGPlFoQ_1TgZJjuiulfSJz5aoJgnyy29eLsXtOmeYDw",));
    info!("subscribed");
    while let Some(tx) = txs.try_next().await? {
        info!(
            "tx {}, lt: {}",
            tx.transaction_id.hash, tx.transaction_id.lt
        );
    }

    // let state = ton
    //     .get_account_state("EQBGXZ9ddZeWypx8EkJieHJX75ct0bpkmu0Y4YoYr3NM0Z9e")
    //     .await;
    // println!("state: {:?}", state);
    // let reserves = ton
    //     .run_get_method(
    //         "EQBGXZ9ddZeWypx8EkJieHJX75ct0bpkmu0Y4YoYr3NM0Z9e".to_string(),
    //         "get_reserves".to_string(),
    //         [].into(),
    //     )
    //     .await?;
    // info!("{:?}", reserves);
    Ok(())
}

mod args;
mod metrics;

use anyhow::Context;
use args::CliArgs;
use clap::Parser;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing_subscriber::util::SubscriberInitExt;

use aceton::Aceton;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run(CliArgs::parse()).await
}

async fn run(args: CliArgs) -> anyhow::Result<()> {
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::default());
    args.logging.make_subscriber()?.try_init()?;

    let cfg = args.config().await.context("config")?;
    let key_pair = args.key_pair().await.context("secret")?;

    let app = Aceton::new(cfg, key_pair).await?;

    app.run().await?;

    Ok(())
}

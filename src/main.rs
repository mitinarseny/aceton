mod config;

use std::{
    io::{self, IsTerminal},
    path::PathBuf,
};

use anyhow::Context;
use clap::{Args, Parser, ValueHint};
use futures::{pin_mut, select_biased, FutureExt};
use lazy_static::lazy_static;
use opentelemetry::KeyValue;
use opentelemetry_otlp::{TonicExporterBuilder, WithExportConfig};
use opentelemetry_sdk::{propagation::TraceContextPropagator, Resource};
use tokio::{fs, signal};
use tonlib::{
    mnemonic::Mnemonic,
    wallet::{TonWallet, WalletVersion},
};
use tracing::{info, level_filters::LevelFilter, Level, Subscriber};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{
    filter::{FilterExt, Targets},
    prelude::*,
    registry::LookupSpan,
    Layer, Registry,
};

use aceton::{self, utils::metrics::MetricsFilter, App};

use crate::config::AcetonConfig;

#[derive(Parser)]
struct CliArgs {
    #[arg(
        short, long,
        value_parser,
        value_hint = ValueHint::FilePath,
        value_name = "FILE",
        default_value_os_t = PathBuf::from("./aceton.toml"),
    )]
    config: PathBuf,

    #[arg(
        short, long,
        value_parser,
        value_hint = ValueHint::FilePath,
        value_name = "FILE",
        default_value_os_t = PathBuf::from("./mnemonic.txt"),
    )]
    mnemonic: PathBuf,

    #[command(flatten)]
    logging: LoggingArgs,
}

#[derive(Args)]
struct LoggingArgs {
    #[arg(long, value_name = "HOST:PORT")]
    /// Endpoint for OTLP traces
    otlp_endpoint: Option<String>,

    #[arg(
        short, long,
        action = clap::ArgAction::Count,
    )]
    /// Increase verbosity (error (deafult) -> warn -> info -> debug -> trace)
    verbose: u8,

    #[arg(long)]
    /// Use JSON logs format even on tty
    json: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let r = run(CliArgs::parse()).fuse();
    let shutdown = shutdown_signal().fuse();

    pin_mut!(r);
    pin_mut!(shutdown);

    select_biased! {
        _ = shutdown => {},
        r = r => {
            r?;
        },
    }

    Ok(())
}

async fn run(args: CliArgs) -> anyhow::Result<()> {
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::default());
    args.logging.make_subscriber()?.try_init()?;

    let config = args.read_config().await.context("config")?;
    let wallet = args.get_wallet().await.context("wallet")?;

    info!("creating TON client...");
    let client = config
        .make_ton_client()
        .await
        .context("unable to make TonClient")?;

    let app = App::new(client, config.config, wallet)
        .await
        .context("failed to initialize app")?;
    info!("app initialized, running...");

    app.run().await
}

lazy_static! {
    static ref ACETON_RESOURCE: Resource = Resource::new([KeyValue::new(
        opentelemetry_semantic_conventions::resource::SERVICE_NAME,
        env!("CARGO_PKG_NAME"),
    )]);
}

impl CliArgs {
    async fn read_config(&self) -> anyhow::Result<AcetonConfig> {
        let s = fs::read_to_string(&self.config)
            .await
            .with_context(|| format!("failed to read file '{}'", self.config.display()))?;

        toml::from_str(s.as_ref()).with_context(|| {
            format!(
                "failed to parse TOML config file '{}'",
                self.config.display()
            )
        })
    }

    async fn read_mnemonic(&self) -> anyhow::Result<Mnemonic> {
        let s = fs::read_to_string(&self.mnemonic)
            .await
            .with_context(|| format!("failed to read file '{}'", self.mnemonic.display()))?;

        Mnemonic::new(s.split_ascii_whitespace().collect(), &None).map_err(Into::into)
    }

    async fn get_wallet(&self) -> anyhow::Result<TonWallet> {
        let mnemonic = self.read_mnemonic().await?;
        let key_pair = mnemonic.to_key_pair()?;
        TonWallet::derive(0, WalletVersion::V4R2, &key_pair).map_err(Into::into)
    }
}

impl LoggingArgs {
    fn make_subscriber(&self) -> anyhow::Result<impl Subscriber> {
        Ok(Registry::default().with(self.make_layer()?))
    }

    fn verbosity_level(&self) -> Level {
        const LEVELS: [Level; 5] = [
            Level::ERROR,
            Level::WARN,
            Level::INFO,
            Level::DEBUG,
            Level::TRACE,
        ];

        LEVELS[(self.verbose as usize).min(LEVELS.len() - 1)]
    }

    fn make_layer<S>(&self) -> anyhow::Result<impl Layer<S>>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        Ok(self
            .make_otlp_layer()?
            .and_then(
                self.make_fmt_layer()
                    // filter out metrics events
                    .with_filter(MetricsFilter.not()),
            )
            .with_filter(
                Targets::default()
                    .with_targets([
                        (env!("CARGO_PKG_NAME"), Level::TRACE),
                        ("otel::tracing", Level::TRACE),
                    ])
                    .with_default(Level::WARN),
            ))
    }

    fn make_otlp_layer<S>(&self) -> anyhow::Result<impl Layer<S>>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        Ok(if let Some(endpoint) = self.otlp_endpoint.clone() {
            Some(
                Self::make_metrics_layer(endpoint.clone())?.and_then(
                    Self::make_tracing_level(endpoint)?
                        // filter out metrics events
                        .with_filter(MetricsFilter.not()),
                ),
            )
        } else {
            None
        })
    }

    fn make_metrics_layer<S>(endpoint: impl Into<String>) -> anyhow::Result<impl Layer<S>>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        Ok(MetricsLayer::new(
            opentelemetry_otlp::new_pipeline()
                .metrics(opentelemetry_sdk::runtime::Tokio)
                .with_exporter(Self::make_otlp_exporter(endpoint))
                .with_resource(ACETON_RESOURCE.clone())
                .build()?,
        ))
    }

    fn make_tracing_level<S>(endpoint: impl Into<String>) -> anyhow::Result<impl Layer<S>>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        Ok(OpenTelemetryLayer::new(
            opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(Self::make_otlp_exporter(endpoint))
                .with_trace_config(
                    opentelemetry_sdk::trace::config().with_resource(ACETON_RESOURCE.clone()),
                )
                .install_batch(opentelemetry_sdk::runtime::Tokio)?,
        ))
    }

    fn make_otlp_exporter(endpoint: impl Into<String>) -> TonicExporterBuilder {
        opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(endpoint)
    }

    fn make_fmt_layer<S>(&self) -> impl Layer<S>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        if self.json || !io::stdout().is_terminal() {
            tracing_subscriber::fmt::layer()
                .json()
                .map_event_format(|f| f.with_source_location(true))
                .boxed()
        } else {
            tracing_subscriber::fmt::layer()
                .pretty()
                .map_event_format(|f| f.with_source_location(true))
                .boxed()
        }
        .with_filter(LevelFilter::from_level(self.verbosity_level()))
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("signal received, starting shutdown...");
}

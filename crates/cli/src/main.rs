mod metrics;

use core::time::Duration;
use std::io::{self, IsTerminal};

use clap::{Args, Parser};
use lazy_static::lazy_static;
use metrics::MetricsFilter;
use opentelemetry::KeyValue;
use opentelemetry_otlp::{TonicExporterBuilder, WithExportConfig};
use opentelemetry_sdk::{propagation::TraceContextPropagator, Resource};
use tonlibjson_client::ton::TonClientBuilder;
use tracing::{info, level_filters::LevelFilter, Level, Subscriber};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{
    filter::{FilterExt, Targets},
    prelude::*,
    registry::LookupSpan,
    Layer, Registry,
};
use url::Url;

#[derive(Parser)]
struct CliArgs {
    // #[arg(
    //     short, long,
    //     value_parser,
    //     value_hint = ValueHint::FilePath,
    //     value_name = "FILE",
    //     default_value_os_t = PathBuf::from("./aceton.toml"),
    // )]
    // config: PathBuf,

    // #[arg(
    //     short, long,
    //     value_parser,
    //     value_hint = ValueHint::FilePath,
    //     value_name = "FILE",
    //     default_value_os_t = PathBuf::from("./mnemonic.txt"),
    // )]
    // mnemonic: PathBuf,
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
    run(CliArgs::parse()).await
}

async fn run(args: CliArgs) -> anyhow::Result<()> {
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::default());
    args.logging.make_subscriber()?.try_init()?;

    let mut ton = TonClientBuilder::from_config_url(
        Url::parse("https://ton.org/global-config.json").unwrap(),
        Duration::from_secs(60),
    )
    .build()
    .await?;
    info!("client init");
    ton.ready().await?;
    info!("client ready");

    let state = ton
        .get_account_state("EQBGXZ9ddZeWypx8EkJieHJX75ct0bpkmu0Y4YoYr3NM0Z9e")
        .await;
    println!("state: {:?}", state);
    let reserves = ton
        .run_get_method(
            "EQBGXZ9ddZeWypx8EkJieHJX75ct0bpkmu0Y4YoYr3NM0Z9e".to_string(),
            "get_reserves".to_string(),
            [].into(),
        )
        .await?;
    info!("{:?}", reserves);
    // ton_client::request()
    Ok(())
}

lazy_static! {
    static ref ACETON_RESOURCE: Resource = Resource::new([KeyValue::new(
        opentelemetry_semantic_conventions::resource::SERVICE_NAME,
        env!("CARGO_PKG_NAME"),
    )]);
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
                    .with_default(Level::INFO),
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

use std::{
    io::{self, IsTerminal},
    path::PathBuf,
};

use anyhow::Context;
use clap::{Args, Parser, ValueHint};
use lazy_static::lazy_static;
use opentelemetry::KeyValue;
use opentelemetry_otlp::{TonicExporterBuilder, WithExportConfig};
use opentelemetry_sdk::Resource;
use tokio::fs;
use ton_contracts::mnemonic::{Keypair, Mnemonic};
use tracing::{info, level_filters::LevelFilter, Level, Subscriber};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{
    filter::{FilterExt, Targets},
    layer::SubscriberExt,
    registry::LookupSpan,
    Layer, Registry,
};

use aceton::config::AcetonConfig;

use crate::metrics::MetricsFilter;

#[derive(Parser)]
pub struct CliArgs {
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
    secret: PathBuf,
    // #[arg(
    //     short, long,
    //     value_parser,
    //     value_hint = ValueHint::FilePath,
    //     value_name = "FILE",
    //     default_value_os_t = PathBuf::from("./mnemonic.txt"),
    // )]
    // mnemonic: PathBuf,
    #[command(flatten)]
    pub logging: LoggingArgs,
}

impl CliArgs {
    pub async fn config(&self) -> anyhow::Result<AcetonConfig> {
        info!(config = %self.config.display(), "reading config");
        let contents = fs::read_to_string(&self.config).await.context("read")?;
        toml::from_str(&contents).context("TOML")
    }

    pub async fn key_pair(&self) -> anyhow::Result<Keypair> {
        let contents = fs::read_to_string(&self.secret).await.context("read")?;
        contents.parse::<Mnemonic>()?.generate_keypair(None)
    }
}

#[derive(Args)]
pub struct LoggingArgs {
    #[arg(long, value_name = "URL")]
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

lazy_static! {
    static ref ACETON_RESOURCE: Resource = Resource::new([KeyValue::new(
        opentelemetry_semantic_conventions::resource::SERVICE_NAME,
        "aceton",
    )]);
}

impl LoggingArgs {
    pub fn make_subscriber(&self) -> anyhow::Result<impl Subscriber> {
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
                    .with_targets([("aceton", Level::TRACE), ("otel::tracing", Level::TRACE)])
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

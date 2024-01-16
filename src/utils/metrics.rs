use tracing::{subscriber::Interest, Metadata};
use tracing_subscriber::layer::{Context, Filter};

const METRIC_PREFIX_MONOTONIC_COUNTER: &str = "monotonic_counter.";
const METRIC_PREFIX_COUNTER: &str = "counter.";
const METRIC_PREFIX_HISTOGRAM: &str = "histogram.";

/// Copied from [`tracing-opentelemetry::metrics`](https://github.com/tokio-rs/tracing-opentelemetry/blob/a03ff2275bbb86add80f20c8c7b6126bd1a2b38f/src/metrics.rs#L369-L377)
pub struct MetricsFilter;

impl MetricsFilter {
    fn is_metrics_event(&self, meta: &Metadata<'_>) -> bool {
        meta.is_event()
            && meta.fields().iter().any(|field| {
                let name = field.name();
                name.starts_with(METRIC_PREFIX_COUNTER)
                    || name.starts_with(METRIC_PREFIX_MONOTONIC_COUNTER)
                    || name.starts_with(METRIC_PREFIX_HISTOGRAM)
            })
    }
}

impl<S> Filter<S> for MetricsFilter {
    fn enabled(&self, meta: &Metadata<'_>, _cx: &Context<'_, S>) -> bool {
        self.is_metrics_event(meta)
    }

    fn callsite_enabled(&self, meta: &'static Metadata<'static>) -> Interest {
        if self.is_metrics_event(meta) {
            Interest::always()
        } else {
            Interest::never()
        }
    }
}

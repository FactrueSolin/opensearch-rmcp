use anyhow::{Context, Result};
use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::{
    Resource,
    metrics::SdkMeterProvider,
    trace::{Sampler, SdkTracerProvider},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct ObservabilityGuard {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
}

impl ObservabilityGuard {
    pub fn shutdown(self) {
        if let Err(err) = self.meter_provider.shutdown() {
            tracing::warn!(error = %err, "failed to shutdown OpenTelemetry meter provider");
        }
        if let Err(err) = self.tracer_provider.shutdown() {
            tracing::warn!(error = %err, "failed to shutdown OpenTelemetry tracer provider");
        }
    }
}

pub fn init() -> Result<ObservabilityGuard> {
    let resource = Resource::builder().build();

    let span_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .build()
        .context("build OTLP span exporter")?;

    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_sampler(Sampler::AlwaysOn)
        .with_batch_exporter(span_exporter)
        .build();

    let tracer = tracer_provider.tracer(env!("CARGO_PKG_NAME"));
    global::set_tracer_provider(tracer_provider.clone());

    let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .build()
        .context("build OTLP metric exporter")?;

    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_periodic_exporter(metric_exporter)
        .build();
    global::set_meter_provider(meter_provider.clone());

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,openperplexity=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();

    Ok(ObservabilityGuard {
        tracer_provider,
        meter_provider,
    })
}

use std::sync::OnceLock;

use opentelemetry::{
    KeyValue, global,
    metrics::{Counter, Histogram, UpDownCounter},
};
use sha2::{Digest, Sha256};
use tracing_opentelemetry::OpenTelemetrySpanExt;

static METRICS: OnceLock<AppMetrics> = OnceLock::new();

pub struct AppMetrics {
    pub mcp_tool_calls: Counter<u64>,
    pub mcp_tool_duration_ms: Histogram<f64>,
    pub mcp_tool_in_flight: UpDownCounter<i64>,
    pub searxng_requests: Counter<u64>,
    pub searxng_request_duration_ms: Histogram<f64>,
    pub searxng_result_count: Histogram<u64>,
    pub rerank_requests: Counter<u64>,
    pub rerank_duration_ms: Histogram<f64>,
    pub rerank_document_count: Histogram<u64>,
    pub auth_failures: Counter<u64>,
}

pub fn metrics() -> &'static AppMetrics {
    METRICS.get_or_init(|| {
        let meter = global::meter(env!("CARGO_PKG_NAME"));
        AppMetrics {
            mcp_tool_calls: meter
                .u64_counter("mcp_tool_calls_total")
                .with_description("Total MCP tool calls")
                .build(),
            mcp_tool_duration_ms: meter
                .f64_histogram("mcp_tool_duration_ms")
                .with_description("MCP tool call duration")
                .with_unit("ms")
                .build(),
            mcp_tool_in_flight: meter
                .i64_up_down_counter("mcp_tool_in_flight")
                .with_description("In-flight MCP tool calls")
                .build(),
            searxng_requests: meter
                .u64_counter("searxng_requests_total")
                .with_description("Total SearXNG requests")
                .build(),
            searxng_request_duration_ms: meter
                .f64_histogram("searxng_request_duration_ms")
                .with_description("SearXNG request duration")
                .with_unit("ms")
                .build(),
            searxng_result_count: meter
                .u64_histogram("searxng_result_count")
                .with_description("Mapped SearXNG result count")
                .build(),
            rerank_requests: meter
                .u64_counter("rerank_requests_total")
                .with_description("Total rerank requests")
                .build(),
            rerank_duration_ms: meter
                .f64_histogram("rerank_duration_ms")
                .with_description("Rerank request duration")
                .with_unit("ms")
                .build(),
            rerank_document_count: meter
                .u64_histogram("rerank_document_count")
                .with_description("Documents sent to rerank")
                .build(),
            auth_failures: meter
                .u64_counter("mcp_auth_failed_total")
                .with_description("MCP authentication failures")
                .build(),
        }
    })
}

pub fn query_hash(query: &str) -> String {
    let digest = Sha256::digest(query.as_bytes());
    hex::encode(&digest[..8])
}

pub fn key(name: &'static str, value: impl Into<opentelemetry::Value>) -> KeyValue {
    KeyValue::new(name, value)
}

pub fn set_current_span_status(success: bool, description: impl Into<String>) {
    if success {
        tracing::Span::current().set_status(opentelemetry::trace::Status::Ok);
    } else {
        tracing::Span::current()
            .set_status(opentelemetry::trace::Status::error(description.into()));
    }
}

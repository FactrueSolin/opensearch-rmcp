use std::sync::Arc;

use anyhow::Result;
use axum::{Router, body::Body, http::Request, middleware, response::Response, routing::get};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tokio_util::sync::CancellationToken;
use tower_http::trace::TraceLayer;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use super::{
    auth::{AuthState, auth_middleware},
    config::McpConfig,
    tools::SearxngTools,
};
use crate::searxng::client::SearxngClient;

async fn health_check() -> &'static str {
    "OK"
}

pub async fn serve(config: McpConfig) -> Result<()> {
    let auth_state = Arc::new(AuthState::new(config.auth_token));
    let client = SearxngClient::new(config.searxng_url);
    let ct = CancellationToken::new();

    let mcp_service: StreamableHttpService<SearxngTools, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(SearxngTools::new(client.clone())),
            LocalSessionManager::default().into(),
            StreamableHttpServerConfig {
                cancellation_token: ct.child_token(),
                ..Default::default()
            },
        );

    let mcp_router = if auth_state.enabled() {
        Router::new()
            .nest_service("/mcp", mcp_service)
            .layer(middleware::from_fn_with_state(
                auth_state.clone(),
                auth_middleware,
            ))
    } else {
        Router::new().nest_service("/mcp", mcp_service)
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .merge(mcp_router)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<Body>| {
                    tracing::info_span!(
                        "mcp.http",
                        otel.kind = "server",
                        "http.request.method" = %request.method(),
                        "url.path" = %request.uri().path(),
                        "http.response.status_code" = tracing::field::Empty,
                    )
                })
                .on_response(|response: &Response<Body>, _latency, span: &Span| {
                    let status = response.status();
                    span.record("http.response.status_code", status.as_u16() as i64);
                    if status.is_server_error() {
                        span.set_status(opentelemetry::trace::Status::error(status.to_string()));
                    } else {
                        span.set_status(opentelemetry::trace::Status::Ok);
                    }
                }),
        );

    let listener = tokio::net::TcpListener::bind(&config.bind).await?;
    tracing::info!("MCP server listening on {}", config.bind);

    let _ = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            ct.cancel();
        })
        .await;
    Ok(())
}

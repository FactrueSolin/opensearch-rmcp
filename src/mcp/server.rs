use std::sync::Arc;

use anyhow::Result;
use axum::{Router, middleware, routing::get};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tokio_util::sync::CancellationToken;

use super::{auth::{AuthState, auth_middleware}, config::McpConfig, tools::SearxngTools};
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
            .layer(middleware::from_fn_with_state(auth_state.clone(), auth_middleware))
    } else {
        Router::new().nest_service("/mcp", mcp_service)
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .merge(mcp_router);

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

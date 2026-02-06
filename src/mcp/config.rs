use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct McpConfig {
    pub bind: String,
    pub searxng_url: String,
    pub auth_token: Option<String>,
}

impl McpConfig {
    pub fn from_env() -> Result<Self> {
        let bind = std::env::var("MCP_BIND").unwrap_or_else(|_| "127.0.0.1:8000".to_string());
        let searxng_url = std::env::var("SEARXNG_URL").context("SEARXNG_URL is required")?;
        let searxng_url = searxng_url.trim().trim_end_matches('/').to_string();
        if searxng_url.is_empty() {
            anyhow::bail!("SEARXNG_URL is required")
        }
        let auth_token = std::env::var("MCP_AUTH_TOKEN")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        Ok(Self {
            bind,
            searxng_url,
            auth_token,
        })
    }
}

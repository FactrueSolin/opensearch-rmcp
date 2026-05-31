use anyhow::Result;

use openperplexity::{
    mcp::{config::McpConfig, server},
    observability,
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let observability = observability::init()?;

    let config = McpConfig::from_env()?;
    let result = server::serve(config).await;
    observability.shutdown();
    result
}

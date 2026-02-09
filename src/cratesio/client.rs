use anyhow::{Context, Result};
use reqwest::{Client, header::USER_AGENT};
use serde::{Deserialize, Serialize};

const CRATES_IO_API: &str = "https://crates.io/api/v1/crates";
const CRATES_IO_USER_AGENT: &str = "mcp-cratesio-probe/0.1 (contact: you@example.com)";

#[derive(Debug, Deserialize)]
struct CratesIoSearchResponse {
    #[serde(default)]
    crates: Vec<CratesIoCrate>,
}

#[derive(Debug, Deserialize)]
struct CratesIoCrate {
    name: String,
    max_stable_version: String,
    description: Option<String>,
    downloads: u64,
    repository: Option<String>,
    documentation: Option<String>,
    homepage: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CrateInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub downloads: u64,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
}

#[derive(Clone)]
pub struct CratesIoClient {
    http: Client,
}

impl CratesIoClient {
    pub fn new() -> Self {
        Self {
            http: Client::new(),
        }
    }

    pub async fn search_simplified_json_string(&self, query: &str, limit: usize) -> Result<String> {
        let payload = self
            .http
            .get(CRATES_IO_API)
            .header(USER_AGENT, CRATES_IO_USER_AGENT)
            .query(&[
                ("q", query),
                ("per_page", &limit.to_string()),
                ("sort", "downloads"),
            ])
            .send()
            .await
            .context("request crates.io failed")?
            .error_for_status()
            .context("crates.io returned error status")?
            .json::<CratesIoSearchResponse>()
            .await
            .context("decode crates.io response failed")?;

        let crates = payload
            .crates
            .into_iter()
            .map(|item| CrateInfo {
                name: item.name,
                version: item.max_stable_version,
                description: item.description,
                downloads: item.downloads,
                repository: item.repository,
                documentation: item.documentation,
                homepage: item.homepage,
            })
            .collect::<Vec<_>>();

        serde_json::to_string(&crates).context("serialize crates.io response failed")
    }
}

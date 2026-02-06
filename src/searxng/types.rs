use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    pub url: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchToolResponse {
    pub query: String,
    pub category: String,
    pub success: bool,
    pub results: Vec<SearchResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QuerySearchResult {
    pub query: String,
    pub success: bool,
    pub results: Vec<SearchResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OpenSearchResponse {
    pub success: bool,
    pub search_type: String,
    pub results: Vec<QuerySearchResult>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearxngResponse {
    #[serde(default)]
    pub results: Vec<SearxngResultItem>,
}

#[derive(Debug, Deserialize)]
pub struct SearxngResultItem {
    pub url: Option<String>,
    pub content: Option<String>,
    pub title: Option<String>,
    pub img_src: Option<String>,
}

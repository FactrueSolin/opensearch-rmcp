use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct RerankRequest {
    pub model: String,
    pub query: String,
    pub documents: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RerankResponse {
    pub id: String,
    pub results: Vec<RerankResult>,
    #[serde(default)]
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RerankResult {
    pub index: usize,
    pub relevance_score: f64,
}

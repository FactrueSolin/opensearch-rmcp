use anyhow::{Context, Result};
use reqwest::Client;
use tracing::{debug, info};

use super::types::{RerankRequest, RerankResponse, RerankResult};

const RERANK_API_ENDPOINT: &str = "https://api.siliconflow.cn/v1/rerank";
const DEFAULT_MODEL: &str = "Qwen/Qwen3-Reranker-8B";

#[derive(Clone)]
pub struct RerankClient {
    api_key: String,
    http: Client,
}

impl RerankClient {
    /// 创建新的 RerankClient 实例
    /// 从环境变量 SILICONFLOW_API_KEY 读取 API 密钥
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("SILICONFLOW_API_KEY")
            .context("SILICONFLOW_API_KEY environment variable not set")?;
        
        Ok(Self {
            api_key,
            http: Client::new(),
        })
    }

    /// 使用指定的 API 密钥创建 RerankClient
    pub fn with_api_key(api_key: String) -> Self {
        Self {
            api_key,
            http: Client::new(),
        }
    }

    /// 对文档进行重排序
    /// 
    /// # 参数
    /// - `query`: 查询文本
    /// - `documents`: 待排序的文档列表
    /// 
    /// # 返回
    /// 返回按相关性得分排序的结果列表（从高到低）
    pub async fn rerank(&self, query: &str, documents: Vec<String>) -> Result<Vec<RerankResult>> {
        if documents.is_empty() {
            debug!("Empty documents list, returning empty results");
            return Ok(Vec::new());
        }

        debug!(
            query = %query,
            documents_count = documents.len(),
            model = DEFAULT_MODEL,
            "Sending rerank request to SiliconFlow API"
        );

        let request = RerankRequest {
            model: DEFAULT_MODEL.to_string(),
            query: query.to_string(),
            documents,
        };

        let response = self
            .http
            .post(RERANK_API_ENDPOINT)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send rerank request")?
            .error_for_status()
            .context("Rerank API returned error status")?
            .json::<RerankResponse>()
            .await
            .context("Failed to decode rerank response")?;

        info!(
            id = %response.id,
            results_count = response.results.len(),
            "Received rerank response from SiliconFlow API"
        );

        debug!(
            response_id = %response.id,
            results = ?response.results,
            "Complete rerank response details"
        );

        // 按相关性得分从高到低排序
        let mut results = response.results;
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        debug!(
            sorted_results_count = results.len(),
            "Sorted rerank results by relevance score (descending)"
        );

        Ok(results)
    }
}

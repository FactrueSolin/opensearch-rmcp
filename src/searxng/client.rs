use anyhow::{Context, Result};
use reqwest::Client;
use tracing::debug;

use super::{
    mapper::map_result_item,
    types::{SearchToolResponse, SearxngResponse},
};
use crate::rerank::RerankClient;

#[derive(Clone)]
pub struct SearxngClient {
    base_url: String,
    http: Client,
    rerank_client: Option<RerankClient>,
}

impl SearxngClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            http: Client::new(),
            rerank_client: None,
        }
    }

    /// 创建带有重排序功能的搜索客户端
    pub fn new_with_rerank(base_url: String, rerank_client: RerankClient) -> Self {
        Self {
            base_url,
            http: Client::new(),
            rerank_client: Some(rerank_client),
        }
    }

    pub async fn search(
        &self,
        query: &str,
        category: Option<&str>,
        limit: usize,
    ) -> Result<SearchToolResponse> {
        let mut response = SearchToolResponse {
            query: query.to_string(),
            category: category.unwrap_or("general").to_string(),
            success: false,
            results: Vec::new(),
            error: None,
        };

        let endpoint = format!("{}/search", self.base_url.trim_end_matches('/'));
        let mut request = self
            .http
            .get(endpoint)
            .query(&[("q", query), ("format", "json")]);

        if let Some(category) = category {
            request = request.query(&[("categories", category)]);
        }

        let payload = request
            .send()
            .await
            .context("request searxng failed")?
            .error_for_status()
            .context("searxng returned error status")?
            .json::<SearxngResponse>()
            .await
            .context("decode searxng response failed")?;

        let category_key = category.unwrap_or("general");
        let mut results: Vec<_> = payload
            .results
            .into_iter()
            .filter_map(|item| map_result_item(category_key, item))
            .collect();

        // 如果配置了重排序客户端，则在结果数量超过 limit 时使用重排序
        if let Some(rerank_client) = &self.rerank_client {
            if results.len() <= limit {
                debug!(
                    results_count = results.len(),
                    limit, "Skipping rerank: results count is within limit"
                );
            } else {
                // 构造待排序文档：将 URL 与 description 组合，给 rerank 更多上下文
                // 形如："{url} - {description}"
                let documents: Vec<String> = results
                    .iter()
                    .map(|r| format!("{} - {}", r.url, r.description))
                    .collect();

                // 构造增强 query：注入搜索类型(category) + 用户原始搜索词
                // 模板：用户使用搜索引擎搜索，正在进行{search_type}的类型的搜索，搜索目标是"{user_query}"
                let rerank_query = format!(
                    "用户使用搜索引擎搜索，正在进行{}的类型的搜索，搜索目标是\"{}\"",
                    category_key, query
                );

                // 调用重排序 API
                match rerank_client.rerank(&rerank_query, documents).await {
                    Ok(rerank_results) => {
                        // 根据重排序结果重新排列搜索结果
                        let mut reordered_results = Vec::new();
                        for rerank_result in rerank_results {
                            if let Some(result) = results.get(rerank_result.index) {
                                reordered_results.push(result.clone());
                            }
                        }
                        results = reordered_results;
                    }
                    Err(e) => {
                        // 重排序失败时记录错误但不影响搜索结果返回
                        eprintln!("Rerank failed: {}, using original order", e);
                    }
                }
            }
        }

        // 按 limit 截取结果
        response.results = results.into_iter().take(limit).collect();
        response.success = true;
        Ok(response)
    }
}

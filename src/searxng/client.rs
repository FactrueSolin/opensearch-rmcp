use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Instant;
use tracing::{Instrument, debug, info_span, warn};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use super::{
    mapper::map_result_item,
    types::{SearchToolResponse, SearxngResponse},
};
use crate::{
    rerank::RerankClient,
    telemetry::{key, metrics, set_current_span_status},
};

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

    pub async fn search(&self, query: &str, category: Option<&str>) -> Result<SearchToolResponse> {
        let category_key = category.unwrap_or("general");
        let started_at = Instant::now();
        let endpoint = format!("{}/search", self.base_url.trim_end_matches('/'));
        let server_address = self
            .base_url
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(&self.base_url)
            .split('/')
            .next()
            .unwrap_or(&self.base_url)
            .to_string();
        let span = info_span!(
            "searxng.search",
            otel.kind = "client",
            "http.request.method" = "GET",
            "url.path" = "/search",
            "server.address" = %server_address,
            "search.category" = category_key,
            "http.response.status_code" = tracing::field::Empty,
            "searxng.result.raw_count" = tracing::field::Empty,
            "searxng.result.mapped_count" = tracing::field::Empty,
            "searxng.outcome" = tracing::field::Empty,
        );

        async move {
            let mut response = SearchToolResponse {
                query: query.to_string(),
                category: category_key.to_string(),
                success: false,
                results: Vec::new(),
                error: None,
            };

            let mut request = self
                .http
                .get(endpoint)
                .query(&[("q", query), ("format", "json")]);

            if let Some(category) = category {
                request = request.query(&[("categories", category)]);
            }

            let http_response = match request.send().await.context("request searxng failed") {
                Ok(response) => response,
                Err(err) => {
                    record_searxng_end("request_error", category_key, None, 0, started_at);
                    return Err(err);
                }
            };

            let status = http_response.status();
            tracing::Span::current().record("http.response.status_code", status.as_u16() as i64);

            let http_response = match http_response
                .error_for_status()
                .context("searxng returned error status")
            {
                Ok(response) => response,
                Err(err) => {
                    record_searxng_end(
                        "status_error",
                        category_key,
                        Some(status.as_u16()),
                        0,
                        started_at,
                    );
                    return Err(err);
                }
            };

            let payload = match http_response
                .json::<SearxngResponse>()
                .await
                .context("decode searxng response failed")
            {
                Ok(payload) => payload,
                Err(err) => {
                    record_searxng_end(
                        "decode_error",
                        category_key,
                        Some(status.as_u16()),
                        0,
                        started_at,
                    );
                    return Err(err);
                }
            };

            let raw_count = payload.results.len();
            let mut results: Vec<_> = payload
                .results
                .into_iter()
                .filter_map(|item| map_result_item(category_key, item))
                .collect();
            tracing::Span::current().record("searxng.result.raw_count", raw_count as i64);
            tracing::Span::current().record("searxng.result.mapped_count", results.len() as i64);

            // 如果配置了重排序客户端，则对结果进行重排序
            if let Some(rerank_client) = &self.rerank_client {
                if results.is_empty() {
                    debug!("Skipping rerank: no search results");
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
                            warn!(error = %e, "Rerank failed, using original order");
                        }
                    }
                }
            }

            // 返回 searXNG 全量结果（如启用 rerank，则返回重排后的全量结果）
            response.results = results;
            response.success = true;
            record_searxng_end(
                "success",
                category_key,
                Some(status.as_u16()),
                response.results.len(),
                started_at,
            );
            Ok(response)
        }
        .instrument(span)
        .await
    }
}

fn record_searxng_end(
    outcome: &'static str,
    category: &str,
    status_code: Option<u16>,
    result_count: usize,
    started_at: Instant,
) {
    let status_code_label = status_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "none".to_string());
    let attrs = [
        key("category", category.to_string()),
        key("outcome", outcome),
        key("status_code", status_code_label),
    ];
    let metrics = metrics();
    metrics.searxng_requests.add(1, &attrs);
    metrics
        .searxng_request_duration_ms
        .record(started_at.elapsed().as_secs_f64() * 1000.0, &attrs);
    metrics.searxng_result_count.record(
        result_count as u64,
        &[key("category", category.to_string())],
    );

    let current_span = tracing::Span::current();
    current_span.record("searxng.outcome", outcome);
    current_span.record("searxng.result.mapped_count", result_count as i64);
    current_span.set_attribute("searxng.result.mapped_count", result_count as i64);
    set_current_span_status(outcome == "success", outcome);
}

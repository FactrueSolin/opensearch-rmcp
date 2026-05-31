use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{Instrument, info_span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::searxng::{
    client::SearxngClient,
    types::{OpenSearchResponse, QuerySearchResult},
};
use crate::telemetry::{key, metrics, query_hash, set_current_span_status};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SearchType {
    General,
    News,
    Images,
    Videos,
    Science,
}

impl SearchType {
    fn as_category(&self) -> Option<&'static str> {
        match self {
            SearchType::General => None,
            SearchType::News => Some("news"),
            SearchType::Images => Some("images"),
            SearchType::Videos => Some("videos"),
            SearchType::Science => Some("science"),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            SearchType::General => "general",
            SearchType::News => "news",
            SearchType::Images => "images",
            SearchType::Videos => "videos",
            SearchType::Science => "science",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OpenSearchParams {
    pub query: String,
    #[serde(default)]
    pub search_type: Option<SearchType>,
}

#[derive(Clone)]
pub struct SearxngTools {
    client: SearxngClient,
    tool_router: ToolRouter<Self>,
}

impl SearxngTools {
    pub fn new(client: SearxngClient) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
        }
    }

    async fn run_open_search(
        &self,
        params: Parameters<OpenSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let search_type = params.0.search_type.unwrap_or(SearchType::General);
        let search_type_str = search_type.as_str().to_string();
        let category = search_type.as_category();
        let query = params.0.query.trim().to_string();
        let query_len = query.len() as i64;
        let query_hash = query_hash(&query);
        let started_at = Instant::now();
        let metrics = metrics();
        let in_flight_attrs = [key("tool", "opensearch")];
        metrics.mcp_tool_in_flight.add(1, &in_flight_attrs);

        let span = info_span!(
            "mcp.tool",
            otel.kind = "server",
            "mcp.tool.name" = "opensearch",
            "mcp.search_type" = %search_type_str,
            "mcp.query.length" = query_len,
            "mcp.query.hash" = %query_hash,
            "mcp.query.empty" = query.is_empty(),
            "mcp.success" = tracing::field::Empty,
            "mcp.result.count" = tracing::field::Empty,
            "mcp.outcome" = tracing::field::Empty,
        );

        async move {
            let mut outcome = "success";
            let mut result_count = 0usize;

            let response = if query.is_empty() {
                outcome = "empty_query";
                OpenSearchResponse {
                    success: false,
                    search_type: search_type_str.clone(),
                    results: Vec::new(),
                    error: Some("query must not be empty".to_string()),
                }
            } else {
                let query_result = match self.client.search(&query, category).await {
                    Ok(response) => QuerySearchResult {
                        query: response.query,
                        success: response.success,
                        results: response.results,
                        error: response.error,
                    },
                    Err(err) => {
                        outcome = "searxng_error";
                        QuerySearchResult {
                            query,
                            success: false,
                            results: Vec::new(),
                            error: Some(err.to_string()),
                        }
                    }
                };

                result_count = query_result.results.len();

                OpenSearchResponse {
                    success: query_result.success,
                    search_type: search_type_str.clone(),
                    results: vec![query_result],
                    error: None,
                }
            };
            let success = response.success;
            let result = Ok(Self::response_to_result(response));

            let attrs = [
                key("tool", "opensearch"),
                key("search_type", search_type_str.clone()),
                key("outcome", outcome),
            ];
            metrics.mcp_tool_calls.add(1, &attrs);
            metrics
                .mcp_tool_duration_ms
                .record(started_at.elapsed().as_secs_f64() * 1000.0, &attrs);
            metrics.mcp_tool_in_flight.add(-1, &in_flight_attrs);

            let current_span = tracing::Span::current();
            current_span.record("mcp.success", success);
            current_span.record("mcp.result.count", result_count as i64);
            current_span.record("mcp.outcome", outcome);
            current_span.set_attribute("mcp.result.count", result_count as i64);
            set_current_span_status(success, outcome);

            result
        }
        .instrument(span)
        .await
    }

    fn response_to_result(response: OpenSearchResponse) -> CallToolResult {
        match serde_json::to_value(&response) {
            Ok(value) => CallToolResult::structured(value),
            Err(err) => {
                let fallback_results = response
                    .results
                    .into_iter()
                    .map(|item| {
                        serde_json::json!({
                            "query": item.query,
                            "success": item.success,
                            "results": item
                                .results
                                .into_iter()
                                .map(|result| {
                                    serde_json::json!({
                                        "url": result.url,
                                        "description": result.description,
                                    })
                                })
                                .collect::<Vec<_>>(),
                            "error": item.error,
                        })
                    })
                    .collect::<Vec<_>>();
                let fallback = serde_json::json!({
                    "success": response.success,
                    "search_type": response.search_type,
                    "results": fallback_results,
                    "error": response
                        .error
                        .or_else(|| Some(format!("structured serialization failed: {}", err))),
                });
                let fallback_text = serde_json::to_string(&fallback).unwrap_or_else(|_| {
                    "{\"success\":false,\"error\":\"fallback serialization failed\"}".to_string()
                });
                CallToolResult::success(vec![Content::text(fallback_text)])
            }
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for SearxngTools {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("搜索服务，提供 opensearch 工具；opensearch 支持按 search_type 选择类别并对单个 query 进行查询".to_string()),
            ..Default::default()
        }
    }
}

#[tool_router]
impl SearxngTools {
    #[tool(
        name = "opensearch",
        description = "搜索工具：search type 支持 general（通用搜索）；news（新闻搜索）；images（图示搜索）；videos（视频搜索）；science（学术搜索）。一次请求只接受一个 query 关键词，在消息中标注消息来源"
    )]
    async fn opensearch(
        &self,
        params: Parameters<OpenSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        self.run_open_search(params).await
    }
}

use futures::future::join_all;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::searxng::{
    client::SearxngClient,
    types::{OpenSearchResponse, QuerySearchResult},
};

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
    pub queries: Vec<String>,
    #[serde(default)]
    pub search_type: Option<SearchType>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Clone)]
pub struct SearxngTools {
    client: SearxngClient,
    tool_router: ToolRouter<Self>,
}

impl SearxngTools {
    const MAX_LIMIT: usize = 50;

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

        if params.0.queries.is_empty() {
            return Ok(Self::response_to_result(OpenSearchResponse {
                success: false,
                search_type: search_type_str,
                results: Vec::new(),
                error: Some("queries must not be empty".to_string()),
            }));
        }

        let limit = params.0.limit.unwrap_or(20);

        if limit == 0 {
            return Ok(Self::response_to_result(OpenSearchResponse {
                success: false,
                search_type: search_type_str,
                results: Vec::new(),
                error: Some("limit must be greater than 0".to_string()),
            }));
        }

        let limit = limit.min(Self::MAX_LIMIT);
        let tasks = params.0.queries.into_iter().map(|query| {
            let trimmed = query.trim().to_string();
            let client = self.client.clone();
            async move {
                if trimmed.is_empty() {
                    return QuerySearchResult {
                        query: trimmed,
                        success: false,
                        results: Vec::new(),
                        error: Some("query is empty".to_string()),
                    };
                }

                match client.search(&trimmed, category, limit).await {
                    Ok(response) => QuerySearchResult {
                        query: response.query,
                        success: response.success,
                        results: response.results,
                        error: response.error,
                    },
                    Err(err) => QuerySearchResult {
                        query: trimmed,
                        success: false,
                        results: Vec::new(),
                        error: Some(err.to_string()),
                    },
                }
            }
        });

        let aggregated_results = join_all(tasks).await;
        let success = aggregated_results.iter().any(|item| item.success);

        Ok(Self::response_to_result(OpenSearchResponse {
            success,
            search_type: search_type_str,
            results: aggregated_results,
            error: None,
        }))
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
            instructions: Some("搜索服务，提供 opensearch 工具；opensearch 支持按 search_type 选择类别并对 queries 并发查询".to_string()),
            ..Default::default()
        }
    }
}

#[tool_router]
impl SearxngTools {
    #[tool(
        name = "opensearch",
        description = "搜索工具：search type 支持 general（通用搜索）；news（新闻搜索）；images（图示搜索）；videos（视频搜索）；science（学术搜索）。可同时搜索多个关键词，在消息中标注消息来源"
    )]
    async fn opensearch(
        &self,
        params: Parameters<OpenSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        self.run_open_search(params).await
    }
}

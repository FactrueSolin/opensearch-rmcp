use reqwest::Client;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImageSearchItem {
    pub image_url: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImageSearchResult {
    pub query: String,
    pub success: bool,
    pub images: Vec<ImageSearchItem>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImageSearchResponse {
    pub success: bool,
    pub results: Vec<ImageSearchResult>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearxResponse {
    #[serde(default)]
    results: Vec<SearxImageResult>,
}

#[derive(Debug, Deserialize)]
struct SearxImageResult {
    title: Option<String>,
    img_src: Option<String>,
}

pub async fn search_images(keywords: &[String], limit: usize) -> ImageSearchResponse {
    let base_url = match resolve_searxng_url() {
        Ok(url) => url,
        Err(err) => {
            return ImageSearchResponse {
                success: false,
                results: Vec::new(),
                error: Some(err),
            };
        }
    };

    let filtered: Vec<String> = keywords
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    if filtered.is_empty() {
        return ImageSearchResponse {
            success: false,
            results: Vec::new(),
            error: Some("keywords is empty".to_string()),
        };
    }

    let client = Client::new();
    let mut join_set = tokio::task::JoinSet::new();

    for (index, keyword) in filtered.into_iter().enumerate() {
        let client = client.clone();
        let base_url = base_url.clone();
        let limit = limit;
        join_set.spawn(async move {
            (
                index,
                search_single(&client, &base_url, &keyword, limit).await,
            )
        });
    }

    let mut results = Vec::new();
    let mut errors = Vec::new();

    while let Some(task) = join_set.join_next().await {
        match task {
            Ok((index, result)) => {
                if !result.success {
                    if let Some(error) = result.error.clone() {
                        errors.push(format!("{}: {}", result.query, error));
                    }
                }
                results.push((index, result));
            }
            Err(err) => errors.push(format!("join task failed: {err}")),
        }
    }

    results.sort_by_key(|(index, _)| *index);
    let ordered = results.into_iter().map(|(_, result)| result).collect();

    ImageSearchResponse {
        success: errors.is_empty(),
        results: ordered,
        error: if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        },
    }
}

async fn search_single(
    client: &Client,
    base_url: &str,
    keyword: &str,
    limit: usize,
) -> ImageSearchResult {
    let url = format!("{}/search", base_url.trim_end_matches('/'));
    let response = match client
        .get(url)
        .query(&[("q", keyword), ("categories", "images"), ("format", "json")])
        .send()
        .await
    {
        Ok(response) => response,
        Err(err) => {
            return ImageSearchResult {
                query: keyword.to_string(),
                success: false,
                images: Vec::new(),
                error: Some(format!("request failed: {err}")),
            };
        }
    };

    if !response.status().is_success() {
        return ImageSearchResult {
            query: keyword.to_string(),
            success: false,
            images: Vec::new(),
            error: Some(format!("request failed with status {}", response.status())),
        };
    }

    let payload: SearxResponse = match response.json().await {
        Ok(payload) => payload,
        Err(err) => {
            return ImageSearchResult {
                query: keyword.to_string(),
                success: false,
                images: Vec::new(),
                error: Some(format!("decode response failed: {err}")),
            };
        }
    };

    let images = payload
        .results
        .into_iter()
        .filter_map(|item| {
            let image_url = item.img_src.and_then(|value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            });
            let description = item.title.and_then(|value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            });
            match (image_url, description) {
                (Some(image_url), Some(description)) => Some(ImageSearchItem {
                    image_url,
                    description,
                }),
                _ => None,
            }
        })
        .take(limit)
        .collect();

    ImageSearchResult {
        query: keyword.to_string(),
        success: true,
        images,
        error: None,
    }
}

fn resolve_searxng_url() -> Result<String, String> {
    let value = std::env::var("SEARXNG_URL").unwrap_or_default();
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("SEARXNG_URL is required".to_string());
    }
    Ok(trimmed.to_string())
}

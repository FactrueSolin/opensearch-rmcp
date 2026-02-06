use super::types::{SearchResult, SearxngResultItem};

pub fn map_result_item(category: &str, item: SearxngResultItem) -> Option<SearchResult> {
    match category {
        "images" => map_image_item(item),
        _ => map_text_item(item),
    }
}

fn map_text_item(item: SearxngResultItem) -> Option<SearchResult> {
    let url = normalize(item.url)?;
    let description = normalize(item.content).or_else(|| normalize(item.title))?;
    Some(SearchResult { url, description })
}

fn map_image_item(item: SearxngResultItem) -> Option<SearchResult> {
    let url = normalize(item.img_src)?;
    let description = normalize(item.title).or_else(|| normalize(item.content))?;
    Some(SearchResult { url, description })
}

fn normalize(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

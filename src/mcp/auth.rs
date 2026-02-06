use axum::{
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct AuthState {
    token: Option<String>,
}

impl AuthState {
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }

    pub fn enabled(&self) -> bool {
        self.token.is_some()
    }

    fn is_valid(&self, token: &str) -> bool {
        self.token
            .as_ref()
            .map(|expected| expected == token)
            .unwrap_or(true)
    }
}

fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|auth_header| auth_header.strip_prefix("Bearer "))
        .map(|value| value.to_string())
}

pub async fn auth_middleware(
    State(state): State<Arc<AuthState>>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    match extract_token(&headers) {
        Some(token) if state.is_valid(&token) => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

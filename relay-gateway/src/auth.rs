use crate::state::SharedState;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};

/// Extractor that verifies the Bearer token in the Authorization header
/// matches the `api_token` in the gateway configuration.
pub struct ApiAuth;

#[async_trait]
impl FromRequestParts<SharedState> for ApiAuth {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &SharedState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts.headers.get("Authorization");

        if let Some(auth_value) = auth_header {
            if let Ok(auth_str) = auth_value.to_str() {
                if let Some(token) = auth_str.strip_prefix("Bearer ") {
                    if token == state.config.api_token {
                        return Ok(ApiAuth);
                    }
                }
            }
        }

        Err((StatusCode::UNAUTHORIZED, "Invalid or missing API token"))
    }
}

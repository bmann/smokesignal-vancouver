use anyhow::Result;
use axum::{
    extract::State,
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::http::{context::WebContext, errors::WebError};
use crate::jose::jwk::{WrappedJsonWebKey, WrappedJsonWebKeySet};

// Function to compute JWKS data and serialize to JSON string
fn compute_jwks_json(web_context: &WebContext) -> Result<String, serde_json::Error> {
    let mut keys = vec![];
    let signing_keys = web_context.config.signing_keys.as_ref();

    for available_signing_key in web_context.config.oauth_active_keys.as_ref() {
        let available_signing_key = available_signing_key.clone();

        let signing_key = match signing_keys.get(&available_signing_key) {
            Some(key) => key.clone(),
            None => continue,
        };
        let public_key = signing_key.public_key();

        let wrapped_json_web_key = WrappedJsonWebKey {
            jwk: public_key.to_jwk(),
            kid: Some(available_signing_key.clone()),
            alg: Some("ES256".to_string()),
        };

        keys.push(wrapped_json_web_key);
    }

    let jwks = WrappedJsonWebKeySet { keys };
    serde_json::to_string(&jwks)
}

// Global cache for the pre-serialized JSON string
static JWKS_JSON_CACHE: once_cell::sync::OnceCell<Arc<String>> = once_cell::sync::OnceCell::new();

#[tracing::instrument(skip_all, err)]
pub async fn handle_oauth_jwks(
    State(web_context): State<WebContext>,
) -> Result<impl IntoResponse, WebError> {
    tracing::debug!("handle_oauth_jwks");

    // Initialize the cache if needed
    if JWKS_JSON_CACHE.get().is_none() {
        // Compute and serialize the JWKS data
        let jwks_json = compute_jwks_json(&web_context)
            .map_err(|e| anyhow::anyhow!("error-oauth-jwks-1 Failed to serialize JWKS: {}", e))?;

        // Store in cache - don't worry if another thread beat us to it
        let _ = JWKS_JSON_CACHE.set(Arc::new(jwks_json));
    }

    // By this point, the cache should be initialized - either by us or another thread
    // In the extremely unlikely event it's still not initialized, we'll create it one more time
    let jwks_json = if let Some(json) = JWKS_JSON_CACHE.get() {
        json
    } else {
        // Final attempt to compute and cache
        let jwks_json = compute_jwks_json(&web_context)
            .map_err(|e| anyhow::anyhow!("error-oauth-jwks-1 Failed to serialize JWKS: {}", e))?;

        // Create a new Arc and set it in the cache
        let json_arc = Arc::new(jwks_json);

        // This will either succeed in setting it, or another thread beat us to it
        if JWKS_JSON_CACHE.set(json_arc.clone()).is_err() {
            // Another thread set it first, so use that value
            JWKS_JSON_CACHE.get().ok_or_else(|| {
                anyhow::anyhow!("error-oauth-jwks-2 Failed to initialize JWKS cache")
            })?
        } else {
            // We set it, so we can use our local copy
            JWKS_JSON_CACHE.get().ok_or_else(|| {
                anyhow::anyhow!("error-oauth-jwks-2 Failed to initialize JWKS cache")
            })?
        }
    };

    // Create response with proper content type
    let mut response = Response::new((**jwks_json).clone());

    // Set content type to application/json
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    Ok(response)
}

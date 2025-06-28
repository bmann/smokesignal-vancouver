use axum::response::{IntoResponse, Redirect, Response};
use http::StatusCode;
use thiserror::Error;

use crate::http::utils::stringify;

/// Represents errors that can occur during web session operations.
///
/// These errors are related to the serialization and deserialization of
/// web session data used for maintaining user authentication state.
#[derive(Debug, Error)]
pub enum WebSessionError {
    /// Error when web session deserialization fails.
    ///
    /// This error occurs when attempting to deserialize a web session from JSON
    /// format, typically when retrieving a session from storage or a cookie.
    #[error("error-websession-1 Unable to deserialize WebSession: {0:?}")]
    DeserializeFailed(serde_json::Error),

    /// Error when web session serialization fails.
    ///
    /// This error occurs when attempting to serialize a web session to JSON
    /// format, typically when storing a session in storage or a cookie.
    #[error("error-websession-2 Unable to serialize WebSession: {0:?}")]
    SerializeFailed(serde_json::Error),
}

/// Represents errors that can occur during authentication middleware operations.
///
/// These errors typically happen in the authentication middleware layer when
/// processing requests, including cryptographic operations and session validation.
#[derive(Debug, Error)]
pub enum AuthMiddlewareError {
    /// Error when content signing fails.
    ///
    /// This error occurs when the authentication middleware attempts to
    /// cryptographically sign content but the operation fails.
    #[error("error-authmiddleware-1 Unable to sign content: {0:?}")]
    SigningFailed(p256::ecdsa::Error),
}

#[derive(Debug, Error)]
pub enum MiddlewareAuthError {
    #[error("error-middleware-auth-1 Access Denied: {0}")]
    AccessDenied(String),

    #[error("error-middleware-auth-2 Not Found")]
    NotFound,

    #[error("error-middleware-auth-3 Unhandled Auth Error: {0:?}")]
    Anyhow(#[from] anyhow::Error),

    #[error(transparent)]
    AuthError(#[from] AuthMiddlewareError),
}

impl IntoResponse for MiddlewareAuthError {
    fn into_response(self) -> Response {
        match self {
            MiddlewareAuthError::AccessDenied(destination) => {
                let encoded_destination = urlencoding::encode(&destination).to_string();
                let args = vec![("destination", encoded_destination.as_str())];
                let uri = format!("/oauth/login?{}", stringify(args));
                Redirect::to(&uri).into_response()
            }
            MiddlewareAuthError::NotFound => {
                tracing::error!(error = ?self, "access denied");
                (StatusCode::NOT_FOUND).into_response()
            }
            _ => {
                tracing::error!(error = ?self, "internal server error");
                (StatusCode::INTERNAL_SERVER_ERROR).into_response()
            }
        }
    }
}

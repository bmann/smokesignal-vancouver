//! # Web Error Types
//!
//! This module defines the top-level error type for the HTTP layer that aggregates
//! all domain-specific errors in the application. This allows errors to be handled
//! uniformly at the HTTP boundary and converted into appropriate HTTP responses.
//!
//! Specific error variants use their own error codes, while general errors use the
//! format: `error-web-<number> <message>: <details>`

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use thiserror::Error;

use super::admin_errors::AdminImportEventError;
use super::admin_errors::AdminImportRsvpError;
use super::common_error::CommonError;
use super::create_event_errors::CreateEventError;
use super::edit_event_error::EditEventError;
use super::event_view_errors::EventViewError;
use super::import_error::ImportError;
use super::login_error::LoginError;
use super::middleware_errors::MiddlewareAuthError;
use super::migrate_event_error::MigrateEventError;
use super::migrate_rsvp_error::MigrateRsvpError;
use super::rsvp_error::RSVPError;
use super::url_error::UrlError;

/// Represents all possible errors that can occur in the HTTP layer.
///
/// This enum serves as an aggregation point for all domain-specific errors
/// in the application, allowing them to be handled uniformly at the HTTP boundary.
///
/// Most variants use transparent error forwarding to preserve the original error message
/// and error code, while a few web-specific errors have their own error code format:
/// `error-web-<number> <message>: <details>`
#[derive(Debug, Error)]
pub enum WebError {
    /// Error when authentication middleware fails.
    ///
    /// This error occurs when there are issues with verifying a user's identity
    /// through the authentication middleware, such as invalid credentials or
    /// expired sessions.
    ///
    /// **Error Code:** `error-web-1`
    #[error("error-web-1 Middleware Auth Error: {0:?}")]
    MiddlewareAuthError(#[from] MiddlewareAuthError),

    /// Error when an unexpected error occurs that isn't covered by other error types.
    ///
    /// This error is a fallback for any unhandled errors in the system. In production,
    /// these should be rare as most errors should be properly typed.
    ///
    /// **Error Code:** `error-web-2`
    ///
    /// Note: This should be replaced with more specific error types as part of
    /// the ongoing effort to use typed errors throughout the codebase.
    #[error("error-web-2 Unhandled web error: {0:?}")]
    Anyhow(#[from] anyhow::Error),

    /// Common HTTP errors.
    ///
    /// This variant wraps errors from the common error module, which contains
    /// frequently used error types shared across HTTP handlers.
    #[error(transparent)]
    Common(#[from] CommonError),

    /// Login-related errors.
    ///
    /// This error occurs during login operations, such as invalid credentials,
    /// account lockouts, or missing account information.
    #[error(transparent)]
    Login(#[from] LoginError),

    /// Event editing errors.
    ///
    /// This error occurs when there are issues editing an event, such as
    /// permission problems or invalid event data.
    #[error(transparent)]
    EditEvent(#[from] EditEventError),

    /// Event migration errors.
    ///
    /// This error occurs when there are issues migrating events between
    /// different formats or systems.
    #[error(transparent)]
    MigrateEvent(#[from] MigrateEventError),

    /// RSVP migration errors.
    ///
    /// This error occurs when there are issues migrating RSVPs between
    /// different formats or systems.
    #[error(transparent)]
    MigrateRsvp(#[from] MigrateRsvpError),

    /// Admin RSVP import errors.
    ///
    /// This error occurs when administrators have issues importing RSVP
    /// records into the system.
    #[error(transparent)]
    AdminImportRsvp(#[from] AdminImportRsvpError),

    /// Admin event import errors.
    ///
    /// This error occurs when administrators have issues importing event
    /// records into the system.
    #[error(transparent)]
    AdminImportEvent(#[from] AdminImportEventError),

    /// RSVP-related errors.
    ///
    /// This error occurs during RSVP operations such as creation, updating,
    /// or retrieval of RSVPs.
    #[error(transparent)]
    RSVP(#[from] RSVPError),

    /// Cache operation errors.
    ///
    /// This error occurs when there are issues with cache operations such as
    /// connection failures or data invalidation problems.
    #[error(transparent)]
    Cache(#[from] crate::storage::errors::CacheError),

    /// JSON Web Key errors.
    ///
    /// This error occurs when there are issues with cryptographic keys,
    /// such as missing or invalid keys.
    #[error(transparent)]
    JwkError(#[from] crate::jose_errors::JwkError),

    /// JSON Object Signing and Encryption errors.
    ///
    /// This error occurs when there are issues with JWT operations,
    /// such as signature validation or token creation.
    #[error(transparent)]
    JoseError(#[from] crate::jose_errors::JoseError),

    /// Configuration errors.
    ///
    /// This error occurs when there are issues with application configuration,
    /// such as missing environment variables or invalid settings.
    #[error(transparent)]
    ConfigError(#[from] crate::config_errors::ConfigError),

    /// Data encoding/decoding errors.
    ///
    /// This error occurs when there are issues with data encoding or decoding,
    /// such as invalid Base64 or JSON parsing problems.
    #[error(transparent)]
    EncodingError(#[from] crate::encoding_errors::EncodingError),

    /// AT Protocol URI errors.
    ///
    /// This error occurs when there are issues with AT Protocol URIs,
    /// such as malformed DIDs or invalid handles.
    #[error(transparent)]
    UriError(#[from] crate::atproto::errors::UriError),

    /// Database storage errors.
    ///
    /// This error occurs when there are issues with database operations,
    /// such as query failures or transaction issues.
    #[error(transparent)]
    StorageError(#[from] crate::storage::errors::StorageError),

    /// Event view errors.
    ///
    /// This error occurs when there are issues with retrieving or
    /// displaying events, such as invalid parameters or missing data.
    #[error(transparent)]
    EventViewError(#[from] EventViewError),

    /// Event creation errors.
    ///
    /// This error occurs when there are issues with creating events,
    /// such as validation failures or missing required fields.
    #[error(transparent)]
    CreateEventError(#[from] CreateEventError),

    /// Event viewing errors.
    ///
    /// This error occurs when there are issues with viewing specific events,
    /// such as permission problems or invalid event identifiers.
    #[error(transparent)]
    ViewEventError(#[from] super::view_event_error::ViewEventError),

    /// Token refresh errors.
    ///
    /// This error occurs when there are issues with refreshing authentication
    /// tokens, such as expired refresh tokens or validation failures.
    #[error(transparent)]
    RefreshError(#[from] crate::refresh_tokens_errors::RefreshError),

    /// URL processing errors.
    ///
    /// This error occurs when there are issues with URL processing or validation,
    /// such as malformed URLs or invalid parameters.
    #[error(transparent)]
    UrlError(#[from] UrlError),

    /// Import-related errors.
    ///
    /// This error occurs when there are issues with importing data into the system,
    /// such as format incompatibilities or validation failures.
    #[error(transparent)]
    ImportError(#[from] ImportError),
}

/// Implementation of Axum's `IntoResponse` trait for WebError.
///
/// This implementation converts errors into appropriate HTTP responses:
/// - Authentication errors use their specialized response handling
/// - All other errors are converted to a generic 500 Internal Server Error
///   and logged with the `tracing` system.
impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        match self {
            WebError::MiddlewareAuthError(err) => err.into_response(),
            _ => {
                tracing::error!(error = ?self, "internal server error");
                (StatusCode::INTERNAL_SERVER_ERROR).into_response()
            }
        }
    }
}

use thiserror::Error;

/// Represents errors that can occur during token refresh operations.
///
/// These errors are related to the process of refreshing OAuth access tokens
/// using refresh tokens, including cryptographic operations and queue management.
#[derive(Debug, Error)]
pub enum RefreshError {
    /// Error when the secret signing key cannot be found.
    ///
    /// This error occurs when attempting to refresh a token but the necessary
    /// secret key for signing the request is not available in the configuration.
    #[error("error-refresh-1 Secret signing key not found")]
    SecretSigningKeyNotFound,

    /// Error when creating a DPoP proof for token refresh fails.
    ///
    /// This error occurs when there is an issue with the cryptographic operations
    /// required to generate a DPoP (Demonstrating Proof-of-Possession) proof.
    #[error("error-refresh-2 Failed to create DPoP proof: {0:?}")]
    DpopProofCreationFailed(elliptic_curve::Error),

    /// Error when a session cannot be placed in the refresh queue.
    ///
    /// This error occurs when there is an issue with the Redis-backed queue
    /// used to manage session refresh operations.
    #[error("error-refresh-3 Failed to place session group into refresh queue: {0:?}")]
    PlaceInRefreshQueueFailed(deadpool_redis::redis::RedisError),
}

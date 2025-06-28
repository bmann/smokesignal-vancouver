use thiserror::Error;

/// Represents errors that can occur during OAuth model operations.
///
/// These errors relate to OAuth authentication flows, session management,
/// and cryptographic operations with JWT and JWK.
#[derive(Debug, Error)]
pub enum OAuthModelError {
    /// Error when creating a DPoP secret from a JWK fails.
    ///
    /// This error occurs when attempting to convert a JSON Web Key (JWK)
    /// into a secret key for DPoP (Demonstrating Proof-of-Possession) operations,
    /// typically due to invalid key format or cryptographic errors.
    #[error("error-oauth-model-1 Failed to create DPoP secret from JWK: {0:?}")]
    DpopSecretFromJwkFailed(elliptic_curve::Error),

    /// Error when the OAuth flow state is invalid.
    ///
    /// This error occurs when the state parameter in an OAuth flow
    /// does not match the expected value or cannot be verified,
    /// which could indicate a potential CSRF attack or session mismatch.
    #[error("error-oauth-model-2 Invalid OAuth flow state")]
    InvalidOAuthFlowState(),

    /// Error when required OAuth session data is missing.
    ///
    /// This error occurs when attempting to use an OAuth session
    /// that is missing critical data needed for authentication or
    /// authorization operations, such as tokens or session identifiers.
    #[error("error-oauth-model-3 Missing required OAuth session data")]
    MissingRequiredOAuthSessionData(),

    /// Error when an OAuth session has expired.
    ///
    /// This error occurs when attempting to use an OAuth session
    /// that has exceeded its validity period and is no longer usable
    /// for authentication or authorization purposes.
    #[error("error-oauth-model-4 OAuth session has expired")]
    OAuthSessionExpired(),
}

/// Represents errors that can occur during storage and database operations.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Error when a web session cannot be found in the database.
    ///
    /// This error occurs when attempting to retrieve a web session using an
    /// invalid or expired session ID.
    #[error("error-storage-1 Web session not found")]
    WebSessionNotFound,

    /// Error when a handle cannot be found in the database.
    ///
    /// This error occurs when attempting to retrieve a user handle that
    /// doesn't exist in the system.
    #[error("error-storage-2 Handle not found")]
    HandleNotFound,

    /// Error when a database record cannot be found.
    ///
    /// This error occurs when attempting to retrieve a specific record
    /// using an ID or other identifier that doesn't exist in the database.
    #[error("error-storage-3 Record not found: {0} {1:?}")]
    RowNotFound(String, sqlx::Error),

    /// Error when a database transaction cannot be committed.
    ///
    /// This error occurs when there is an issue finalizing a database
    /// transaction, potentially causing data inconsistency.
    #[error("error-storage-4 Cannot commit database transaction: {0:?}")]
    CannotCommitDatabaseTransaction(sqlx::Error),

    /// Error when a database transaction cannot be started.
    ///
    /// This error occurs when there is an issue initiating a database
    /// transaction, typically due to connection issues or database constraints.
    #[error("error-storage-5 Cannot begin database transaction: {0:?}")]
    CannotBeginDatabaseTransaction(sqlx::Error),

    /// Error when a database query cannot be executed.
    ///
    /// This error occurs when a SQL query fails to execute, typically due to
    /// syntax errors, constraint violations, or database connectivity issues.
    #[error("error-storage-6 Unable to execute query: {0:?}")]
    UnableToExecuteQuery(sqlx::Error),

    /// Error when an OAuth request cannot be found.
    ///
    /// This error occurs when attempting to retrieve an OAuth request
    /// that doesn't exist or has expired.
    #[error("error-storage-7 OAuth request not found")]
    OAuthRequestNotFound,

    /// Error when an RSVP cannot be found.
    ///
    /// This error occurs when attempting to retrieve an RSVP record
    /// that doesn't exist in the database.
    #[error("error-storage-8 RSVP not found")]
    RSVPNotFound,

    /// Error when an OAuth model operation fails.
    ///
    /// This error occurs when there's an issue with OAuth model operations,
    /// such as token generation, validation, or storage.
    #[error("error-storage-9 OAuth model error: {0}")]
    OAuthModelError(#[from] OAuthModelError),
}

/// Represents errors that can occur during cache operations.
#[derive(Debug, Error)]
pub enum CacheError {
    /// Error when a cache pool cannot be created.
    ///
    /// This error occurs when the system fails to initialize the Redis
    /// connection pool, typically due to configuration or connectivity issues.
    #[error("error-cache-1 Failed to create cache pool: {0:?}")]
    FailedToCreatePool(deadpool_redis::CreatePoolError),

    /// Error when a cache connection cannot be obtained.
    ///
    /// This error occurs when the system fails to get a connection from
    /// the Redis connection pool, typically due to pool exhaustion or connectivity issues.
    #[error("error-cache-2 Failed to get connection: {0:?}")]
    FailedToGetConnection(deadpool_redis::PoolError),

    /// Error when a session cannot be placed in the refresh queue.
    ///
    /// This error occurs when the system fails to add a session to the
    /// Redis-backed refresh queue, typically due to Redis errors or connectivity issues.
    #[error("error-cache-3 Failed to place session group into refresh queue: {0:?}")]
    FailedToPlaceInRefreshQueue(deadpool_redis::redis::RedisError),
}

use thiserror::Error;

/// Represents errors that can occur during OAuth client operations.
///
/// These errors are related to the OAuth client functionality, including
/// interacting with authorization servers, protected resources, and token management.
#[derive(Debug, Error)]
pub enum OAuthClientError {
    /// Error when a request to the authorization server fails.
    ///
    /// This error occurs when the OAuth client fails to establish a connection
    /// or complete a request to the authorization server.
    #[error("error-oauth-client-1 Authorization Server Request Failed: {0:?}")]
    AuthorizationServerRequestFailed(reqwest::Error),

    /// Error when the authorization server response is malformed.
    ///
    /// This error occurs when the response from the authorization server
    /// cannot be properly parsed or processed.
    #[error("error-oauth-client-2 Malformed Authorization Server Response: {0:?}")]
    MalformedAuthorizationServerResponse(reqwest::Error),

    /// Error when the authorization server response is invalid.
    ///
    /// This error occurs when the response from the authorization server
    /// is well-formed but contains invalid or unexpected data.
    #[error("error-oauth-client-3 Invalid Authorization Server Response: {0:?}")]
    InvalidAuthorizationServerResponse(anyhow::Error),

    /// Error when an OAuth protected resource is invalid.
    ///
    /// This error occurs when trying to access a protected resource that
    /// is not properly configured for OAuth access.
    #[error("error-oauth-client-4 Invalid OAuth Protected Resource")]
    InvalidOAuthProtectedResource,

    /// Error when a request to an OAuth protected resource fails.
    ///
    /// This error occurs when the OAuth client fails to establish a connection
    /// or complete a request to a protected resource.
    #[error("error-oauth-client-5 OAuth Protected Resource Request Failed: {0:?}")]
    OAuthProtectedResourceRequestFailed(reqwest::Error),

    /// Error when a protected resource response is malformed.
    ///
    /// This error occurs when the response from a protected resource
    /// cannot be properly parsed or processed.
    #[error("error-oauth-client-6 Malformed OAuth Protected Resource Response: {0:?}")]
    MalformedOAuthProtectedResourceResponse(reqwest::Error),

    /// Error when a protected resource response is invalid.
    ///
    /// This error occurs when the response from a protected resource
    /// is well-formed but contains invalid or unexpected data.
    #[error("error-oauth-client-7 Invalid OAuth Protected Resource Response: {0:?}")]
    InvalidOAuthProtectedResourceResponse(anyhow::Error),

    /// Error when a PAR middleware request fails.
    ///
    /// This error occurs when a Pushed Authorization Request (PAR) middleware
    /// request fails to complete successfully.
    #[error("error-oauth-client-8 PAR Middleware Request Failed: {0:?}")]
    PARMiddlewareRequestFailed(reqwest_middleware::Error),

    /// Error when a PAR request fails.
    ///
    /// This error occurs when a Pushed Authorization Request (PAR)
    /// fails to be properly processed by the authorization server.
    #[error("error-oauth-client-9 PAR Request Failed: {0:?}")]
    PARRequestFailed(reqwest::Error),

    /// Error when a PAR response is malformed.
    ///
    /// This error occurs when the response from a Pushed Authorization
    /// Request (PAR) cannot be properly parsed or processed.
    #[error("error-oauth-client-10 Malformed PAR Response: {0:?}")]
    MalformedPARResponse(reqwest::Error),

    /// Error when token minting fails.
    ///
    /// This error occurs when the system fails to mint (create) a new
    /// OAuth token, typically due to cryptographic or validation issues.
    #[error("error-oauth-client-11 Token minting failed: {0:?}")]
    MintTokenFailed(anyhow::Error),

    /// Error when a token response is malformed.
    ///
    /// This error occurs when the response containing a token cannot
    /// be properly parsed or processed.
    #[error("error-oauth-client-12 Malformed Token Response: {0:?}")]
    MalformedTokenResponse(reqwest::Error),

    /// Error when a token middleware request fails.
    ///
    /// This error occurs when a token-related middleware request
    /// fails to complete successfully.
    #[error("error-oauth-client-13 Token Request Failed: {0:?}")]
    TokenMiddlewareRequestFailed(reqwest_middleware::Error),
}

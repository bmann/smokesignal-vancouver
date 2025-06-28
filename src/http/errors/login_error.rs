use thiserror::Error;

/// Represents errors that can occur during user login and authentication.
///
/// These errors typically happen during the authentication process when users
/// are logging in to the application, including OAuth flows and DID validation.
#[derive(Debug, Error)]
pub enum LoginError {
    /// Error when a DID document does not contain a handle.
    ///
    /// This error occurs during authentication when the user's DID document
    /// is retrieved but does not contain a required handle identifier.
    #[error("error-login-1 DID document does not contain a handle")]
    NoHandle,

    /// Error when a DID document does not contain a PDS endpoint.
    ///
    /// This error occurs during authentication when the user's DID document
    /// is retrieved but does not contain a required AT Protocol Personal
    /// Data Server (PDS) endpoint.
    #[error("error-login-2 DID document does not contain an AT Protocol PDS endpoint")]
    NoPDS,

    /// Error when an OAuth callback is incomplete.
    ///
    /// This error occurs when the OAuth authentication flow callback
    /// returns with incomplete information, preventing successful authentication.
    #[error("error-login-100 OAuth callback incomplete")]
    OAuthCallbackIncomplete,

    /// Error when there is an OAuth issuer mismatch.
    ///
    /// This error occurs when the issuer in the OAuth response does not
    /// match the expected issuer, which could indicate a security issue.
    #[error("error-login-101 OAuth issuer mismatch")]
    OAuthIssuerMismatch,
}

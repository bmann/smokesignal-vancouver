use thiserror::Error;

/// Represents errors that can occur during JOSE (JSON Object Signing and Encryption) operations.
///
/// These errors are related to JSON Web Token (JWT) signing and verification,
/// JSON Web Key (JWK) operations, and DPoP (Demonstrating Proof-of-Possession) functionality.
#[derive(Debug, Error)]
pub enum JoseError {
    /// Error when token signing fails.
    ///
    /// This error occurs when the application tries to sign a JWT token
    /// using an ECDSA signing key but the signing operation fails.
    #[error("error-jose-1 Failed to sign token: {0:?}")]
    SigningFailed(p256::ecdsa::Error),

    /// Error when a required signing key is not found.
    ///
    /// This error occurs when the application tries to use a signing key
    /// that is not available in the loaded configuration.
    #[error("error-jose-2 Signing key not found")]
    SigningKeyNotFound,

    /// Error when a simple error cannot be parsed.
    ///
    /// This error occurs when the application fails to parse an error
    /// response from an OAuth server.
    #[error("error-jose-3 Unable to parse simple error")]
    UnableToParseSimpleError,

    /// Error when a required DPoP header is missing.
    ///
    /// This error occurs when making a request to a protected resource
    /// that requires a DPoP header, but the header is not present.
    #[error("error-jose-4 Missing DPoP header")]
    MissingDpopHeader,

    /// Error when a DPoP header cannot be parsed.
    ///
    /// This error occurs when the application receives a DPoP header
    /// that is malformed or contains invalid data.
    #[error("error-jose-5 Unable to parse DPoP header: {0}")]
    UnableToParseDpopHeader(String),

    /// Error when a DPoP proof token cannot be created.
    ///
    /// This error occurs when the application fails to create a valid
    /// DPoP proof token required for accessing protected resources.
    #[error("error-jose-6 Unable to mint DPoP proof token: {0}")]
    UnableToMintDpopProofToken(String),

    /// Error when an unexpected error occurs during JOSE operations.
    ///
    /// This is a catch-all error for unexpected issues that occur
    /// during JOSE-related operations.
    #[error("error-jose-7 Unexpected error: {0}")]
    UnexpectedError(String),

    /// Error when a JWT token has an invalid format.
    ///
    /// This error occurs when a JWT token doesn't have three parts
    /// separated by periods (header.payload.signature).
    #[error("error-jose-8 Invalid token format")]
    InvalidTokenFormat,

    /// Error when a JWT header cannot be decoded or parsed.
    ///
    /// This error occurs when the header part of a JWT token contains
    /// invalid base64url-encoded data or invalid JSON.
    #[error("error-jose-9 Invalid token header")]
    InvalidHeader,

    /// Error when a JWT claims part cannot be decoded or parsed.
    ///
    /// This error occurs when the claims part of a JWT token contains
    /// invalid base64url-encoded data or invalid JSON.
    #[error("error-jose-10 Invalid token claims")]
    InvalidClaims,

    /// Error when a JWT signature cannot be decoded.
    ///
    /// This error occurs when the signature part of a JWT token contains
    /// invalid base64url-encoded data.
    #[error("error-jose-11 Invalid token signature")]
    InvalidSignature,

    /// Error when JWT signature verification fails.
    ///
    /// This error occurs when the signature of a JWT token doesn't match
    /// the expected signature computed from the header and claims.
    #[error("error-jose-12 Signature verification failed")]
    SignatureVerificationFailed,

    /// Error when a JWT token has expired.
    ///
    /// This error occurs when the current time is past the expiration
    /// time (exp) specified in the JWT claims.
    #[error("error-jose-13 Token has expired")]
    TokenExpired,

    /// Error when a JWT token is not yet valid.
    ///
    /// This error occurs when the current time is before the not-before
    /// time (nbf) specified in the JWT claims.
    #[error("error-jose-14 Token is not yet valid")]
    TokenNotYetValid,

    /// Error when the system time cannot be determined.
    ///
    /// This rare error occurs when the system time cannot be retrieved
    /// or is invalid.
    #[error("error-jose-15 System time error")]
    SystemTimeError,

    /// Error when a JWT token uses an unsupported algorithm.
    ///
    /// This error occurs when the JWT token uses an algorithm (alg)
    /// that the application doesn't support or allow.
    #[error("error-jose-16 Unsupported algorithm")]
    UnsupportedAlgorithm,

    /// Error when a JWT token has invalid key parameters.
    ///
    /// This error occurs when the JWT token uses key parameters that
    /// are invalid or not supported.
    #[error("error-jose-17 Invalid key parameters: {0}")]
    InvalidKeyParameters(String),
}

/// Represents errors that can occur during JSON Web Key (JWK) operations.
///
/// These errors relate to operations with cryptographic keys in JWK format.
#[derive(Debug, Error)]
pub enum JwkError {
    /// Error when a secret JWK key is not found.
    ///
    /// This error occurs when the application tries to use a secret JWK key
    /// that is not available in the loaded configuration.
    #[error("error-jwk-1 Secret JWK key not found")]
    SecretKeyNotFound,
}

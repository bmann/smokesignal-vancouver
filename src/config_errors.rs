use thiserror::Error;

/// Represents errors that can occur during application configuration.
///
/// These errors typically happen during application startup when loading
/// and validating configuration from environment variables and files.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Error when a required environment variable is not set.
    ///
    /// This error occurs when the application starts up and a required
    /// environment variable is missing from the execution environment.
    #[error("error-config-1 {0} must be set")]
    EnvVarRequired(String),

    /// Error when the signing keys file cannot be read.
    ///
    /// This error occurs when the application fails to read the file
    /// containing signing keys, typically due to file system permissions
    /// or missing file issues.
    #[error("error-config-2 Unable to read signing keys file: {0:?}")]
    ReadSigningKeysFailed(std::io::Error),

    /// Error when the signing keys file cannot be parsed.
    ///
    /// This error occurs when the signing keys file contains malformed JSON
    /// that cannot be properly deserialized.
    #[error("error-config-3 Unable to parse signing keys file: {0:?}")]
    ParseSigningKeysFailed(serde_json::Error),

    /// Error when no valid signing keys are found.
    ///
    /// This error occurs when the signing keys file does not contain any
    /// valid keys that the application can use for signing operations.
    #[error("error-config-4 Signing keys must contain at least one valid key")]
    EmptySigningKeys,

    /// Error when the destination key is invalid.
    ///
    /// This error occurs when the DESTINATION_KEY environment variable
    /// does not reference a valid key in the SIGNING_KEYS file.
    #[error("error-config-5 DESTINATION_KEY must be a valid key in the SIGNING_KEYS file")]
    InvalidDestinationKey,

    /// Error when no valid OAuth active keys are found.
    ///
    /// This error occurs when the configuration does not include any
    /// valid keys that can be used for OAuth operations.
    #[error("error-config-6 OAuth active keys must contain at least one valid key")]
    EmptyOAuthActiveKeys,

    /// Error when no valid invitation active keys are found.
    ///
    /// This error occurs when the configuration does not include any
    /// valid keys that can be used for invitation operations.
    #[error("error-config-7 Invitation active keys must contain at least one valid key")]
    EmptyInvitationActiveKeys,

    /// Error when the PORT environment variable cannot be parsed.
    ///
    /// This error occurs when the PORT environment variable contains a value
    /// that cannot be parsed as a valid u16 integer.
    #[error("error-config-8 Parsing PORT into u16 failed: {0:?}")]
    PortParsingFailed(std::num::ParseIntError),

    /// Error when the HTTP_COOKIE_KEY cannot be decoded.
    ///
    /// This error occurs when the HTTP_COOKIE_KEY environment variable
    /// contains a value that is not valid base64-encoded data.
    #[error("error-config-9 Unable to base64 decode HTTP_COOKIE_KEY: {0:?}")]
    CookieKeyDecodeFailed(base64::DecodeSliceError),

    /// Error when the decoded HTTP_COOKIE_KEY cannot be processed.
    ///
    /// This error occurs when the decoded HTTP_COOKIE_KEY has an invalid
    /// format or length that prevents it from being used.
    #[error("error-config-10 Unable to process decoded HTTP_COOKIE_KEY")]
    CookieKeyProcessFailed,

    /// Error when version information is not available.
    ///
    /// This error occurs when neither GIT_HASH nor CARGO_PKG_VERSION
    /// environment variables are set, preventing version identification.
    #[error("error-config-11 One of GIT_HASH or CARGO_PKG_VERSION must be set")]
    VersionNotSet,

    /// Error when a referenced signing key is not found.
    ///
    /// This error occurs when attempting to use a signing key that
    /// does not exist in the loaded signing keys configuration.
    #[error("error-config-12 Signing key not found")]
    SigningKeyNotFound,

    /// Error when a DNS nameserver IP cannot be parsed.
    ///
    /// This error occurs when the DNS_NAMESERVERS environment variable contains
    /// an IP address that cannot be parsed as a valid IpAddr.
    #[error("error-config-13 Unable to parse nameserver IP '{0}': {1}")]
    NameserverParsingFailed(String, std::net::AddrParseError),

    /// Error when the signing keys file is not found.
    ///
    /// This error occurs when the file specified in the SIGNING_KEYS environment
    /// variable does not exist on the file system.
    #[error("error-config-14 Signing keys file not found: {0}")]
    SigningKeysFileNotFound(String),

    /// Error when the signing keys file is empty.
    ///
    /// This error occurs when the file specified in the SIGNING_KEYS environment
    /// variable exists but contains no data.
    #[error("error-config-15 Signing keys file is empty")]
    EmptySigningKeysFile,

    /// Error when the JWKS structure doesn't contain any keys.
    ///
    /// This error occurs when the signing keys file contains a valid JWKS structure,
    /// but the 'keys' array is empty.
    #[error("error-config-16 No keys found in JWKS")]
    MissingKeysInJWKS,

    /// Error when signing keys fail validation.
    ///
    /// This error occurs when the signing keys file contains keys
    /// that fail validation checks (such as having invalid format).
    #[error("error-config-17 Signing keys validation failed: {0:?}")]
    SigningKeysValidationFailed(Vec<String>),
}

use thiserror::Error;

/// Represents errors that can occur during URL processing and validation.
///
/// These errors typically happen when validating or processing URLs in the system,
/// including checking for supported collection types in URL paths.
#[derive(Debug, Error)]
pub enum UrlError {
    /// Error when an unsupported collection type is specified in a URL.
    ///
    /// This error occurs when a URL contains a collection type that is not
    /// supported by the system, typically in an AT Protocol URI path.
    #[error("error-url-1 Unsupported collection type")]
    UnsupportedCollection,
}

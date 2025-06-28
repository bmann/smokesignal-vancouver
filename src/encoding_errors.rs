use thiserror::Error;

/// Represents errors that can occur during data encoding and decoding operations.
///
/// These errors relate to serialization, deserialization, encoding and decoding
/// of data across various formats used throughout the application.
#[derive(Debug, Error)]
pub enum EncodingError {
    /// Error when JSON serialization fails.
    ///
    /// This error occurs when attempting to convert a Rust structure to
    /// JSON format fails, typically due to data that cannot be properly
    /// represented in JSON.
    #[error("error-encoding-1 JSON serialization failed: {0:?}")]
    JsonSerializationFailed(serde_json::Error),

    /// Error when Base64 decoding fails.
    ///
    /// This error occurs when attempting to decode Base64-encoded data
    /// that is malformed or contains invalid characters.
    #[error("error-encoding-2 Base64 decoding failed: {0:?}")]
    Base64DecodingFailed(base64::DecodeError),

    /// Error when JSON deserialization fails.
    ///
    /// This error occurs when attempting to parse JSON data into a Rust
    /// structure fails, typically due to missing fields, type mismatches,
    /// or malformed JSON.
    #[error("error-encoding-3 JSON deserialization failed: {0:?}")]
    JsonDeserializationFailed(serde_json::Error),
}

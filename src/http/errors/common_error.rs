use thiserror::Error;

/// Represents common errors that can occur across various HTTP handlers.
#[derive(Debug, Error)]
pub enum CommonError {
    /// Error when a handle slug is invalid.
    ///
    /// This error occurs when a URL contains a handle slug that doesn't conform
    /// to the expected format or contains invalid characters.
    #[error("error-common-1 Invalid handle slug")]
    InvalidHandleSlug,

    /// Error when a user lacks permission for an action.
    ///
    /// This error occurs when a user attempts to perform an action they
    /// are not authorized to do, such as modifying another user's data.
    #[error("error-common-2 Not authorized to perform this action")]
    NotAuthorized,

    /// Error when an unsupported event type is encountered.
    ///
    /// This error occurs when an operation is attempted on an event type
    /// that is not supported by the current functionality.
    #[error("error-common-3 Unsupported event type")]
    UnsupportedEventType,

    /// Error when a required field is missing.
    ///
    /// This error occurs when a form or request is missing a mandatory field
    /// that is needed to complete the operation.
    #[error("error-common-4 Required field not provided")]
    FieldRequired,

    /// Error when data has an invalid format or is corrupted.
    ///
    /// This error occurs when input data doesn't match the expected format
    /// or appears to be corrupted or tampered with.
    #[error("error-common-5 Invalid format or corrupted data")]
    InvalidFormat,

    /// Error when an AT Protocol URI has an invalid format.
    ///
    /// This error occurs when an AT Protocol URI is malformed or
    /// doesn't follow the expected format specification.
    #[error("error-common-6 Invalid AT-URI format")]
    InvalidAtUri,

    /// Error when a requested record cannot be found.
    ///
    /// This error occurs when an operation is attempted on a record
    /// that doesn't exist in the database.
    #[error("error-common-7 Record not found")]
    RecordNotFound,

    /// Error when record data cannot be parsed.
    ///
    /// This error occurs when a record's data cannot be properly
    /// deserialized or parsed into the expected structure.
    #[error("error-common-8 Failed to parse record data")]
    FailedToParse,

    /// Error when event data has an invalid format or is corrupted.
    ///
    /// This error occurs when event data doesn't match the expected format
    /// or appears to be corrupted or tampered with.
    #[error("error-common-9 Invalid event format or corrupted data")]
    InvalidEventFormat,
}

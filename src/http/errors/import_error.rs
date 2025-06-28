use thiserror::Error;

/// Represents errors that can occur during data import operations.
///
/// These errors typically happen when attempting to import events and RSVPs
/// from different sources, including community and Smokesignal systems.
#[derive(Debug, Error)]
pub enum ImportError {
    /// Error when listing community events fails.
    ///
    /// This error occurs when attempting to retrieve a list of community
    /// events during an import operation fails, preventing the import.
    #[error("error-import-1 Failed to list community events: {0}")]
    FailedToListCommunityEvents(String),

    /// Error when listing community RSVPs fails.
    ///
    /// This error occurs when attempting to retrieve a list of community
    /// RSVPs during an import operation fails, preventing the import.
    #[error("error-import-2 Failed to list community RSVPs: {0}")]
    FailedToListCommunityRSVPs(String),

    /// Error when listing Smokesignal events fails.
    ///
    /// This error occurs when attempting to retrieve a list of Smokesignal
    /// events during an import operation fails, preventing the import.
    #[error("error-import-3 Failed to list Smokesignal events: {0}")]
    FailedToListSmokesignalEvents(String),

    /// Error when listing Smokesignal RSVPs fails.
    ///
    /// This error occurs when attempting to retrieve a list of Smokesignal
    /// RSVPs during an import operation fails, preventing the import.
    #[error("error-import-4 Failed to list Smokesignal RSVPs: {0}")]
    FailedToListSmokesignalRSVPs(String),

    /// Error when an unsupported collection type is specified.
    ///
    /// This error occurs when the import operation specifies a collection
    /// type that isn't supported for import operations.
    #[error("error-import-5 Unsupported collection type: {0}")]
    UnsupportedCollectionType(String),
}

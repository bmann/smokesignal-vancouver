use thiserror::Error;

/// Represents errors that can occur when viewing specific events.
///
/// These errors typically happen when retrieving or displaying specific
/// event data, including lookup failures and data enhancement issues.
#[derive(Debug, Error)]
pub enum ViewEventError {
    /// Error when a requested event cannot be found.
    ///
    /// This error occurs when attempting to view an event that doesn't
    /// exist in the system, typically due to an invalid identifier.
    #[error("error-view-event-1 Event not found: {0}")]
    EventNotFound(String),

    /// Error when a fallback retrieval method fails.
    ///
    /// This error occurs when the primary method of retrieving an event fails,
    /// and the fallback method also fails to retrieve the event.
    #[error("error-view-event-2 Failed to get event from fallback: {0}")]
    FallbackFailed(String),

    /// Error when fetching event details fails.
    ///
    /// This error occurs when the system fails to retrieve additional
    /// details for an event, such as RSVP counts or related data.
    #[error("error-view-event-3 Failed to fetch event details: {0}")]
    FetchEventDetailsFailed(String),
}

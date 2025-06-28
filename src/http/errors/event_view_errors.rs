use thiserror::Error;

/// Represents errors that can occur during event viewing operations.
///
/// These errors typically happen when retrieving and displaying event data
/// to users, including data validation and enhancement issues.
#[derive(Debug, Error)]
pub enum EventViewError {
    /// Error when an invalid collection is specified.
    ///
    /// This error occurs when an event view request specifies a collection
    /// name that doesn't exist or isn't supported by the system.
    #[error("error-event-view-1 Invalid collection: {0}")]
    InvalidCollection(String),

    /// Error when an event name is missing.
    ///
    /// This error occurs when attempting to view an event that is missing
    /// a required name field, which is necessary for display.
    #[error("error-event-view-2 Event name is missing")]
    MissingEventName,

    /// Error when RSVP count calculation fails.
    ///
    /// This error occurs when the system fails to retrieve or calculate
    /// the RSVP counts (going, interested, not going) for an event.
    #[error("error-event-view-3 Failed to hydrate event RSVP counts: {0}")]
    FailedToHydrateRsvpCounts(String),
}

use thiserror::Error;

/// Represents errors that can occur during event creation.
///
/// These errors are typically triggered during validation of user-submitted
/// event creation forms.
#[derive(Debug, Error)]
pub enum CreateEventError {
    /// Error when the event name is not provided.
    ///
    /// This error occurs when a user attempts to create an event without
    /// specifying a name, which is a required field.
    #[error("error-create-event-1 Name not set")]
    NameNotSet,

    /// Error when the event description is not provided.
    ///
    /// This error occurs when a user attempts to create an event without
    /// specifying a description, which is a required field.
    #[error("error-create-event-2 Description not set")]
    DescriptionNotSet,

    /// Error when the event dates are invalid.
    ///
    /// This error occurs when the provided event dates are invalid, such as when
    /// the end date is before the start date, or dates are outside allowed ranges.
    #[error("error-create-event-3 Invalid event dates")]
    InvalidEventDates,

    /// Error when an invalid event mode is specified.
    ///
    /// This error occurs when the provided event mode doesn't match one of the
    /// supported values (e.g., "in-person", "online", "hybrid").
    #[error("error-create-event-4 Invalid event mode")]
    InvalidEventMode,

    /// Error when an invalid event status is specified.
    ///
    /// This error occurs when the provided event status doesn't match one of the
    /// supported values (e.g., "confirmed", "tentative", "cancelled").
    #[error("error-create-event-5 Invalid event status")]
    InvalidEventStatus,
}

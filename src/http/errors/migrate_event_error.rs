use thiserror::Error;

/// Represents errors that can occur during event migration operations.
///
/// These errors typically happen when attempting to migrate events between
/// different formats or systems, such as from smokesignal to community events.
#[derive(Debug, Error)]
pub enum MigrateEventError {
    /// Error when an invalid handle slug is provided.
    ///
    /// This error occurs when attempting to migrate an event with a handle slug
    /// that is not properly formatted or does not exist in the system.
    #[error("error-migrate-event-1 Invalid handle slug")]
    InvalidHandleSlug,

    /// Error when a user is not authorized to migrate an event.
    ///
    /// This error occurs when a user attempts to migrate an event that they
    /// do not have permission to modify, typically because they are not
    /// the event creator or an administrator.
    #[error("error-migrate-event-2 Not authorized to migrate this event")]
    NotAuthorized,

    /// Error when attempting to migrate an unsupported event type.
    ///
    /// This error occurs when a user attempts to migrate an event type that
    /// cannot be migrated, as only smokesignal events can be converted to
    /// the community event format.
    #[error(
        "error-migrate-event-3 Unsupported event type. Only smokesignal events can be migrated"
    )]
    UnsupportedEventType,

    /// Error when an event has already been migrated.
    ///
    /// This error occurs when attempting to migrate an event that is already
    /// in the community event format, which would be redundant.
    #[error("error-migrate-event-4 Event is already a community event")]
    AlreadyMigrated,

    /// Error when a destination URI conflict exists.
    ///
    /// This error occurs when attempting to migrate an event to a URI that
    /// already has an event associated with it, which would cause a conflict.
    #[error("error-migrate-event-5 An event already exists at the destination URI")]
    DestinationExists,
}

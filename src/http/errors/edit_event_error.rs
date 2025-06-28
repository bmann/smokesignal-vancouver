use thiserror::Error;

/// Represents errors that can occur during event editing operations.
///
/// These errors typically happen when users attempt to modify existing events
/// and encounter authorization, validation, or type compatibility issues.
#[derive(Debug, Error)]
pub enum EditEventError {
    /// Error when an invalid handle slug is provided.
    ///
    /// This error occurs when attempting to edit an event with a handle slug
    /// that is not properly formatted or does not exist in the system.
    #[error("error-edit-event-1 Invalid handle slug")]
    InvalidHandleSlug,

    /// Error when a user is not authorized to edit an event.
    ///
    /// This error occurs when a user attempts to edit an event that they
    /// do not have permission to modify, typically because they are not
    /// the event creator or an administrator.
    #[error("error-edit-event-2 Not authorized to edit this event")]
    NotAuthorized,

    /// Error when attempting to edit an unsupported event type.
    ///
    /// This error occurs when a user attempts to edit an event type that
    /// does not support editing, as only community calendar events can be
    /// modified after creation.
    #[error(
        "error-edit-event-3 Unsupported event type. Only community calendar events can be edited"
    )]
    UnsupportedEventType,

    /// Error when attempting to edit location data on an event that has multiple locations.
    ///
    /// This error occurs when a user attempts to modify location information for an event
    /// that has multiple locations defined. Such events can only be edited through the API.
    #[error("error-edit-event-4 Cannot edit locations: Event has multiple locations")]
    MultipleLocationsPresent,

    /// Error when attempting to edit location data on an event that has an unsupported location type.
    ///
    /// This error occurs when a user attempts to modify location information for an event
    /// that has a location type that is not supported for editing through the web interface.
    #[error("error-edit-event-5 Cannot edit locations: Event has unsupported location type")]
    UnsupportedLocationType,

    /// Error when attempting to edit location data on an event that has no locations.
    ///
    /// This error occurs when a user attempts to add location information to an event
    /// that was not created with location information.
    #[error("error-edit-event-6 Cannot edit locations: Event has no locations")]
    NoLocationsPresent,
}

use thiserror::Error;

/// Represents errors that can occur during RSVP operations.
///
/// These errors relate to the handling of event RSVPs, such as
/// when users attempt to respond to event invitations.
#[derive(Debug, Error)]
pub enum RSVPError {
    /// Error when an RSVP cannot be found.
    ///
    /// This error occurs when attempting to retrieve or modify an RSVP
    /// that doesn't exist in the system, typically when providing an
    /// invalid AT-URI.
    #[error("error-rsvps-1 RSVP Not Found: No RSVP found with provided AT-URI.")]
    NotFound,
}

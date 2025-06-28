use thiserror::Error;

/// Represents errors that can occur during RSVP migration.
///
/// These errors relate to the migration or conversion of RSVP data
/// between different systems, formats, or versions.
#[derive(Debug, Error)]
pub enum MigrateRsvpError {
    /// Error when an invalid RSVP status is provided during migration.
    ///
    /// This error occurs when attempting to migrate an RSVP with a status
    /// that doesn't match one of the expected values ('going', 'interested',
    /// or 'notgoing').
    #[error("error-migrate-rsvp-1 Invalid RSVP status: {0}. Expected 'going', 'interested', or 'notgoing'.")]
    InvalidRsvpStatus(String),

    /// Error when a user is not authorized to migrate an RSVP.
    ///
    /// This error occurs when a user attempts to migrate an RSVP that they
    /// do not have permission to modify, typically because they are not
    /// the RSVP owner or an administrator.
    #[error("error-migrate-rsvp-2 Not authorized to migrate this RSVP")]
    NotAuthorized,
}

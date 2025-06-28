use thiserror::Error;

/// These errors relate to the process of importing RSVP data into the system
/// by administrators, typically during data migration or recovery.
#[derive(Debug, Error)]
pub enum AdminImportRsvpError {
    /// Error when an RSVP cannot be inserted during import.
    ///
    /// This error occurs when attempting to insert an imported RSVP into
    /// the database fails, typically due to data validation issues or
    /// database constraints.
    #[error("error-admin-import-rsvp-1 Failed to insert RSVP: {0}")]
    InsertFailed(String),
}

/// These errors relate to the process of importing event data into the system
/// by administrators, typically during data migration or recovery.
#[derive(Debug, Error)]
pub enum AdminImportEventError {
    /// Error when an event cannot be inserted during import.
    ///
    /// This error occurs when attempting to insert an imported event into
    /// the database fails, typically due to data validation issues or
    /// database constraints.
    #[error("error-admin-import-event-1 Failed to insert event: {0}")]
    InsertFailed(String),
}

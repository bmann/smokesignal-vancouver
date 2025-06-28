use chrono::Utc;
use metrohash::MetroHash64;
use sqlx::{Postgres, QueryBuilder};
use std::borrow::Cow;
use std::hash::Hasher;

use self::model::DenylistEntry;

use crate::storage::{errors::StorageError, StoragePool};

pub mod model {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use sqlx::FromRow;

    #[derive(Clone, FromRow, Deserialize, Serialize, Debug)]
    pub struct DenylistEntry {
        pub subject: String,
        pub reason: String,
        pub updated_at: DateTime<Utc>,
    }
}

// Add a new entry to the denylist or update an existing one
pub async fn denylist_add_or_update(
    pool: &StoragePool,
    subject: Cow<'_, str>,
    reason: Cow<'_, str>,
) -> Result<(), StorageError> {
    // Validate subject and reason before proceeding
    if subject.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Subject cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let mut h = MetroHash64::new();
    h.write(subject.as_bytes());
    let subject = crockford::encode(h.finish());

    let now = Utc::now();

    sqlx::query(
        r"
        INSERT INTO denylist (subject, reason, updated_at)
        VALUES ($1, $2, $3)
        ON CONFLICT(subject) DO UPDATE
        SET reason = $2, updated_at = $3
        ",
    )
    .bind(subject)
    .bind(reason)
    .bind(now)
    .execute(tx.as_mut())
    .await
    .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(())
}

// Remove an entry from the denylist
pub async fn denylist_remove(pool: &StoragePool, subject: &str) -> Result<(), StorageError> {
    // Validate subject before proceeding
    if subject.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Subject cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let mut h = MetroHash64::default();
    h.write(subject.as_bytes());
    let subject = crockford::encode(h.finish());

    sqlx::query("DELETE FROM denylist WHERE subject = $1")
        .bind(subject)
        .execute(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(())
}

// Check if a subject is in the denylist
pub async fn denylist_check(pool: &StoragePool, subject: &str) -> Result<bool, StorageError> {
    // Validate subject before proceeding
    if subject.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Subject cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let mut h = MetroHash64::default();
    h.write(subject.as_bytes());
    let subject = crockford::encode(h.finish());

    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM denylist WHERE subject = $1")
        .bind(subject)
        .fetch_one(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(count > 0)
}

// Get a list of denylist entries with pagination
pub async fn denylist_list(
    pool: &StoragePool,
    page: i64,
    page_size: i64,
) -> Result<(i64, Vec<DenylistEntry>), StorageError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM denylist")
        .fetch_one(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    let offset = (page - 1) * page_size;

    let entries = sqlx::query_as::<_, model::DenylistEntry>(
        "SELECT * FROM denylist ORDER BY updated_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(page_size + 1)
    .bind(offset)
    .fetch_all(tx.as_mut())
    .await
    .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok((count, entries))
}

pub async fn denylist_exists(pool: &StoragePool, subjects: &[&str]) -> Result<bool, StorageError> {
    // Validate input - empty array should return false, not error
    if subjects.is_empty() {
        return Ok(false);
    }

    // Validate that all subjects are non-empty
    for subject in subjects {
        if subject.trim().is_empty() {
            return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
                "Subject cannot be empty".into(),
            )));
        }
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    // Process subjects to get hashed values first
    let hashed_subjects: Vec<String> = subjects
        .iter()
        .map(|subject| {
            let mut h = MetroHash64::default();
            h.write(subject.as_bytes());
            crockford::encode(h.finish())
        })
        .collect();

    // Build the query with placeholders
    let mut query_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM denylist WHERE subject IN (");
    let mut separated = query_builder.separated(", ");
    for hashed_subject in &hashed_subjects {
        separated.push_bind(hashed_subject);
    }
    separated.push_unseparated(") ");

    // Use build_query_scalar to correctly include the bindings
    let query = query_builder.build_query_scalar::<i64>();
    let count = query
        .fetch_one(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(count > 0)
}

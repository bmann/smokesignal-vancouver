use std::borrow::Cow;

use chrono::Utc;
use cityhasher::HashMap;
use sqlx::{Postgres, QueryBuilder};

use crate::storage::denylist::denylist_add_or_update;
use crate::storage::errors::StorageError;
use crate::storage::StoragePool;
use model::Handle;

pub mod model {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use sqlx::FromRow;

    #[derive(Clone, FromRow, Deserialize, Serialize, Debug)]
    pub struct Handle {
        pub did: String,
        pub handle: String,
        pub pds: String,

        pub language: String,
        pub tz: String,

        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
        pub active_at: Option<DateTime<Utc>>,
    }
}

pub async fn handle_warm_up(
    pool: &StoragePool,
    did: &str,
    handle: &str,
    pds: &str,
) -> Result<(), StorageError> {
    // Validate inputs aren't empty
    if did.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "DID cannot be empty".into(),
        )));
    }

    if handle.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Handle cannot be empty".into(),
        )));
    }

    if pds.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "PDS cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let now = Utc::now();
    let insert_result = sqlx::query("INSERT INTO handles (did, handle, pds, created_at, updated_at) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING")
        .bind(did)
        .bind(handle)
        .bind(pds)
        .bind(now)
        .bind(now)
        .execute(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    if insert_result.rows_affected() == 0 {
        sqlx::query("UPDATE handles SET updated_at = $1, handle = $2, pds = $3 WHERE did = $4")
            .bind(now)
            .bind(handle)
            .bind(pds)
            .bind(did)
            .execute(tx.as_mut())
            .await
            .map_err(StorageError::UnableToExecuteQuery)?;
    }

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)
}

pub enum HandleField {
    Language(Cow<'static, str>),
    Timezone(Cow<'static, str>),
    ActiveNow,
}

pub async fn handle_update_field(
    pool: &StoragePool,
    did: &str,
    field: HandleField,
) -> Result<(), StorageError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let now = Utc::now();

    let query = match &field {
        HandleField::Language(_) => {
            "UPDATE handles SET language = $1, updated_at = $2 WHERE did = $3"
        }
        HandleField::Timezone(_) => "UPDATE handles SET tz = $1, updated_at = $2 WHERE did = $3",
        HandleField::ActiveNow => {
            "UPDATE handles SET active_at = $1, updated_at = $2 WHERE did = $3"
        }
    };

    let mut query_builder = sqlx::query(query);

    match field {
        HandleField::Language(language) => {
            query_builder = query_builder.bind(language);
        }
        HandleField::Timezone(tz) => {
            query_builder = query_builder.bind(tz);
        }
        HandleField::ActiveNow => {
            query_builder = query_builder.bind(now);
        }
    }

    query_builder
        .bind(now)
        .bind(did)
        .execute(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)
}

pub async fn handle_for_did(pool: &StoragePool, did: &str) -> Result<Handle, StorageError> {
    // Validate DID is not empty
    if did.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "DID cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let entity = sqlx::query_as::<_, Handle>("SELECT * FROM handles WHERE did = $1")
        .bind(did)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StorageError::HandleNotFound,
            other => StorageError::UnableToExecuteQuery(other),
        })?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(entity)
}

pub async fn handle_for_handle(pool: &StoragePool, handle: &str) -> Result<Handle, StorageError> {
    // Validate handle is not empty
    if handle.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Handle cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let entity = sqlx::query_as::<_, Handle>("SELECT * FROM handles WHERE handle = $1")
        .bind(handle)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StorageError::HandleNotFound,
            other => StorageError::UnableToExecuteQuery(other),
        })?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(entity)
}

pub async fn handle_list(
    pool: &StoragePool,
    page: i64,
    page_size: i64,
) -> Result<(i64, Vec<Handle>), StorageError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let total_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM handles")
        .fetch_one(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    let offset = (page - 1) * page_size;

    let handles = sqlx::query_as::<_, Handle>(
        "SELECT * FROM handles ORDER BY updated_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(page_size + 1) // Fetch one more to know if there are more entries
    .bind(offset)
    .fetch_all(tx.as_mut())
    .await
    .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok((total_count, handles))
}

// Nuke a handle and all its events and RSVPs, and add to denylist
pub async fn handle_nuke(
    pool: &StoragePool,
    did: &str,
    admin_did: &str,
) -> Result<(), StorageError> {
    // Validate inputs aren't empty
    if did.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "DID cannot be empty".into(),
        )));
    }

    if admin_did.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Admin DID cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    // Get handle information first
    let handle = sqlx::query_as::<_, Handle>("SELECT * FROM handles WHERE did = $1")
        .bind(did)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StorageError::HandleNotFound,
            other => StorageError::UnableToExecuteQuery(other),
        })?;

    // Delete RSVPs created by this identity
    sqlx::query("DELETE FROM rsvps WHERE did = $1")
        .bind(did)
        .execute(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    // Delete events created by this identity
    sqlx::query("DELETE FROM events WHERE did = $1")
        .bind(did)
        .execute(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    // Delete the handle entry
    sqlx::query("DELETE FROM handles WHERE did = $1")
        .bind(did)
        .execute(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    // Create a safe reason with proper escaping
    let handle_reason = format!(
        "{} nuked by {}",
        &handle.handle.replace('\'', ""),
        admin_did.replace('\'', "")
    );
    let pds_reason = format!(
        "{} nuked by {}",
        &handle.pds.replace('\'', ""),
        admin_did.replace('\'', "")
    );
    let did_reason = format!(
        "{} nuked by {}",
        did.replace('\'', ""),
        admin_did.replace('\'', "")
    );

    denylist_add_or_update(
        pool,
        Cow::Borrowed(&handle.handle),
        Cow::Owned(handle_reason),
    )
    .await?;
    denylist_add_or_update(pool, Cow::Borrowed(&handle.pds), Cow::Owned(pds_reason)).await?;
    denylist_add_or_update(pool, Cow::Borrowed(did), Cow::Owned(did_reason)).await?;

    Ok(())
}

pub async fn handles_by_did(
    pool: &StoragePool,
    dids: Vec<String>,
) -> Result<HashMap<std::string::String, Handle>, StorageError> {
    if dids.is_empty() {
        return Ok(HashMap::default());
    }

    // Validate all DIDs are non-empty
    for did in &dids {
        if did.trim().is_empty() {
            return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
                "DID cannot be empty".into(),
            )));
        }
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    // Build the query with placeholders
    let mut query_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT * FROM handles WHERE did IN (");
    let mut separated = query_builder.separated(", ");
    for did in &dids {
        separated.push_bind(did);
    }
    separated.push_unseparated(") ");

    // The query_builder.build() already includes the bindings, so we don't need to bind again
    let query = query_builder.build_query_as::<Handle>();
    let values = query
        .fetch_all(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(HashMap::from_iter(
        values
            .iter()
            .map(|value| (value.did.clone(), value.clone())),
    ))
}

#[cfg(test)]
pub mod test {
    use sqlx::PgPool;

    use crate::storage::handle::handle_for_did;
    use crate::storage::handle::handle_for_handle;
    use crate::storage::handle::handle_warm_up;

    #[sqlx::test(fixtures(path = "../../fixtures/storage", scripts("handles")))]
    async fn test_handle_for_did(pool: PgPool) -> sqlx::Result<()> {
        let handle = handle_for_did(&pool, "did:plc:d5c1ed6d01421a67b96f68fa").await;
        println!("result {:?}", handle);
        assert!(!handle.is_err());
        let handle = handle.unwrap();
        assert_eq!(handle.handle, "whole-crane.examplepds.com");

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures/storage", scripts("handles")))]
    async fn test_handle_for_handle(pool: PgPool) -> sqlx::Result<()> {
        let handle = handle_for_handle(&pool, "whole-crane.examplepds.com").await;
        println!("result {:?}", handle);
        assert!(!handle.is_err());
        let handle = handle.unwrap();
        assert_eq!(handle.did, "did:plc:d5c1ed6d01421a67b96f68fa");

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures/storage", scripts("handles")))]
    async fn test_handle_warm_up(pool: PgPool) -> sqlx::Result<()> {
        let did = "did:plc:f263c822655b579fc8a79635";
        let handle = "inspiring-bobwhite.examplepds.com";
        let updated_handle = "charming-needlefish.examplepds.com";
        let pds = "https://pds.examplepds.com";

        let warmup_result = handle_warm_up(&pool, did, handle, pds).await;
        assert!(!warmup_result.is_err());

        {
            let handle = handle_for_handle(&pool, handle).await;
            assert!(!handle.is_err());
            let handle = handle.unwrap();
            assert_eq!(handle.did, did);
        }

        {
            let warmup_result = handle_warm_up(&pool, did, updated_handle, pds).await;
            assert!(!warmup_result.is_err());
        }
        {
            let handle = handle_for_handle(&pool, handle).await;
            assert!(handle.is_err());
        }

        Ok(())
    }
}

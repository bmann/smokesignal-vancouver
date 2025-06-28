use std::borrow::Cow;

use chrono::{DateTime, Utc};
use serde_json::json;

use crate::{
    jose::jwk::WrappedJsonWebKey,
    storage::{errors::StorageError, handle::model::Handle, StoragePool},
};
use model::{OAuthRequest, OAuthSession};

pub struct OAuthRequestParams {
    pub oauth_state: Cow<'static, str>,
    pub issuer: Cow<'static, str>,
    pub did: Cow<'static, str>,
    pub nonce: Cow<'static, str>,
    pub pkce_verifier: Cow<'static, str>,
    pub secret_jwk_id: Cow<'static, str>,
    pub dpop_jwk: Option<WrappedJsonWebKey>,
    pub destination: Option<Cow<'static, str>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub async fn oauth_request_insert(
    pool: &StoragePool,
    params: OAuthRequestParams,
) -> Result<(), StorageError> {
    // Validate required input parameters
    if params.oauth_state.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "OAuth state cannot be empty".into(),
        )));
    }

    if params.issuer.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Issuer cannot be empty".into(),
        )));
    }

    if params.did.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "DID cannot be empty".into(),
        )));
    }

    if params.nonce.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Nonce cannot be empty".into(),
        )));
    }

    if params.pkce_verifier.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "PKCE verifier cannot be empty".into(),
        )));
    }

    if params.secret_jwk_id.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Secret JWK ID cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let dpop_jwk_value = params
        .dpop_jwk
        .map(|jwk| json!(jwk))
        .unwrap_or_else(|| json!({}));

    sqlx::query("INSERT INTO oauth_requests (oauth_state, issuer, did, nonce, pkce_verifier, secret_jwk_id, dpop_jwk, destination, created_at, expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)")
        .bind(&params.oauth_state)
        .bind(&params.issuer)
        .bind(&params.did)
        .bind(&params.nonce)
        .bind(&params.pkce_verifier)
        .bind(&params.secret_jwk_id)
        .bind(dpop_jwk_value)
        .bind(params.destination)
        .bind(params.created_at)
        .bind(params.expires_at)
        .execute(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)
}

pub async fn oauth_request_get(
    pool: &StoragePool,
    oauth_state: &str,
) -> Result<OAuthRequest, StorageError> {
    // Validate oauth_state is not empty
    if oauth_state.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "OAuth state cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let record =
        sqlx::query_as::<_, OAuthRequest>("SELECT * FROM oauth_requests WHERE oauth_state = $1")
            .bind(oauth_state)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => StorageError::OAuthRequestNotFound,
                other => StorageError::UnableToExecuteQuery(other),
            })?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok(record)
}

pub async fn oauth_request_remove(
    pool: &StoragePool,
    oauth_state: &str,
) -> Result<(), StorageError> {
    // Validate oauth_state is not empty
    if oauth_state.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "OAuth state cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    sqlx::query("DELETE FROM oauth_requests WHERE oauth_state = $1")
        .bind(oauth_state)
        .execute(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)
}

pub struct OAuthSessionParams {
    pub session_group: Cow<'static, str>,
    pub access_token: Cow<'static, str>,
    pub did: Cow<'static, str>,
    pub issuer: Cow<'static, str>,
    pub refresh_token: Cow<'static, str>,
    pub secret_jwk_id: Cow<'static, str>,
    pub dpop_jwk: WrappedJsonWebKey,
    pub created_at: DateTime<Utc>,
    pub access_token_expires_at: DateTime<Utc>,
}

pub async fn oauth_session_insert(
    pool: &StoragePool,
    params: OAuthSessionParams,
) -> Result<(), StorageError> {
    // Validate required input parameters
    if params.session_group.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Session group cannot be empty".into(),
        )));
    }

    if params.access_token.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Access token cannot be empty".into(),
        )));
    }

    if params.did.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "DID cannot be empty".into(),
        )));
    }

    if params.issuer.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Issuer cannot be empty".into(),
        )));
    }

    if params.refresh_token.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Refresh token cannot be empty".into(),
        )));
    }

    if params.secret_jwk_id.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Secret JWK ID cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    sqlx::query("INSERT INTO oauth_sessions (session_group, access_token, did, issuer, refresh_token, secret_jwk_id, dpop_jwk, created_at, access_token_expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
        .bind(&params.session_group)
        .bind(&params.access_token)
        .bind(&params.did)
        .bind(&params.issuer)
        .bind(&params.refresh_token)
        .bind(&params.secret_jwk_id)
        .bind(json!(params.dpop_jwk))
        .bind(params.created_at)
        .bind(params.access_token_expires_at)
        .execute(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)
}

pub async fn oauth_session_update(
    pool: &StoragePool,
    session_group: Cow<'_, str>,
    access_token: Cow<'_, str>,
    refresh_token: Cow<'_, str>,
    access_token_expires_at: DateTime<Utc>,
) -> Result<(), StorageError> {
    // Validate input parameters
    if session_group.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Session group cannot be empty".into(),
        )));
    }

    if access_token.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Access token cannot be empty".into(),
        )));
    }

    if refresh_token.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Refresh token cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    sqlx::query("UPDATE oauth_sessions SET access_token = $1, refresh_token = $2, access_token_expires_at = $3 WHERE session_group = $4")
        .bind(access_token)
        .bind(refresh_token)
        .bind(access_token_expires_at)
        .bind(session_group)
        .execute(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)
}

/// Delete an OAuth session by its session group.
pub async fn oauth_session_delete(
    pool: &StoragePool,
    session_group: &str,
) -> Result<(), StorageError> {
    // Validate session_group is not empty
    if session_group.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Session group cannot be empty".into(),
        )));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    sqlx::query("DELETE FROM oauth_sessions WHERE session_group = $1")
        .bind(session_group)
        .execute(tx.as_mut())
        .await
        .map_err(StorageError::UnableToExecuteQuery)?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)
}

/// Look up a web session by session group and optionally filter by DID.
pub async fn web_session_lookup(
    pool: &StoragePool,
    session_group: &str,
    did: Option<&str>,
) -> Result<(Handle, OAuthSession), StorageError> {
    // Validate session_group is not empty
    if session_group.trim().is_empty() {
        return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
            "Session group cannot be empty".into(),
        )));
    }

    // If did is provided, validate it's not empty
    if let Some(did_value) = did {
        if did_value.trim().is_empty() {
            return Err(StorageError::UnableToExecuteQuery(sqlx::Error::Protocol(
                "DID cannot be empty".into(),
            )));
        }
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(StorageError::CannotBeginDatabaseTransaction)?;

    let oauth_session = match did {
        Some(did_value) => {
            sqlx::query_as::<_, OAuthSession>(
                "SELECT * FROM oauth_sessions WHERE session_group = $1 AND did = $2 ORDER BY created_at DESC LIMIT 1",
            )
            .bind(session_group)
            .bind(did_value)
            .fetch_one(tx.as_mut())
            .await
        },
        None => {
            sqlx::query_as::<_, OAuthSession>(
                "SELECT * FROM oauth_sessions WHERE session_group = $1 ORDER BY created_at DESC LIMIT 1",
            )
            .bind(session_group)
            .fetch_one(tx.as_mut())
            .await
        }
    }
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => StorageError::WebSessionNotFound,
        other => StorageError::UnableToExecuteQuery(other),
    })?;

    let did_for_handle = did.unwrap_or(&oauth_session.did);

    let handle = sqlx::query_as::<_, Handle>("SELECT * FROM handles WHERE did = $1")
        .bind(did_for_handle)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StorageError::HandleNotFound,
            other => StorageError::UnableToExecuteQuery(other),
        })?;

    tx.commit()
        .await
        .map_err(StorageError::CannotCommitDatabaseTransaction)?;

    Ok((handle, oauth_session))
}

pub mod model {
    use anyhow::Error;
    use chrono::{DateTime, Utc};
    use p256::SecretKey;
    use serde::Deserialize;
    use sqlx::FromRow;

    use crate::{
        atproto::auth::SimpleOAuthSessionProvider, jose::jwk::WrappedJsonWebKey,
        storage::errors::OAuthModelError,
    };

    #[derive(Clone, FromRow, Deserialize)]
    pub struct OAuthRequest {
        pub oauth_state: String,
        pub issuer: String,
        pub did: String,
        pub nonce: String,
        pub pkce_verifier: String,
        pub secret_jwk_id: String,
        pub destination: Option<String>,
        pub dpop_jwk: sqlx::types::Json<WrappedJsonWebKey>,
        pub created_at: DateTime<Utc>,
        pub expires_at: DateTime<Utc>,
    }

    pub struct OAuthRequestState {
        pub state: String,
        pub nonce: String,
        pub code_challenge: String,
    }

    #[derive(Clone, FromRow, Deserialize)]
    pub struct OAuthSession {
        pub session_group: String,
        pub access_token: String,
        pub did: String,
        pub issuer: String,
        pub refresh_token: String,
        pub secret_jwk_id: String,
        pub dpop_jwk: sqlx::types::Json<WrappedJsonWebKey>,
        pub created_at: DateTime<Utc>,
        pub access_token_expires_at: DateTime<Utc>,
    }

    impl TryFrom<OAuthSession> for SimpleOAuthSessionProvider {
        type Error = Error;

        fn try_from(value: OAuthSession) -> Result<Self, Self::Error> {
            let dpop_secret = SecretKey::from_jwk(&value.dpop_jwk.jwk)
                .map_err(OAuthModelError::DpopSecretFromJwkFailed)?;

            Ok(SimpleOAuthSessionProvider {
                access_token: value.access_token,
                issuer: value.issuer,
                dpop_secret,
            })
        }
    }
}

#[cfg(test)]
pub mod test {
    use sqlx::PgPool;

    use crate::{
        jose,
        storage::oauth::{
            oauth_request_get, oauth_request_insert, oauth_request_remove, oauth_session_insert,
            web_session_lookup, OAuthRequestParams, OAuthSessionParams,
        },
    };

    #[sqlx::test(fixtures(path = "../../fixtures/storage", scripts("handles")))]
    async fn test_oauth_request(pool: PgPool) -> anyhow::Result<()> {
        let dpop_jwk = jose::jwk::generate();
        let created_at = chrono::Utc::now();
        let expires_at = created_at + chrono::Duration::seconds(60 as i64);

        let res = oauth_request_insert(
            &pool,
            OAuthRequestParams {
                oauth_state: "oauth_state".to_string().into(),
                issuer: "pds.examplepds.com".to_string().into(),
                did: "did:plc:d5c1ed6d01421a67b96f68fa".to_string().into(),
                nonce: "nonce".to_string().into(),
                pkce_verifier: "pkce_verifier".to_string().into(),
                secret_jwk_id: "secret_jwk_id".to_string().into(),
                dpop_jwk: Some(dpop_jwk.clone()),
                destination: None,
                created_at,
                expires_at,
            },
        )
        .await;

        assert!(!res.is_err());

        let oauth_request = oauth_request_get(&pool, "oauth_state").await;
        assert!(!oauth_request.is_err());
        let oauth_request = oauth_request.unwrap();

        assert_eq!(oauth_request.did, "did:plc:d5c1ed6d01421a67b96f68fa");
        assert_eq!(oauth_request.dpop_jwk.as_ref(), &dpop_jwk);

        let res = oauth_request_remove(&pool, "oauth_state").await;
        assert!(!res.is_err());

        {
            let oauth_request = oauth_request_get(&pool, "oauth_state").await;
            assert!(oauth_request.is_err());
        }

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures/storage", scripts("handles")))]
    async fn test_oauth_session(pool: PgPool) -> anyhow::Result<()> {
        let dpop_jwk = jose::jwk::generate();

        let session_group = ulid::Ulid::new().to_string();
        let now = chrono::Utc::now();

        let insert_session_res = oauth_session_insert(
            &pool,
            OAuthSessionParams {
                session_group: session_group.clone().into(),
                access_token: "access_token".to_string().into(),
                did: "did:plc:d5c1ed6d01421a67b96f68fa".to_string().into(),
                issuer: "pds.examplepds.com".to_string().into(),
                refresh_token: "refresh_token".to_string().into(),
                secret_jwk_id: "secret_jwk_id".to_string().into(),
                dpop_jwk: dpop_jwk.clone(),
                created_at: now,
                access_token_expires_at: now + chrono::Duration::seconds(60 as i64),
            },
        )
        .await;

        assert!(!insert_session_res.is_err());

        let web_session = web_session_lookup(
            &pool,
            &session_group,
            Some("did:plc:d5c1ed6d01421a67b96f68fa"),
        )
        .await;
        assert!(!web_session.is_err());

        Ok(())
    }
}

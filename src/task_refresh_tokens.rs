use anyhow::Result;
use chrono::{Duration, Utc};
use deadpool_redis::redis::{pipe, AsyncCommands};
use p256::SecretKey;
use std::borrow::Cow;
use tokio::time::{sleep, Instant};
use tokio_util::sync::CancellationToken;

use crate::{
    config::{OAuthActiveKeys, SigningKeys},
    oauth::client_oauth_refresh,
    refresh_tokens_errors::RefreshError,
    storage::{
        cache::{build_worker_queue, OAUTH_REFRESH_HEARTBEATS, OAUTH_REFRESH_QUEUE},
        oauth::{oauth_session_delete, oauth_session_update, web_session_lookup},
        CachePool, StoragePool,
    },
};

pub struct RefreshTokensTaskConfig {
    pub sleep_interval: Duration,
    pub worker_id: String,
    pub external_url_base: String,
    pub signing_keys: SigningKeys,
    pub oauth_active_keys: OAuthActiveKeys,
}

pub struct RefreshTokensTask {
    pub config: RefreshTokensTaskConfig,
    pub http_client: reqwest::Client,
    pub storage_pool: StoragePool,
    pub cache_pool: CachePool,
    pub cancellation_token: CancellationToken,
}

impl RefreshTokensTask {
    #[must_use]
    pub fn new(
        config: RefreshTokensTaskConfig,
        http_client: reqwest::Client,
        storage_pool: StoragePool,
        cache_pool: CachePool,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            config,
            http_client,
            storage_pool,
            cache_pool,
            cancellation_token,
        }
    }

    /// Runs the refresh tokens task as a long-running process
    ///
    /// # Errors
    /// Returns an error if the sleep interval cannot be converted, or if there's a problem
    /// processing the work items
    pub async fn run(&self) -> Result<()> {
        tracing::debug!("RefreshTokensTask started");

        let interval = self.config.sleep_interval.to_std()?;

        let sleeper = sleep(interval);
        tokio::pin!(sleeper);

        loop {
            tokio::select! {
            () = self.cancellation_token.cancelled() => {
                break;
            },
            () = &mut sleeper => {
                    if let Err(err) = self.process_work().await {
                        tracing::error!("RefreshTokensTask failed: {}", err);
                    }
                sleeper.as_mut().reset(Instant::now() + interval);
            }
            }
        }

        tracing::info!("RefreshTokensTask stopped");

        Ok(())
    }

    async fn process_work(&self) -> Result<i32> {
        let worker_queue = build_worker_queue(&self.config.worker_id);

        let mut conn = self.cache_pool.get().await?;

        let now = chrono::Utc::now();
        let epoch_millis = now.timestamp_millis();

        let _: () = conn
            .hset(
                OAUTH_REFRESH_HEARTBEATS,
                &self.config.worker_id,
                now.to_string(),
            )
            .await?;

        let global_queue_count: i32 = conn
            .zcount(OAUTH_REFRESH_QUEUE, 0, epoch_millis + 1)
            .await?;
        let worker_queue_count: i32 = conn.zcount(&worker_queue, 0, epoch_millis + 1).await?;

        tracing::trace!(
            global_queue_count = global_queue_count,
            worker_queue_count = worker_queue_count,
            "queue counts"
        );

        let mut process_work = worker_queue_count > 0;

        if global_queue_count > 0 && worker_queue_count == 0 {
            let (moved, new_count): (i64, i64) = pipe()
                .atomic()
                // Take some work from the global queue and put it in the worker queue
                // ZRANGESTORE dst src min max [BYSCORE | BYLEX] [REV] [LIMIT offset count]
                .cmd("ZRANGESTORE")
                .arg(&worker_queue)
                .arg(OAUTH_REFRESH_QUEUE)
                .arg(0)
                .arg(epoch_millis)
                .arg("BYSCORE")
                .arg("LIMIT")
                .arg(0)
                .arg(5)
                // Update the global queue to remove the items that were moved
                .cmd("ZDIFFSTORE")
                .arg(OAUTH_REFRESH_QUEUE)
                .arg(2)
                .arg(OAUTH_REFRESH_QUEUE)
                .arg(&worker_queue)
                .query_async(&mut conn)
                .await?;
            process_work = true;

            tracing::debug!(
                moved = moved,
                new_count = new_count,
                "moved work from global queue to worker queue"
            );
        }

        if !process_work {
            return Ok(0);
        }

        let count = 0;
        let results: Vec<(String, i64)> = conn
            .zrangebyscore_limit_withscores(&worker_queue, 0, epoch_millis, 0, 5)
            .await?;

        for (session_group, deadline) in results {
            tracing::info!(session_group, deadline, "processing work");
            let _: () = conn.zrem(&worker_queue, &session_group).await?;

            if let Err(err) = self
                .refresh_oauth_session(&mut conn, &session_group, deadline)
                .await
            {
                tracing::error!(session_group, deadline, err = ?err, "failed to refresh oauth session: {}", err);

                if let Err(err) = oauth_session_delete(&self.storage_pool, &session_group).await {
                    tracing::error!(session_group, err = ?err, "failed to delete oauth session: {}", err);
                }
            }
        }

        Ok(count)
    }

    async fn refresh_oauth_session(
        &self,
        conn: &mut deadpool_redis::Connection,
        session_group: &str,
        _deadline: i64,
    ) -> Result<()> {
        let (handle, oauth_session) =
            web_session_lookup(&self.storage_pool, session_group, None).await?;

        let secret_signing_key = self
            .config
            .signing_keys
            .as_ref()
            .get(&oauth_session.secret_jwk_id)
            .cloned();

        if secret_signing_key.is_none() {
            return Err(RefreshError::SecretSigningKeyNotFound.into());
        }

        let dpop_secret_key = SecretKey::from_jwk(&oauth_session.dpop_jwk.jwk)
            .map_err(RefreshError::DpopProofCreationFailed)?;

        let token_response = client_oauth_refresh(
            &self.http_client,
            &self.config.external_url_base,
            (&oauth_session.secret_jwk_id, secret_signing_key.unwrap()),
            oauth_session.refresh_token.as_str(),
            &handle,
            &dpop_secret_key,
        )
        .await?;

        let now = Utc::now();

        oauth_session_update(
            &self.storage_pool,
            Cow::Borrowed(session_group),
            Cow::Borrowed(&token_response.access_token),
            Cow::Borrowed(&token_response.refresh_token),
            now + chrono::Duration::seconds(i64::from(token_response.expires_in)),
        )
        .await?;

        let modified_expires_at = ((f64::from(token_response.expires_in)) * 0.8).round() as i64;
        let refresh_at = (now + chrono::Duration::seconds(modified_expires_at)).timestamp_millis();

        let _: () = conn
            .zadd(OAUTH_REFRESH_QUEUE, session_group, refresh_at)
            .await
            .map_err(RefreshError::PlaceInRefreshQueueFailed)?;

        Ok(())
    }
}

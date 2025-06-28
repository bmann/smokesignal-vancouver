use anyhow::Result;
use deadpool_redis::{Config, Pool, Runtime};

use crate::storage::errors::CacheError;

pub const OAUTH_REFRESH_QUEUE: &str = "auth_session:oauth:refresh";
pub const OAUTH_REFRESH_HEARTBEATS: &str = "auth_session:oauth:refresh:workers";

pub fn build_worker_queue(worker_id: &str) -> String {
    format!("{}:{}", OAUTH_REFRESH_QUEUE, worker_id)
}

pub fn create_cache_pool(redis_url: &str) -> Result<Pool> {
    let cfg = Config::from_url(redis_url);
    cfg.create_pool(Some(Runtime::Tokio1))
        .map_err(|err| CacheError::FailedToCreatePool(err).into())
}

// Mock implementation for testing
#[cfg(test)]
pub struct MockCachePool {}

#[cfg(test)]
impl std::fmt::Debug for MockCachePool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockCachePool").finish()
    }
}

#[cfg(test)]
impl Clone for MockCachePool {
    fn clone(&self) -> Self {
        Self {}
    }
}

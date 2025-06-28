use sqlx::{Pool, Postgres};

pub type StoragePool = Pool<Postgres>;

use deadpool_redis::Pool as DeadPool;
pub type CachePool = DeadPool;

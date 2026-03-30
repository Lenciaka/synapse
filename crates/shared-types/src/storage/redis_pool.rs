//! Redis connection pool for the Synapse system.
//!
//! Wraps `redis::aio::ConnectionManager` to provide a cheap-to-clone,
//! async-safe connection handle.  The URL is read from the `REDIS_URL`
//! environment variable at construction time.

use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use thiserror::Error;

/// Errors that can occur when using the Redis pool.
#[derive(Debug, Error)]
pub enum RedisError {
    /// The `REDIS_URL` environment variable is missing.
    #[error("REDIS_URL environment variable not set")]
    MissingUrl,

    /// An error returned by the underlying redis client.
    #[error("redis error: {0}")]
    Client(#[from] redis::RedisError),
}

/// A cheap-to-clone async Redis connection pool.
///
/// Internally backed by [`redis::aio::ConnectionManager`], which multiplexes
/// commands over a single managed connection and automatically reconnects on
/// failure.
#[derive(Clone)]
pub struct RedisPool {
    manager: ConnectionManager,
}

impl RedisPool {
    /// Creates a new [`RedisPool`] using the URL in the `REDIS_URL`
    /// environment variable.
    ///
    /// # Errors
    ///
    /// Returns [`RedisError::MissingUrl`] when `REDIS_URL` is unset, or a
    /// [`RedisError::Client`] variant when the connection cannot be
    /// established.
    pub async fn from_env() -> Result<Self, RedisError> {
        let url = std::env::var("REDIS_URL").map_err(|_| RedisError::MissingUrl)?;
        Self::connect(&url).await
    }

    /// Creates a new [`RedisPool`] connecting to the given `url`.
    ///
    /// # Errors
    ///
    /// Returns a [`RedisError::Client`] variant when the connection fails.
    pub async fn connect(url: &str) -> Result<Self, RedisError> {
        let client = redis::Client::open(url)?;
        let manager = ConnectionManager::new(client).await?;
        Ok(Self { manager })
    }

    /// Stores `value` under `key` in Redis.
    ///
    /// # Errors
    ///
    /// Returns a [`RedisError::Client`] variant on network or server errors.
    pub async fn set(&self, key: &str, value: &str) -> Result<(), RedisError> {
        let mut conn = self.manager.clone();
        conn.set::<_, _, ()>(key, value).await?;
        Ok(())
    }

    /// Retrieves the value stored under `key`, or `None` if the key does not
    /// exist.
    ///
    /// # Errors
    ///
    /// Returns a [`RedisError::Client`] variant on network or server errors.
    pub async fn get(&self, key: &str) -> Result<Option<String>, RedisError> {
        let mut conn = self.manager.clone();
        let value: Option<String> = conn.get(key).await?;
        Ok(value)
    }

    /// Deletes the value stored under `key`.
    ///
    /// # Errors
    ///
    /// Returns a [`RedisError::Client`] variant on network or server errors.
    pub async fn del(&self, key: &str) -> Result<(), RedisError> {
        let mut conn = self.manager.clone();
        conn.del::<_, ()>(key).await?;
        Ok(())
    }

    /// Returns all keys matching the given `pattern`.
    ///
    /// Uses the Redis `KEYS` command.  This is acceptable for small datasets
    /// (hundreds of keys); for production workloads with millions of keys
    /// consider switching to `SCAN`.
    ///
    /// # Errors
    ///
    /// Returns a [`RedisError::Client`] variant on network or server errors.
    pub async fn keys(&self, pattern: &str) -> Result<Vec<String>, RedisError> {
        let mut conn = self.manager.clone();
        let keys: Vec<String> = conn.keys(pattern).await?;
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test: requires a live Redis server at `REDIS_URL`.
    /// Run with `REDIS_URL=redis://127.0.0.1:6379 cargo test -- --ignored`
    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL"]
    async fn redis_set_get_roundtrip() {
        let url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let pool = RedisPool::connect(&url).await.expect("connect to Redis");

        let key = "synapse:test:roundtrip";
        let value = "hello-synapse";

        pool.set(key, value).await.expect("set key");
        let got = pool.get(key).await.expect("get key");
        assert_eq!(got.as_deref(), Some(value));

        pool.del(key).await.expect("del key");
        let gone = pool.get(key).await.expect("get after del");
        assert_eq!(gone, None);
    }

    /// Unit test: verifies that `from_env` fails cleanly when REDIS_URL is absent.
    #[tokio::test]
    async fn missing_env_var_returns_error() {
        // Temporarily remove the variable if set.
        let saved = std::env::var("REDIS_URL").ok();
        std::env::remove_var("REDIS_URL");

        let result = RedisPool::from_env().await;
        assert!(
            matches!(result, Err(RedisError::MissingUrl)),
            "expected MissingUrl error"
        );

        // Restore.
        if let Some(v) = saved {
            std::env::set_var("REDIS_URL", v);
        }
    }
}

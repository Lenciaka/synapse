//! Storage backends for the Synapse system.
//!
//! This module exposes two storage abstractions:
//!
//! * [`redis_pool`] — Redis connection pool via `redis-rs` + `ConnectionManager`.
//! * [`sqlite`]     — SQLite database pool with `sqlx` migrations.

pub mod redis_pool;
pub mod sqlite;

pub use redis_pool::{RedisError, RedisPool};
pub use sqlite::{AuditLogRow, SqliteDb, SqliteError};

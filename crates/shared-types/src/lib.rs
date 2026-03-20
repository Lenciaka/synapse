//! Shared types, traits, and abstractions for the Synapse multi-agent system.
//!
//! This crate defines the core domain model: [`Task`], [`CodingAgent`] trait,
//! [`AgentCapabilities`], and the [`AgentRegistry`].  It also provides
//! storage helpers ([`storage::RedisPool`] and [`storage::SqliteDb`]) that
//! other crates import to interact with Redis and SQLite.

pub mod agent;
pub mod storage;
pub mod task;

pub use agent::{AgentCapabilities, AgentRegistry, CodingAgent};
pub use storage::{AuditLogRow, RedisError, RedisPool, SqliteDb, SqliteError};
pub use task::{Task, TaskId, TaskStatus, TaskType};

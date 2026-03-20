//! Shared types, traits, and abstractions for the Synapse multi-agent system.
//!
//! This crate defines the core domain model: [`Task`], [`CodingAgent`] trait,
//! [`AgentCapabilities`], and the [`AgentRegistry`].  It also provides
//! storage helpers ([`storage::RedisPool`] and [`storage::SqliteDb`]) and a
//! NATS messaging client ([`nats::NatsClient`]) that other crates import to
//! interact with Redis, SQLite, and NATS.

pub mod agent;
pub mod nats;
pub mod storage;
pub mod task;

pub use agent::{AgentCapabilities, AgentError, AgentRegistry, CodingAgent};
pub use nats::{NatsClient, NatsError};
pub use storage::{AuditLogRow, RedisError, RedisPool, SqliteDb, SqliteError};
pub use task::{Task, TaskId, TaskStatus, TaskType};

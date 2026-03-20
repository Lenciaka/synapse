//! Shared types, traits, and abstractions for the Synapse multi-agent system.
//!
//! This crate defines the core domain model: [`Task`], [`CodingAgent`] trait,
//! [`AgentCapabilities`], and the [`AgentRegistry`].

pub mod agent;
pub mod task;

pub use agent::{AgentCapabilities, AgentRegistry, CodingAgent};
pub use task::{Task, TaskId, TaskStatus, TaskType};

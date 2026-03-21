//! MCP server library for the Synapse multi-agent system.
//!
//! Provides an axum HTTP server on a configurable port (default `:3000`) that
//! exposes MCP tool endpoints via the [`rmcp`] Streamable HTTP transport and a
//! simple health-check endpoint at `GET /health`.
//!
//! # Architecture
//!
//! ```text
//! Agents (MCP HTTP :3000) --> axum --> rmcp StreamableHttpService
//!                                  \-> GET /health
//! ```
//!
//! The server integrates with NATS and Redis via [`shared_types`] for
//! event publishing and state storage.  Tool implementations are added in
//! subsequent tasks (TASK-006, TASK-007, TASK-008).

pub mod handler;
pub mod server;
pub mod tools;

pub use server::{McpServerConfig, ServerError};

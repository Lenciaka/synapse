//! MCP tool implementations for the Synapse server.
//!
//! Each sub-module implements one or more MCP tools that agents invoke via the
//! MCP protocol.  Tools are registered on the [`SynapseMcpHandler`] via the
//! `#[tool_router]` macro.

pub mod tasks;

//! MCP tool implementations for the Synapse server.
//!
//! Each sub-module defines one or more tools that are registered with the
//! rmcp [`ToolRouter`] and dispatched via the MCP `tools/call` endpoint.

pub mod context;
pub mod tasks;

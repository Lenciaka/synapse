//! MCP server handler -- implements the [`rmcp::ServerHandler`] trait.
//!
//! This module defines [`SynapseMcpHandler`], the MCP request handler that
//! will dispatch incoming tool calls to Synapse-specific tools (context,
//! tasks, GitHub, etc.).  In this skeleton the handler returns server info
//! and empty tool lists; concrete tools are added in TASK-006 through
//! TASK-008.

use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::ServerHandler;

/// MCP request handler for the Synapse server.
///
/// Implements [`ServerHandler`] with sensible defaults.  Tools, prompts, and
/// resources are wired in via the rmcp router in later tasks.
#[derive(Debug, Clone, Default)]
pub struct SynapseMcpHandler;

impl SynapseMcpHandler {
    /// Creates a new handler instance.
    pub fn new() -> Self {
        Self
    }
}

impl ServerHandler for SynapseMcpHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Synapse MCP server -- multi-agent development orchestration.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_returns_server_info() {
        let handler = SynapseMcpHandler::new();
        let info = handler.get_info();
        assert!(info.instructions.is_some());
    }
}

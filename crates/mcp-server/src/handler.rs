//! MCP server handler -- implements the [`rmcp::ServerHandler`] trait.
//!
//! This module defines [`SynapseMcpHandler`], the MCP request handler that
//! dispatches incoming tool calls to Synapse-specific tools (context, tasks,
//! GitHub, etc.).  Context tools (`read_context`, `write_context`,
//! `search_memory`) are registered here; further tools are added in TASK-007
//! and TASK-008.

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::ServerHandler;
use shared_types::RedisPool;

use crate::tools::context::{ReadContext, SearchMemory, WriteContext};

/// MCP request handler for the Synapse server.
///
/// Implements [`ServerHandler`] with sensible defaults and carries shared
/// state (Redis pool) that tool implementations use.
#[derive(Debug, Clone, Default)]
pub struct SynapseMcpHandler {
    /// Optional Redis connection pool.  `None` when no Redis URL is configured
    /// (e.g. in pure unit tests that do not need storage).
    redis: Option<RedisPool>,
}

impl SynapseMcpHandler {
    /// Creates a new handler instance without a Redis connection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new handler backed by the given Redis pool.
    pub fn with_redis(pool: RedisPool) -> Self {
        Self { redis: Some(pool) }
    }

    /// Returns a reference to the Redis pool, if one was configured.
    pub fn redis(&self) -> Option<&RedisPool> {
        self.redis.as_ref()
    }

    /// Builds the [`ToolRouter`] containing all registered MCP tools.
    ///
    /// Each tool is registered via the rmcp `AsyncTool` trait so that the
    /// router can dispatch `tools/call` requests by name.
    pub fn tool_router() -> ToolRouter<Self> {
        ToolRouter::new()
            .with_async_tool::<ReadContext>()
            .with_async_tool::<WriteContext>()
            .with_async_tool::<SearchMemory>()
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

    #[test]
    fn handler_without_redis_returns_none() {
        let handler = SynapseMcpHandler::new();
        assert!(handler.redis().is_none());
    }

    #[test]
    fn tool_router_lists_three_tools() {
        let router = SynapseMcpHandler::tool_router();
        let tools = router.list_all();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&"read_context"), "missing read_context");
        assert!(names.contains(&"write_context"), "missing write_context");
        assert!(names.contains(&"search_memory"), "missing search_memory");
        assert_eq!(tools.len(), 3);
    }
}

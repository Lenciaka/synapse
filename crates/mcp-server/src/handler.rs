//! MCP server handler -- implements the [`rmcp::ServerHandler`] trait.
//!
//! This module defines [`SynapseMcpHandler`], the MCP request handler that
//! dispatches incoming tool calls to Synapse-specific tools (context, tasks,
//! GitHub, etc.).  Context tools (`read_context`, `write_context`,
//! `search_memory`) and task tools (`list_tasks`, `update_task`) are
//! registered here; further tools are added in TASK-008.

use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::ServerHandler;
use shared_types::{NatsClient, RedisPool};

use crate::tools::context::{ReadContext, SearchMemory, WriteContext};
use crate::tools::tasks::{ListTasks, TaskStore, UpdateTask};

/// MCP request handler for the Synapse server.
///
/// Implements [`ServerHandler`] with sensible defaults and carries shared
/// state (Redis pool, optional NATS client, task store) that tool
/// implementations use.
#[derive(Debug, Clone, Default)]
pub struct SynapseMcpHandler {
    /// Optional Redis connection pool.  `None` when no Redis URL is configured
    /// (e.g. in pure unit tests that do not need storage).
    redis: Option<RedisPool>,
    /// Optional task store backed by Redis (and optionally NATS for event
    /// publishing).  `None` when Redis is not configured.
    task_store: Option<TaskStore>,
}

impl SynapseMcpHandler {
    /// Creates a new handler instance without a Redis connection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new handler backed by the given Redis pool.
    ///
    /// The task store is initialised with no NATS client, so status-change
    /// events will not be published.
    pub fn with_redis(pool: RedisPool) -> Self {
        let task_store = TaskStore::new(pool.clone(), None);
        Self {
            redis: Some(pool),
            task_store: Some(task_store),
        }
    }

    /// Creates a new handler backed by Redis and NATS.
    ///
    /// Status-change events for tasks will be published to NATS.
    pub fn with_redis_and_nats(pool: RedisPool, nats: Arc<NatsClient>) -> Self {
        let task_store = TaskStore::new(pool.clone(), Some(nats));
        Self {
            redis: Some(pool),
            task_store: Some(task_store),
        }
    }

    /// Returns a reference to the Redis pool, if one was configured.
    pub fn redis(&self) -> Option<&RedisPool> {
        self.redis.as_ref()
    }

    /// Returns a reference to the [`TaskStore`], if one was configured.
    pub fn task_store(&self) -> Option<&TaskStore> {
        self.task_store.as_ref()
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
            .with_async_tool::<ListTasks>()
            .with_async_tool::<UpdateTask>()
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
    fn tool_router_lists_five_tools() {
        let router = SynapseMcpHandler::tool_router();
        let tools = router.list_all();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&"read_context"), "missing read_context");
        assert!(names.contains(&"write_context"), "missing write_context");
        assert!(names.contains(&"search_memory"), "missing search_memory");
        assert!(names.contains(&"list_tasks"), "missing list_tasks");
        assert!(names.contains(&"update_task"), "missing update_task");
        assert_eq!(tools.len(), 5);
    }
}

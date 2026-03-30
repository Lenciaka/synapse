//! MCP server handler -- implements the [`rmcp::ServerHandler`] trait.
//!
//! This module defines [`SynapseMcpHandler`], the MCP request handler that
//! dispatches incoming tool calls to Synapse-specific tools (tasks, context,
//! GitHub, etc.).

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use serde::{Deserialize, Serialize};

use crate::tools::tasks::TaskStore;
use shared_types::task::{Task, TaskStatus};

// ── Tool parameter / output types ────────────────────────────────────────────

/// Parameters for the `list_tasks` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct ListTasksParams {
    /// Optional status filter (e.g. `"pending"`, `"in_progress"`).
    pub status: Option<String>,
    /// Optional agent id filter.
    pub assigned_to: Option<String>,
}

/// Output of the `list_tasks` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListTasksOutput {
    /// The matching tasks.
    pub tasks: Vec<Task>,
}

/// Parameters for the `update_task` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateTaskParams {
    /// The task id to update.
    pub id: String,
    /// The new status (e.g. `"in_progress"`, `"done"`).
    pub status: String,
    /// Optional notes to attach to the task.
    pub notes: Option<String>,
}

/// Output of the `update_task` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct UpdateTaskOutput {
    /// The updated task.
    pub task: Task,
}

// ── Handler ──────────────────────────────────────────────────────────────────

/// MCP request handler for the Synapse server.
///
/// Implements [`ServerHandler`] with tool routing for task management.
/// Holds a [`ToolRouter`] that dispatches `list_tasks` and `update_task`
/// calls to the [`TaskStore`].
#[derive(Clone)]
pub struct SynapseMcpHandler {
    /// The task store used by task tools.
    task_store: TaskStore,
    /// The tool router that dispatches tool calls.
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl SynapseMcpHandler {
    /// Creates a new handler instance with the given task store.
    pub fn new(task_store: TaskStore) -> Self {
        let tool_router = Self::tool_router();
        Self {
            task_store,
            tool_router,
        }
    }
}

#[tool_router]
impl SynapseMcpHandler {
    /// List tasks with optional status and assigned_to filters.
    ///
    /// Queries the Redis task store and returns matching tasks as JSON.
    #[tool(
        name = "list_tasks",
        description = "List tasks with optional status and assigned_to filters"
    )]
    async fn list_tasks(
        &self,
        Parameters(params): Parameters<ListTasksParams>,
    ) -> Result<Json<ListTasksOutput>, String> {
        let status_filter: Option<TaskStatus> = match params.status {
            Some(ref s) => {
                let parsed: TaskStatus = serde_json::from_str(&format!("\"{s}\""))
                    .map_err(|e| format!("invalid status '{s}': {e}"))?;
                Some(parsed)
            }
            None => None,
        };

        let tasks = self
            .task_store
            .list_tasks(status_filter.as_ref(), params.assigned_to.as_deref())
            .await
            .map_err(|e| format!("failed to list tasks: {e}"))?;

        Ok(Json(ListTasksOutput { tasks }))
    }

    /// Update a task's status and optionally set notes.
    ///
    /// Validates the state machine transition and publishes a
    /// `synapse.task.status_changed` event to NATS.
    #[tool(
        name = "update_task",
        description = "Update a task's status (with state machine validation) and optionally set notes"
    )]
    async fn update_task(
        &self,
        Parameters(params): Parameters<UpdateTaskParams>,
    ) -> Result<Json<UpdateTaskOutput>, String> {
        let new_status: TaskStatus = serde_json::from_str(&format!("\"{}\"", params.status))
            .map_err(|e| format!("invalid status '{}': {e}", params.status))?;

        let task = self
            .task_store
            .update_task(&params.id, new_status, params.notes)
            .await
            .map_err(|e| format!("failed to update task: {e}"))?;

        Ok(Json(UpdateTaskOutput { task }))
    }
}

#[tool_handler]
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
    fn list_tasks_params_deserializes_with_defaults() {
        let params: ListTasksParams = serde_json::from_str("{}").unwrap();
        assert!(params.status.is_none());
        assert!(params.assigned_to.is_none());
    }

    #[test]
    fn list_tasks_params_deserializes_with_values() {
        let params: ListTasksParams =
            serde_json::from_str(r#"{"status": "pending", "assigned_to": "claude-code"}"#).unwrap();
        assert_eq!(params.status.as_deref(), Some("pending"));
        assert_eq!(params.assigned_to.as_deref(), Some("claude-code"));
    }

    #[test]
    fn update_task_params_deserializes() {
        let params: UpdateTaskParams =
            serde_json::from_str(r#"{"id": "t-1", "status": "in_progress"}"#).unwrap();
        assert_eq!(params.id, "t-1");
        assert_eq!(params.status, "in_progress");
        assert!(params.notes.is_none());
    }

    #[test]
    fn update_task_params_with_notes() {
        let params: UpdateTaskParams = serde_json::from_str(
            r#"{"id": "t-1", "status": "blocked", "notes": "waiting for review"}"#,
        )
        .unwrap();
        assert_eq!(params.notes.as_deref(), Some("waiting for review"));
    }
}

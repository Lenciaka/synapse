//! Gemini CLI agent adapter for the Synapse multi-agent system.
//!
//! This crate provides [`GeminiCliAgent`], which implements the
//! [`CodingAgent`](shared_types::CodingAgent) trait by shelling out to the
//! Gemini CLI binary and communicating with the MCP server over JSON-RPC.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use shared_types::agent::{AgentCapabilities, AgentError, CodingAgent};
use shared_types::task::{Task, TaskType};
use std::time::Duration;

/// Errors specific to the Gemini CLI agent.
#[derive(Debug, thiserror::Error)]
pub enum GeminiAgentError {
    /// An HTTP request to the MCP server failed.
    #[error("MCP request failed: {0}")]
    Mcp(#[from] reqwest::Error),
    /// The CLI subprocess exited with an error.
    #[error("CLI subprocess failed: {0}")]
    Subprocess(String),
    /// A required environment variable was missing.
    #[error("missing env var {name}: {source}")]
    MissingEnv {
        /// Name of the missing environment variable.
        name: &'static str,
        /// Underlying error from `std::env::var`.
        source: std::env::VarError,
    },
    /// Failed to parse an MCP JSON-RPC response.
    #[error("MCP response parse error: {0}")]
    ParseResponse(String),
}

/// JSON-RPC request body for MCP tool calls.
#[derive(Debug, Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    params: JsonRpcParams<'a>,
    id: u64,
}

/// Parameters for a JSON-RPC `tools/call` request.
#[derive(Debug, Serialize)]
struct JsonRpcParams<'a> {
    name: &'a str,
    arguments: serde_json::Value,
}

/// Top-level JSON-RPC response envelope.
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
    #[allow(dead_code)]
    id: Option<u64>,
}

/// Gemini CLI agent that implements the [`CodingAgent`] trait.
///
/// It communicates with the Synapse MCP server over HTTP JSON-RPC to poll for
/// tasks, and executes work by invoking the Gemini CLI binary as a subprocess.
///
/// # Configuration
///
/// | Env var           | Default                   | Description                      |
/// |-------------------|---------------------------|----------------------------------|
/// | `MCP_URL`         | `http://localhost:3000`   | Base URL of the MCP server       |
/// | `AGENT_ID`        | `gemini-cli`              | Unique agent identifier          |
/// | `GEMINI_CLI_BIN`  | `gemini`                  | Path to the Gemini CLI binary    |
/// | `POLL_INTERVAL`   | `5`                       | Seconds between poll cycles      |
pub struct GeminiCliAgent {
    mcp_url: String,
    agent_id: String,
    gemini_bin: String,
    capabilities: AgentCapabilities,
    client: reqwest::Client,
}

impl GeminiCliAgent {
    /// Creates a new [`GeminiCliAgent`] reading configuration from environment
    /// variables with sensible defaults.
    pub fn from_env() -> Result<Self, GeminiAgentError> {
        let mcp_url =
            std::env::var("MCP_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let agent_id = std::env::var("AGENT_ID").unwrap_or_else(|_| "gemini-cli".to_string());
        let gemini_bin = std::env::var("GEMINI_CLI_BIN").unwrap_or_else(|_| "gemini".to_string());

        Ok(Self::new(mcp_url, agent_id, gemini_bin))
    }

    /// Creates a new [`GeminiCliAgent`] with explicit configuration values.
    pub fn new(mcp_url: String, agent_id: String, gemini_bin: String) -> Self {
        let capabilities = AgentCapabilities {
            agent_id: agent_id.clone(),
            supported_task_types: vec![TaskType::Code, TaskType::Review],
        };
        Self {
            mcp_url,
            agent_id,
            gemini_bin,
            capabilities,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Sends a JSON-RPC `tools/call` request to the MCP server.
    async fn mcp_call(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, GeminiAgentError> {
        let body = JsonRpcRequest {
            jsonrpc: "2.0",
            method: "tools/call",
            params: JsonRpcParams {
                name: tool_name,
                arguments,
            },
            id: 1,
        };

        let url = format!("{}/mcp", self.mcp_url);
        let resp: JsonRpcResponse = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        if let Some(err) = resp.error {
            return Err(GeminiAgentError::ParseResponse(format!(
                "JSON-RPC error: {}",
                err
            )));
        }

        resp.result
            .ok_or_else(|| GeminiAgentError::ParseResponse("missing result field".to_string()))
    }

    /// Lists pending tasks assigned to this agent by calling the MCP
    /// `list_tasks` tool.
    pub async fn list_pending_tasks(&self) -> Result<Vec<Task>, GeminiAgentError> {
        let args = serde_json::json!({
            "status": "pending",
            "assigned_to": self.agent_id,
        });

        let result = self.mcp_call("list_tasks", args).await?;

        let tasks: Vec<Task> = serde_json::from_value(result).map_err(|e| {
            GeminiAgentError::ParseResponse(format!("failed to parse task list: {}", e))
        })?;

        Ok(tasks)
    }

    /// Updates the status of a task via the MCP `update_task` tool.
    pub async fn update_task_status(
        &self,
        task_id: &str,
        status: &str,
        notes: Option<&str>,
    ) -> Result<(), GeminiAgentError> {
        let mut args = serde_json::json!({
            "id": task_id,
            "status": status,
        });

        if let Some(n) = notes {
            args.as_object_mut()
                .ok_or_else(|| {
                    GeminiAgentError::ParseResponse("failed to build arguments".to_string())
                })?
                .insert(
                    "notes".to_string(),
                    serde_json::Value::String(n.to_string()),
                );
        }

        let _result = self.mcp_call("update_task", args).await?;
        Ok(())
    }

    /// Executes the Gemini CLI binary with the given task description and
    /// returns the captured stdout output.
    async fn run_gemini_cli(&self, task: &Task) -> Result<String, GeminiAgentError> {
        let prompt = format!("Task: {}\nDescription: {}", task.title, task.description);

        let output = tokio::process::Command::new(&self.gemini_bin)
            .arg(&prompt)
            .output()
            .await
            .map_err(|e| GeminiAgentError::Subprocess(format!("failed to spawn: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GeminiAgentError::Subprocess(format!(
                "exit code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }

    /// Returns the configured poll interval from the `POLL_INTERVAL` env var,
    /// defaulting to 5 seconds.
    pub fn poll_interval() -> Duration {
        let secs: u64 = std::env::var("POLL_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        Duration::from_secs(secs)
    }
}

#[async_trait]
impl CodingAgent for GeminiCliAgent {
    fn id(&self) -> &str {
        &self.agent_id
    }

    fn capabilities(&self) -> &AgentCapabilities {
        &self.capabilities
    }

    async fn is_available(&self) -> bool {
        let args = serde_json::json!({
            "assigned_to": self.agent_id,
        });
        self.mcp_call("list_tasks", args).await.is_ok()
    }

    async fn execute(&self, task: &Task) -> Result<(), AgentError> {
        self.run_gemini_cli(task)
            .await
            .map_err(|e| AgentError::ExecutionFailed {
                agent_id: self.agent_id.clone(),
                reason: e.to_string(),
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_from_env_defaults() {
        // Clear env vars to test defaults
        std::env::remove_var("MCP_URL");
        std::env::remove_var("AGENT_ID");
        std::env::remove_var("GEMINI_CLI_BIN");

        let agent = GeminiCliAgent::from_env().unwrap();
        assert_eq!(agent.mcp_url, "http://localhost:3000");
        assert_eq!(agent.agent_id, "gemini-cli");
        assert_eq!(agent.gemini_bin, "gemini");
    }

    #[test]
    fn agent_id_returns_configured_id() {
        let agent = GeminiCliAgent::new(
            "http://test:3000".to_string(),
            "test-gemini".to_string(),
            "/usr/bin/gemini".to_string(),
        );
        assert_eq!(agent.id(), "test-gemini");
    }

    #[test]
    fn capabilities_include_code_and_review() {
        let agent = GeminiCliAgent::new(
            "http://test:3000".to_string(),
            "gemini-cli".to_string(),
            "gemini".to_string(),
        );
        let caps = agent.capabilities();
        assert_eq!(caps.agent_id, "gemini-cli");
        assert!(caps.supported_task_types.contains(&TaskType::Code));
        assert!(caps.supported_task_types.contains(&TaskType::Review));
    }

    #[test]
    fn new_builds_with_custom_values() {
        let agent = GeminiCliAgent::new(
            "http://custom:9000".to_string(),
            "custom-agent".to_string(),
            "/opt/gemini".to_string(),
        );
        assert_eq!(agent.mcp_url, "http://custom:9000");
        assert_eq!(agent.agent_id, "custom-agent");
        assert_eq!(agent.gemini_bin, "/opt/gemini");
    }

    #[test]
    fn poll_interval_default() {
        std::env::remove_var("POLL_INTERVAL");
        let interval = GeminiCliAgent::poll_interval();
        assert_eq!(interval, Duration::from_secs(5));
    }

    #[test]
    fn poll_interval_from_env() {
        std::env::set_var("POLL_INTERVAL", "10");
        let interval = GeminiCliAgent::poll_interval();
        assert_eq!(interval, Duration::from_secs(10));
        std::env::remove_var("POLL_INTERVAL");
    }

    #[tokio::test]
    async fn execute_returns_error_for_missing_binary() {
        let agent = GeminiCliAgent::new(
            "http://localhost:3000".to_string(),
            "gemini-cli".to_string(),
            "/nonexistent/gemini-binary-that-does-not-exist".to_string(),
        );
        let task = Task {
            id: "t-1".to_string(),
            title: "test".to_string(),
            description: "test task".to_string(),
            status: shared_types::TaskStatus::Pending,
            task_type: TaskType::Code,
            assigned_to: Some("gemini-cli".to_string()),
            notes: None,
        };
        let result = agent.execute(&task).await;
        assert!(result.is_err());
    }
}

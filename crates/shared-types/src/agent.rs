//! Agent trait, capabilities, and registry for the Synapse system.

use std::sync::Arc;

use crate::task::{Task, TaskType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when working with agents.
#[derive(Debug, Error)]
pub enum AgentError {
    /// No agent was available to handle the task.
    #[error("no available agent for task type {task_type:?}")]
    NoAvailableAgent { task_type: TaskType },
    /// An agent-specific execution error occurred.
    #[error("agent {agent_id} execution failed: {reason}")]
    ExecutionFailed { agent_id: String, reason: String },
}

/// Describes what kinds of tasks an agent can handle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Human-readable agent identifier (e.g. `"claude-code"`).
    pub agent_id: String,
    /// Task types this agent can handle.
    pub supported_task_types: Vec<TaskType>,
}

/// Core trait that every Synapse coding agent must implement.
#[async_trait]
pub trait CodingAgent: Send + Sync {
    /// Returns the unique identifier for this agent.
    fn id(&self) -> &str;

    /// Returns this agent's capabilities.
    fn capabilities(&self) -> &AgentCapabilities;

    /// Returns `true` if this agent is currently reachable and able to accept work.
    async fn is_available(&self) -> bool;

    /// Executes a task to completion.
    ///
    /// Returns `Ok(())` on success or an [`AgentError`] on failure.
    async fn execute(&self, task: &Task) -> Result<(), AgentError>;
}

/// Registry of all known agents; selects the best available agent for a task
/// using a preference-ordered fallback list.
#[derive(Default)]
pub struct AgentRegistry {
    agents: Vec<Arc<dyn CodingAgent>>,
}

impl AgentRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an agent with the registry.
    pub fn register(&mut self, agent: Arc<dyn CodingAgent>) {
        self.agents.push(agent);
    }

    /// Selects the first available agent from the `prefer` list that supports
    /// the given task type. Agents are checked in the order they appear in
    /// `prefer`; any agent not in `prefer` is ignored. Returns `None` when no
    /// suitable agent is available.
    pub async fn select(&self, task: &Task, prefer: &[&str]) -> Option<Arc<dyn CodingAgent>> {
        for preferred_id in prefer {
            for agent in &self.agents {
                if agent.id() == *preferred_id
                    && agent
                        .capabilities()
                        .supported_task_types
                        .contains(&task.task_type)
                    && agent.is_available().await
                {
                    return Some(Arc::clone(agent));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{TaskId, TaskStatus};

    struct StubAgent {
        caps: AgentCapabilities,
        available: bool,
    }

    #[async_trait]
    impl CodingAgent for StubAgent {
        fn id(&self) -> &str {
            &self.caps.agent_id
        }
        fn capabilities(&self) -> &AgentCapabilities {
            &self.caps
        }
        async fn is_available(&self) -> bool {
            self.available
        }
        async fn execute(&self, _task: &Task) -> Result<(), AgentError> {
            Ok(())
        }
    }

    fn make_task(task_type: TaskType) -> Task {
        Task {
            id: TaskId::from("t-1"),
            title: "stub".to_string(),
            description: String::new(),
            status: TaskStatus::Pending,
            task_type,
            assigned_to: None,
            notes: None,
        }
    }

    #[tokio::test]
    async fn registry_selects_available_agent() {
        let mut reg = AgentRegistry::new();
        reg.register(Arc::new(StubAgent {
            caps: AgentCapabilities {
                agent_id: "claude-code".to_string(),
                supported_task_types: vec![TaskType::Code],
            },
            available: true,
        }));
        let task = make_task(TaskType::Code);
        let prefer = &["claude-code"];
        let agent = reg.select(&task, prefer).await;
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().id(), "claude-code");
    }

    #[tokio::test]
    async fn registry_skips_unavailable_and_falls_back() {
        let mut reg = AgentRegistry::new();
        // First agent: supports Code but offline
        reg.register(Arc::new(StubAgent {
            caps: AgentCapabilities {
                agent_id: "claude-code".to_string(),
                supported_task_types: vec![TaskType::Code],
            },
            available: false,
        }));
        // Second agent: supports Code and online
        reg.register(Arc::new(StubAgent {
            caps: AgentCapabilities {
                agent_id: "codex".to_string(),
                supported_task_types: vec![TaskType::Code],
            },
            available: true,
        }));
        let task = make_task(TaskType::Code);
        let prefer = &["claude-code", "codex"];
        let agent = reg.select(&task, prefer).await;
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().id(), "codex");
    }

    #[tokio::test]
    async fn registry_returns_none_when_no_agent_available() {
        let reg = AgentRegistry::new();
        let task = make_task(TaskType::Code);
        let prefer = &["claude-code"];
        let result = reg.select(&task, prefer).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn registry_respects_prefer_order() {
        let mut reg = AgentRegistry::new();
        reg.register(Arc::new(StubAgent {
            caps: AgentCapabilities {
                agent_id: "codex".to_string(),
                supported_task_types: vec![TaskType::Code],
            },
            available: true,
        }));
        reg.register(Arc::new(StubAgent {
            caps: AgentCapabilities {
                agent_id: "claude-code".to_string(),
                supported_task_types: vec![TaskType::Code],
            },
            available: true,
        }));
        let task = make_task(TaskType::Code);
        // Even though codex was registered first, prefer list puts claude-code first
        let prefer = &["claude-code", "codex"];
        let agent = reg.select(&task, prefer).await;
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().id(), "claude-code");
    }

    #[tokio::test]
    async fn registry_ignores_agents_not_in_prefer_list() {
        let mut reg = AgentRegistry::new();
        reg.register(Arc::new(StubAgent {
            caps: AgentCapabilities {
                agent_id: "codex".to_string(),
                supported_task_types: vec![TaskType::Code],
            },
            available: true,
        }));
        let task = make_task(TaskType::Code);
        // prefer list does not include "codex"
        let prefer = &["claude-code"];
        let result = reg.select(&task, prefer).await;
        assert!(result.is_none());
    }
}

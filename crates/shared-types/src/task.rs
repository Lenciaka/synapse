//! Task domain types for the Synapse system.

use serde::{Deserialize, Serialize};

/// Unique identifier for a task.
pub type TaskId = String;

/// The current lifecycle state of a [`Task`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Waiting to be picked up by an agent.
    Pending,
    /// An agent is actively working on this task.
    InProgress,
    /// The task output is awaiting review.
    InReview,
    /// The task has been completed successfully.
    Done,
    /// The task cannot proceed and needs human intervention.
    Blocked,
}

/// The category of work a task represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// A coding / implementation task.
    Code,
    /// A code review task.
    Review,
    /// A security patch task.
    SecurityPatch,
}

/// A unit of work assigned to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier.
    pub id: TaskId,
    /// Human-readable title.
    pub title: String,
    /// Detailed description of the work to be done.
    pub description: String,
    /// Current lifecycle state.
    pub status: TaskStatus,
    /// Category of work.
    pub task_type: TaskType,
    /// Agent ID this task is assigned to, if any.
    pub assigned_to: Option<String>,
    /// Optional human-readable notes (used for blockers, review comments, etc.).
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_serialization_roundtrip() {
        let task = Task {
            id: "t-1".to_string(),
            title: "Test task".to_string(),
            description: "Do something".to_string(),
            status: TaskStatus::Pending,
            task_type: TaskType::Code,
            assigned_to: Some("claude-code".to_string()),
            notes: None,
        };
        let json = serde_json::to_string(&task).unwrap();
        let decoded: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(task.id, decoded.id);
        assert_eq!(task.status, decoded.status);
    }

    #[test]
    fn task_status_serde_snake_case() {
        assert_eq!(
            serde_json::to_string(&TaskStatus::InProgress).unwrap(),
            "\"in_progress\""
        );
        assert_eq!(
            serde_json::from_str::<TaskStatus>("\"in_review\"").unwrap(),
            TaskStatus::InReview
        );
    }
}

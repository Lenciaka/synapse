//! Task domain types for the Synapse system.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Unique identifier for a task.
pub type TaskId = String;

/// The current lifecycle state of a [`Task`].
///
/// Valid transitions:
/// - `Pending` -> `InProgress`, `Blocked`
/// - `InProgress` -> `InReview`, `Done`, `Blocked`
/// - `InReview` -> `Done`, `Blocked`, `InProgress`
/// - `Blocked` -> `Pending`, `InProgress`
/// - `Done` is terminal
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

impl TaskStatus {
    /// Returns `true` if transitioning from `self` to `target` is a valid
    /// state machine transition.
    ///
    /// The task state machine allows:
    /// - `Pending` -> `InProgress` | `Blocked`
    /// - `InProgress` -> `InReview` | `Done` | `Blocked`
    /// - `InReview` -> `Done` | `Blocked` | `InProgress`
    /// - `Blocked` -> `Pending` | `InProgress`
    /// - `Done` is a terminal state (no outbound transitions)
    pub fn can_transition_to(&self, target: &TaskStatus) -> bool {
        matches!(
            (self, target),
            (TaskStatus::Pending, TaskStatus::InProgress)
                | (TaskStatus::Pending, TaskStatus::Blocked)
                | (TaskStatus::InProgress, TaskStatus::InReview)
                | (TaskStatus::InProgress, TaskStatus::Done)
                | (TaskStatus::InProgress, TaskStatus::Blocked)
                | (TaskStatus::InReview, TaskStatus::Done)
                | (TaskStatus::InReview, TaskStatus::Blocked)
                | (TaskStatus::InReview, TaskStatus::InProgress)
                | (TaskStatus::Blocked, TaskStatus::Pending)
                | (TaskStatus::Blocked, TaskStatus::InProgress)
        )
    }
}

/// The category of work a task represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
    /// Unix timestamp (seconds) when the task was created.
    pub created_at: i64,
    /// Unix timestamp (seconds) when the task was last updated.
    pub updated_at: i64,
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
            created_at: 1_700_000_000,
            updated_at: 1_700_000_000,
        };
        let json = serde_json::to_string(&task).unwrap();
        let decoded: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(task.id, decoded.id);
        assert_eq!(task.status, decoded.status);
        assert_eq!(decoded.created_at, 1_700_000_000);
        assert_eq!(decoded.updated_at, 1_700_000_000);
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

    #[test]
    fn valid_transitions() {
        // Pending -> InProgress, Blocked
        assert!(TaskStatus::Pending.can_transition_to(&TaskStatus::InProgress));
        assert!(TaskStatus::Pending.can_transition_to(&TaskStatus::Blocked));

        // InProgress -> InReview, Done, Blocked
        assert!(TaskStatus::InProgress.can_transition_to(&TaskStatus::InReview));
        assert!(TaskStatus::InProgress.can_transition_to(&TaskStatus::Done));
        assert!(TaskStatus::InProgress.can_transition_to(&TaskStatus::Blocked));

        // InReview -> Done, Blocked, InProgress
        assert!(TaskStatus::InReview.can_transition_to(&TaskStatus::Done));
        assert!(TaskStatus::InReview.can_transition_to(&TaskStatus::Blocked));
        assert!(TaskStatus::InReview.can_transition_to(&TaskStatus::InProgress));

        // Blocked -> Pending, InProgress
        assert!(TaskStatus::Blocked.can_transition_to(&TaskStatus::Pending));
        assert!(TaskStatus::Blocked.can_transition_to(&TaskStatus::InProgress));
    }

    #[test]
    fn invalid_transitions() {
        // Done is terminal
        assert!(!TaskStatus::Done.can_transition_to(&TaskStatus::Pending));
        assert!(!TaskStatus::Done.can_transition_to(&TaskStatus::InProgress));
        assert!(!TaskStatus::Done.can_transition_to(&TaskStatus::InReview));
        assert!(!TaskStatus::Done.can_transition_to(&TaskStatus::Blocked));

        // Self-transitions are not allowed
        assert!(!TaskStatus::Pending.can_transition_to(&TaskStatus::Pending));
        assert!(!TaskStatus::InProgress.can_transition_to(&TaskStatus::InProgress));

        // Skip transitions
        assert!(!TaskStatus::Pending.can_transition_to(&TaskStatus::Done));
        assert!(!TaskStatus::Pending.can_transition_to(&TaskStatus::InReview));
    }
}

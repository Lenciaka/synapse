//! Task domain types for the Synapse system.

use serde::{Deserialize, Serialize};

/// Unique identifier for a task.
pub type TaskId = String;

/// The current lifecycle state of a [`Task`].
///
/// State machine transitions:
///
/// ```text
/// Pending --> InProgress --> InReview --> Done
///    |            |             |
///    v            v             v
///  Blocked <-- Blocked <-- Blocked
///    |
///    +--> Pending | InProgress
/// ```
///
/// `Done` is a terminal state -- no transitions out of it are allowed.
/// Self-transitions (e.g. `Pending -> Pending`) are also rejected.
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

impl TaskStatus {
    /// Returns `true` if transitioning from `self` to `target` is a valid
    /// state machine transition.
    ///
    /// The allowed transitions are:
    /// - `Pending` -> `InProgress` | `Blocked`
    /// - `InProgress` -> `InReview` | `Done` | `Blocked`
    /// - `InReview` -> `Done` | `InProgress` | `Blocked`
    /// - `Blocked` -> `Pending` | `InProgress`
    /// - `Done` -> (none -- terminal)
    ///
    /// Self-transitions are never allowed.
    pub fn can_transition_to(&self, target: &TaskStatus) -> bool {
        if self == target {
            return false;
        }
        matches!(
            (self, target),
            (TaskStatus::Pending, TaskStatus::InProgress)
                | (TaskStatus::Pending, TaskStatus::Blocked)
                | (TaskStatus::InProgress, TaskStatus::InReview)
                | (TaskStatus::InProgress, TaskStatus::Done)
                | (TaskStatus::InProgress, TaskStatus::Blocked)
                | (TaskStatus::InReview, TaskStatus::Done)
                | (TaskStatus::InReview, TaskStatus::InProgress)
                | (TaskStatus::InReview, TaskStatus::Blocked)
                | (TaskStatus::Blocked, TaskStatus::Pending)
                | (TaskStatus::Blocked, TaskStatus::InProgress)
        )
    }
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

    // -- State machine tests --

    #[test]
    fn pending_can_go_to_in_progress() {
        assert!(TaskStatus::Pending.can_transition_to(&TaskStatus::InProgress));
    }

    #[test]
    fn pending_can_go_to_blocked() {
        assert!(TaskStatus::Pending.can_transition_to(&TaskStatus::Blocked));
    }

    #[test]
    fn pending_cannot_skip_to_done() {
        assert!(!TaskStatus::Pending.can_transition_to(&TaskStatus::Done));
    }

    #[test]
    fn pending_cannot_skip_to_in_review() {
        assert!(!TaskStatus::Pending.can_transition_to(&TaskStatus::InReview));
    }

    #[test]
    fn in_progress_can_go_to_in_review() {
        assert!(TaskStatus::InProgress.can_transition_to(&TaskStatus::InReview));
    }

    #[test]
    fn in_progress_can_go_to_done() {
        assert!(TaskStatus::InProgress.can_transition_to(&TaskStatus::Done));
    }

    #[test]
    fn in_progress_can_go_to_blocked() {
        assert!(TaskStatus::InProgress.can_transition_to(&TaskStatus::Blocked));
    }

    #[test]
    fn in_review_can_go_to_done() {
        assert!(TaskStatus::InReview.can_transition_to(&TaskStatus::Done));
    }

    #[test]
    fn in_review_can_go_to_blocked() {
        assert!(TaskStatus::InReview.can_transition_to(&TaskStatus::Blocked));
    }

    #[test]
    fn in_review_can_go_back_to_in_progress() {
        assert!(TaskStatus::InReview.can_transition_to(&TaskStatus::InProgress));
    }

    #[test]
    fn blocked_can_go_to_pending() {
        assert!(TaskStatus::Blocked.can_transition_to(&TaskStatus::Pending));
    }

    #[test]
    fn blocked_can_go_to_in_progress() {
        assert!(TaskStatus::Blocked.can_transition_to(&TaskStatus::InProgress));
    }

    #[test]
    fn done_is_terminal() {
        assert!(!TaskStatus::Done.can_transition_to(&TaskStatus::Pending));
        assert!(!TaskStatus::Done.can_transition_to(&TaskStatus::InProgress));
        assert!(!TaskStatus::Done.can_transition_to(&TaskStatus::InReview));
        assert!(!TaskStatus::Done.can_transition_to(&TaskStatus::Blocked));
        assert!(!TaskStatus::Done.can_transition_to(&TaskStatus::Done));
    }

    #[test]
    fn self_transitions_not_allowed() {
        assert!(!TaskStatus::Pending.can_transition_to(&TaskStatus::Pending));
        assert!(!TaskStatus::InProgress.can_transition_to(&TaskStatus::InProgress));
        assert!(!TaskStatus::InReview.can_transition_to(&TaskStatus::InReview));
        assert!(!TaskStatus::Blocked.can_transition_to(&TaskStatus::Blocked));
    }
}

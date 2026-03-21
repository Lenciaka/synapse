//! MCP tools for task management: `list_tasks` and `update_task`.
//!
//! Tasks are stored in Redis as JSON under the key prefix `synapse:task:<id>`.
//! On status changes, the `synapse.task.status_changed` NATS subject is
//! published to so that downstream consumers (gRPC server, TUI) can react
//! in real time.

use std::sync::Arc;

use shared_types::nats::{subjects, NatsClient};
use shared_types::storage::RedisPool;
use shared_types::task::{Task, TaskStatus};
use thiserror::Error;

/// Redis key prefix for all tasks.
const TASK_KEY_PREFIX: &str = "synapse:task:";

/// Errors that can occur when working with task tools.
#[derive(Debug, Error)]
pub enum TaskToolError {
    /// The requested task was not found in the store.
    #[error("task not found: {0}")]
    NotFound(String),

    /// An invalid state transition was attempted.
    #[error("invalid state transition from {from:?} to {to:?}")]
    InvalidTransition {
        /// The current status of the task.
        from: TaskStatus,
        /// The requested target status.
        to: TaskStatus,
    },

    /// Failed to deserialize a task from Redis.
    #[error("failed to deserialize task: {0}")]
    Deserialize(#[from] serde_json::Error),

    /// A Redis operation failed.
    #[error("redis error: {0}")]
    Redis(#[from] shared_types::storage::RedisError),

    /// A NATS publish operation failed.
    #[error("nats error: {0}")]
    Nats(#[from] shared_types::nats::NatsError),
}

/// Shared state for task tools, holding Redis and NATS connections.
#[derive(Clone)]
pub struct TaskStore {
    redis: RedisPool,
    nats: Option<Arc<NatsClient>>,
}

impl TaskStore {
    /// Creates a new [`TaskStore`] with the given Redis pool and optional NATS
    /// client.
    ///
    /// When `nats` is `None`, status-change events will not be published.  This
    /// is useful for unit tests that do not need a live NATS connection.
    pub fn new(redis: RedisPool, nats: Option<Arc<NatsClient>>) -> Self {
        Self { redis, nats }
    }

    /// Returns the Redis key for a given task id.
    fn task_key(id: &str) -> String {
        format!("{TASK_KEY_PREFIX}{id}")
    }

    /// Stores a task in Redis.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or the Redis write fails.
    pub async fn save_task(&self, task: &Task) -> Result<(), TaskToolError> {
        let key = Self::task_key(&task.id);
        let json = serde_json::to_string(task)?;
        self.redis.set(&key, &json).await?;
        Ok(())
    }

    /// Retrieves a single task by id from Redis.
    ///
    /// # Errors
    ///
    /// Returns [`TaskToolError::NotFound`] if the key does not exist, or a
    /// deserialization / Redis error otherwise.
    pub async fn get_task(&self, id: &str) -> Result<Task, TaskToolError> {
        let key = Self::task_key(id);
        let json = self
            .redis
            .get(&key)
            .await?
            .ok_or_else(|| TaskToolError::NotFound(id.to_string()))?;
        let task: Task = serde_json::from_str(&json)?;
        Ok(task)
    }

    /// Lists tasks, optionally filtering by status and/or assigned agent.
    ///
    /// Scans all keys under the `synapse:task:*` prefix, deserializes each
    /// task, and applies the provided filters.
    ///
    /// # Errors
    ///
    /// Returns a Redis or deserialization error if any task record is
    /// malformed.
    pub async fn list_tasks(
        &self,
        status: Option<&TaskStatus>,
        assigned_to: Option<&str>,
    ) -> Result<Vec<Task>, TaskToolError> {
        let pattern = format!("{TASK_KEY_PREFIX}*");
        let keys = self.redis.keys(&pattern).await?;

        let mut tasks = Vec::new();
        for key in keys {
            if let Some(json) = self.redis.get(&key).await? {
                let task: Task = serde_json::from_str(&json)?;

                let status_match = status.is_none_or(|s| &task.status == s);
                let agent_match =
                    assigned_to.is_none_or(|a| task.assigned_to.as_deref() == Some(a));

                if status_match && agent_match {
                    tasks.push(task);
                }
            }
        }

        Ok(tasks)
    }

    /// Updates a task's status (and optionally its notes) after validating the
    /// state transition.  On success, publishes a
    /// `synapse.task.status_changed` event to NATS (when a NATS client is
    /// available).
    ///
    /// # Errors
    ///
    /// Returns [`TaskToolError::NotFound`] if the task does not exist,
    /// [`TaskToolError::InvalidTransition`] if the requested transition
    /// violates the state machine, or a Redis/NATS error on I/O failures.
    pub async fn update_task(
        &self,
        id: &str,
        new_status: TaskStatus,
        notes: Option<String>,
    ) -> Result<Task, TaskToolError> {
        let mut task = self.get_task(id).await?;

        if !task.status.can_transition_to(&new_status) {
            return Err(TaskToolError::InvalidTransition {
                from: task.status,
                to: new_status,
            });
        }

        task.status = new_status;
        if let Some(n) = notes {
            task.notes = Some(n);
        }
        task.updated_at = now_unix();

        self.save_task(&task).await?;

        // Publish status-changed event to NATS.
        if let Some(ref nats) = self.nats {
            let payload = serde_json::to_vec(&task)?;
            nats.publish(subjects::TASK_STATUS_CHANGED, payload).await?;
        }

        Ok(task)
    }
}

/// Returns the current Unix timestamp in seconds.
fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::task::TaskType;

    /// Creates a test task with the given id and status.
    fn make_task(id: &str, status: TaskStatus) -> Task {
        Task {
            id: id.to_string(),
            title: format!("Task {id}"),
            description: "test task".to_string(),
            status,
            task_type: TaskType::Code,
            assigned_to: None,
            notes: None,
            created_at: 1_700_000_000,
            updated_at: 1_700_000_000,
        }
    }

    // ── State machine tests ─────────────────────────────────────────────

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

    // ── Integration tests (Redis) ───────────────────────────────────────

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL"]
    async fn save_and_get_task() {
        let pool = RedisPool::connect("redis://127.0.0.1:6379")
            .await
            .expect("connect to Redis");
        let store = TaskStore::new(pool, None);

        let task = make_task("test-save-get", TaskStatus::Pending);
        store.save_task(&task).await.expect("save task");

        let loaded = store.get_task("test-save-get").await.expect("get task");
        assert_eq!(loaded.id, "test-save-get");
        assert_eq!(loaded.status, TaskStatus::Pending);

        // Cleanup
        store
            .redis
            .del(&TaskStore::task_key("test-save-get"))
            .await
            .expect("cleanup");
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL"]
    async fn update_task_valid_transition() {
        let pool = RedisPool::connect("redis://127.0.0.1:6379")
            .await
            .expect("connect to Redis");
        let store = TaskStore::new(pool, None);

        let task = make_task("test-update-valid", TaskStatus::Pending);
        store.save_task(&task).await.expect("save task");

        let updated = store
            .update_task("test-update-valid", TaskStatus::InProgress, None)
            .await
            .expect("update task");
        assert_eq!(updated.status, TaskStatus::InProgress);

        // Cleanup
        store
            .redis
            .del(&TaskStore::task_key("test-update-valid"))
            .await
            .expect("cleanup");
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL"]
    async fn update_task_invalid_transition() {
        let pool = RedisPool::connect("redis://127.0.0.1:6379")
            .await
            .expect("connect to Redis");
        let store = TaskStore::new(pool, None);

        let task = make_task("test-update-invalid", TaskStatus::Pending);
        store.save_task(&task).await.expect("save task");

        let result = store
            .update_task("test-update-invalid", TaskStatus::Done, None)
            .await;
        assert!(
            matches!(result, Err(TaskToolError::InvalidTransition { .. })),
            "expected InvalidTransition error, got {result:?}"
        );

        // Cleanup
        store
            .redis
            .del(&TaskStore::task_key("test-update-invalid"))
            .await
            .expect("cleanup");
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL"]
    async fn update_task_not_found() {
        let pool = RedisPool::connect("redis://127.0.0.1:6379")
            .await
            .expect("connect to Redis");
        let store = TaskStore::new(pool, None);

        let result = store
            .update_task("nonexistent-task", TaskStatus::InProgress, None)
            .await;
        assert!(
            matches!(result, Err(TaskToolError::NotFound(_))),
            "expected NotFound error, got {result:?}"
        );
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL"]
    async fn list_tasks_filters_by_status() {
        let pool = RedisPool::connect("redis://127.0.0.1:6379")
            .await
            .expect("connect to Redis");
        let store = TaskStore::new(pool, None);

        let t1 = make_task("test-list-s-1", TaskStatus::Pending);
        let mut t2 = make_task("test-list-s-2", TaskStatus::InProgress);
        t2.assigned_to = Some("agent-a".to_string());

        store.save_task(&t1).await.expect("save t1");
        store.save_task(&t2).await.expect("save t2");

        let pending = store
            .list_tasks(Some(&TaskStatus::Pending), None)
            .await
            .expect("list pending");
        assert!(pending.iter().any(|t| t.id == "test-list-s-1"));

        let in_progress = store
            .list_tasks(Some(&TaskStatus::InProgress), None)
            .await
            .expect("list in_progress");
        assert!(in_progress.iter().any(|t| t.id == "test-list-s-2"));

        // Cleanup
        store
            .redis
            .del(&TaskStore::task_key("test-list-s-1"))
            .await
            .expect("cleanup");
        store
            .redis
            .del(&TaskStore::task_key("test-list-s-2"))
            .await
            .expect("cleanup");
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL"]
    async fn list_tasks_filters_by_assigned_to() {
        let pool = RedisPool::connect("redis://127.0.0.1:6379")
            .await
            .expect("connect to Redis");
        let store = TaskStore::new(pool, None);

        let mut t1 = make_task("test-list-a-1", TaskStatus::Pending);
        t1.assigned_to = Some("agent-x".to_string());
        let mut t2 = make_task("test-list-a-2", TaskStatus::Pending);
        t2.assigned_to = Some("agent-y".to_string());

        store.save_task(&t1).await.expect("save t1");
        store.save_task(&t2).await.expect("save t2");

        let filtered = store
            .list_tasks(None, Some("agent-x"))
            .await
            .expect("list by agent");
        assert!(filtered.iter().any(|t| t.id == "test-list-a-1"));
        assert!(!filtered.iter().any(|t| t.id == "test-list-a-2"));

        // Cleanup
        store
            .redis
            .del(&TaskStore::task_key("test-list-a-1"))
            .await
            .expect("cleanup");
        store
            .redis
            .del(&TaskStore::task_key("test-list-a-2"))
            .await
            .expect("cleanup");
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL"]
    async fn update_task_sets_notes() {
        let pool = RedisPool::connect("redis://127.0.0.1:6379")
            .await
            .expect("connect to Redis");
        let store = TaskStore::new(pool, None);

        let task = make_task("test-notes", TaskStatus::Pending);
        store.save_task(&task).await.expect("save task");

        let updated = store
            .update_task(
                "test-notes",
                TaskStatus::InProgress,
                Some("working on it".to_string()),
            )
            .await
            .expect("update task");
        assert_eq!(updated.notes.as_deref(), Some("working on it"));

        // Cleanup
        store
            .redis
            .del(&TaskStore::task_key("test-notes"))
            .await
            .expect("cleanup");
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL"]
    async fn full_lifecycle_pending_to_done() {
        let pool = RedisPool::connect("redis://127.0.0.1:6379")
            .await
            .expect("connect to Redis");
        let store = TaskStore::new(pool, None);

        let task = make_task("test-lifecycle", TaskStatus::Pending);
        store.save_task(&task).await.expect("save");

        // Pending -> InProgress
        let t = store
            .update_task("test-lifecycle", TaskStatus::InProgress, None)
            .await
            .expect("pending -> in_progress");
        assert_eq!(t.status, TaskStatus::InProgress);

        // InProgress -> InReview
        let t = store
            .update_task("test-lifecycle", TaskStatus::InReview, None)
            .await
            .expect("in_progress -> in_review");
        assert_eq!(t.status, TaskStatus::InReview);

        // InReview -> Done
        let t = store
            .update_task("test-lifecycle", TaskStatus::Done, None)
            .await
            .expect("in_review -> done");
        assert_eq!(t.status, TaskStatus::Done);

        // Done -> anything should fail
        let err = store
            .update_task("test-lifecycle", TaskStatus::Pending, None)
            .await;
        assert!(matches!(err, Err(TaskToolError::InvalidTransition { .. })));

        // Cleanup
        store
            .redis
            .del(&TaskStore::task_key("test-lifecycle"))
            .await
            .expect("cleanup");
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL"]
    async fn blocked_and_unblocked_lifecycle() {
        let pool = RedisPool::connect("redis://127.0.0.1:6379")
            .await
            .expect("connect to Redis");
        let store = TaskStore::new(pool, None);

        let task = make_task("test-blocked", TaskStatus::Pending);
        store.save_task(&task).await.expect("save");

        // Pending -> Blocked
        let t = store
            .update_task(
                "test-blocked",
                TaskStatus::Blocked,
                Some("needs human input".to_string()),
            )
            .await
            .expect("pending -> blocked");
        assert_eq!(t.status, TaskStatus::Blocked);
        assert_eq!(t.notes.as_deref(), Some("needs human input"));

        // Blocked -> InProgress
        let t = store
            .update_task("test-blocked", TaskStatus::InProgress, None)
            .await
            .expect("blocked -> in_progress");
        assert_eq!(t.status, TaskStatus::InProgress);

        // InProgress -> Done (direct)
        let t = store
            .update_task("test-blocked", TaskStatus::Done, None)
            .await
            .expect("in_progress -> done");
        assert_eq!(t.status, TaskStatus::Done);

        // Cleanup
        store
            .redis
            .del(&TaskStore::task_key("test-blocked"))
            .await
            .expect("cleanup");
    }
}

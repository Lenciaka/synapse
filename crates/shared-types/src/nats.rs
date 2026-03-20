//! NATS messaging client for the Synapse system.
//!
//! Provides a thin, async-safe wrapper around [`async_nats::Client`] with:
//!
//! * [`NatsClient::from_env`] — construct from the `NATS_URL` environment variable.
//! * [`NatsClient::publish`] — fire-and-forget publish to a NATS subject.
//! * [`NatsClient::subscribe`] — return an async `Stream` of raw NATS messages.
//!
//! # Subject schema
//!
//! Only the subjects listed in [`Subject`] are permitted.  All subjects follow
//! the `synapse.*.*` hierarchy defined in `CLAUDE.md` and
//! `infra/nats/subjects.md`.

use async_nats::Subscriber;
use futures_util::Stream;
use std::pin::Pin;
use thiserror::Error;

// ── Subject constants ────────────────────────────────────────────────────────

/// All NATS subjects used by Synapse.
///
/// Every subject is defined here to prevent accidental typos or undocumented
/// subject invention.  Never add a subject without updating `CLAUDE.md` and
/// `infra/nats/subjects.md`.
pub mod subjects {
    /// A new task has been created.
    pub const TASK_CREATED: &str = "synapse.task.created";
    /// A task's status has changed.
    pub const TASK_STATUS_CHANGED: &str = "synapse.task.status_changed";
    /// A single log line produced by an agent.
    pub const AGENT_LOG_LINE: &str = "synapse.agent.log_line";
    /// An agent's overall status has changed (online / paused / offline).
    pub const AGENT_STATUS_CHANGED: &str = "synapse.agent.status_changed";
    /// A human checkpoint is required before the agent may continue.
    pub const CHECKPOINT_REQUIRED: &str = "synapse.checkpoint.required";
    /// A human has approved a checkpoint.
    pub const CHECKPOINT_APPROVED: &str = "synapse.checkpoint.approved";
    /// A pull request has been opened by an agent.
    pub const PR_OPENED: &str = "synapse.pr.opened";
    /// A pull request has been reviewed.
    pub const PR_REVIEWED: &str = "synapse.pr.reviewed";
}

// ── Error type ───────────────────────────────────────────────────────────────

/// Errors that can occur when using the NATS client.
#[derive(Debug, Error)]
pub enum NatsError {
    /// The `NATS_URL` environment variable is missing.
    #[error("NATS_URL environment variable not set")]
    MissingUrl,

    /// The underlying `async-nats` client returned an error during connect.
    #[error("nats connection error: {0}")]
    Connect(#[from] async_nats::ConnectError),

    /// A publish operation failed.
    #[error("nats publish error: {0}")]
    Publish(#[from] async_nats::PublishError),

    /// A subscribe operation failed.
    #[error("nats subscribe error: {0}")]
    Subscribe(#[from] async_nats::SubscribeError),
}

// ── Client ───────────────────────────────────────────────────────────────────

/// Cheap-to-clone async NATS client.
///
/// Internally backed by [`async_nats::Client`], which is already `Clone` and
/// multiplexes all commands over a single managed connection.
#[derive(Clone, Debug)]
pub struct NatsClient {
    inner: async_nats::Client,
}

impl NatsClient {
    /// Creates a new [`NatsClient`] using the URL in the `NATS_URL`
    /// environment variable.
    ///
    /// # Errors
    ///
    /// Returns [`NatsError::MissingUrl`] when `NATS_URL` is unset, or
    /// [`NatsError::Connect`] when the connection cannot be established.
    pub async fn from_env() -> Result<Self, NatsError> {
        let url = std::env::var("NATS_URL").map_err(|_| NatsError::MissingUrl)?;
        Self::connect(&url).await
    }

    /// Creates a new [`NatsClient`] connecting to the given `url`.
    ///
    /// # Errors
    ///
    /// Returns [`NatsError::Connect`] when the connection fails.
    pub async fn connect(url: &str) -> Result<Self, NatsError> {
        let client = async_nats::connect(url).await?;
        Ok(Self { inner: client })
    }

    /// Publishes `payload` bytes to `subject`.
    ///
    /// The subject **must** be one of the constants in [`subjects`].
    ///
    /// # Errors
    ///
    /// Returns [`NatsError::Publish`] when the publish cannot be flushed to
    /// the NATS server.
    pub async fn publish(
        &self,
        subject: impl Into<String>,
        payload: impl Into<bytes::Bytes>,
    ) -> Result<(), NatsError> {
        self.inner.publish(subject.into(), payload.into()).await?;
        Ok(())
    }

    /// Subscribes to `subject` and returns a [`Stream`] of
    /// [`async_nats::Message`] values.
    ///
    /// The subject may contain NATS wildcards (e.g. `synapse.>`) for
    /// hierarchical subscriptions.
    ///
    /// # Errors
    ///
    /// Returns [`NatsError::Subscribe`] when the server rejects the
    /// subscription.
    pub async fn subscribe(
        &self,
        subject: impl Into<String>,
    ) -> Result<Pin<Box<dyn Stream<Item = async_nats::Message> + Send>>, NatsError> {
        let subscriber: Subscriber = self.inner.subscribe(subject.into()).await?;
        Ok(Box::pin(subscriber))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    /// Verifies that `from_env` fails cleanly when `NATS_URL` is absent.
    #[tokio::test]
    async fn missing_env_var_returns_error() {
        let saved = std::env::var("NATS_URL").ok();
        // SAFETY: test-only, single-threaded tokio runtime here.
        unsafe { std::env::remove_var("NATS_URL") };

        let result = NatsClient::from_env().await;
        assert!(
            matches!(result, Err(NatsError::MissingUrl)),
            "expected MissingUrl error"
        );

        if let Some(v) = saved {
            unsafe { std::env::set_var("NATS_URL", v) };
        }
    }

    /// Integration test: connect to a real NATS server, publish a message to
    /// a subject, subscribe and assert the received payload matches.
    ///
    /// Requires a live NATS server reachable at `NATS_URL` (default:
    /// `nats://127.0.0.1:4222`).
    ///
    /// Run with:
    /// ```text
    /// NATS_URL=nats://127.0.0.1:4222 cargo test -- --ignored nats_publish_subscribe_roundtrip
    /// ```
    #[tokio::test]
    #[ignore = "requires live NATS server at NATS_URL"]
    async fn nats_publish_subscribe_roundtrip() {
        let url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());

        let client = NatsClient::connect(&url).await.expect("connect to NATS");

        let subject = subjects::TASK_STATUS_CHANGED;
        let payload = br#"{"task_id":"t-1","status":"done"}"#;

        let mut stream = client.subscribe(subject).await.expect("subscribe").take(1);

        client
            .publish(subject, payload.as_ref())
            .await
            .expect("publish");

        let msg = stream
            .next()
            .await
            .expect("should receive a message within stream");

        assert_eq!(msg.payload.as_ref(), payload.as_ref());
    }

    /// Unit test: verifies that all subject constants remain consistent with
    /// the CLAUDE.md NATS schema.
    #[test]
    fn subject_constants_have_correct_prefix() {
        let all_subjects = [
            subjects::TASK_CREATED,
            subjects::TASK_STATUS_CHANGED,
            subjects::AGENT_LOG_LINE,
            subjects::AGENT_STATUS_CHANGED,
            subjects::CHECKPOINT_REQUIRED,
            subjects::CHECKPOINT_APPROVED,
            subjects::PR_OPENED,
            subjects::PR_REVIEWED,
        ];

        for subject in &all_subjects {
            assert!(
                subject.starts_with("synapse."),
                "subject `{subject}` must start with `synapse.`"
            );
        }
    }
}

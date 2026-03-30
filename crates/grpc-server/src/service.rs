//! `SynapseUI` gRPC service implementation.
//!
//! All RPCs currently return [`tonic::Status::unimplemented`] and will be
//! filled in by subsequent tasks (TASK-010, TASK-011, TASK-012).

use std::pin::Pin;
use std::sync::Arc;

use futures_util::Stream;
use tonic::{Request, Response, Status};

use crate::proto;

/// Stub implementation of the `SynapseUI` gRPC service.
///
/// Holds handles to Redis and NATS so that subsequent tasks (TASK-010,
/// TASK-011, TASK-012) can use them without changing the constructor
/// signature.  Each RPC method returns [`Status::unimplemented`] until the
/// corresponding task is completed.
#[derive(Clone)]
pub struct SynapseUiService {
    /// Redis connection pool for task and agent state queries.
    // Used by TASK-010 (ListTasks, GetTask, ListAgents).
    #[allow(dead_code)]
    pub(crate) redis: shared_types::storage::RedisPool,
    /// Optional NATS client for publishing checkpoint/agent events.
    // Used by TASK-011 (ApproveCheckpoint, PauseAgent, ResumeAgent).
    #[allow(dead_code)]
    pub(crate) nats: Option<Arc<shared_types::nats::NatsClient>>,
}

impl SynapseUiService {
    /// Creates a new [`SynapseUiService`] with the given Redis pool and
    /// optional NATS client.
    pub fn new(
        redis: shared_types::storage::RedisPool,
        nats: Option<Arc<shared_types::nats::NatsClient>>,
    ) -> Self {
        Self { redis, nats }
    }
}

#[tonic::async_trait]
impl proto::synapse_ui_server::SynapseUi for SynapseUiService {
    async fn list_tasks(
        &self,
        _request: Request<proto::ListTasksRequest>,
    ) -> Result<Response<proto::ListTasksResponse>, Status> {
        Err(Status::unimplemented("ListTasks is not yet implemented"))
    }

    async fn get_task(
        &self,
        _request: Request<proto::GetTaskRequest>,
    ) -> Result<Response<proto::GetTaskResponse>, Status> {
        Err(Status::unimplemented("GetTask is not yet implemented"))
    }

    async fn list_agents(
        &self,
        _request: Request<proto::ListAgentsRequest>,
    ) -> Result<Response<proto::ListAgentsResponse>, Status> {
        Err(Status::unimplemented("ListAgents is not yet implemented"))
    }

    async fn approve_checkpoint(
        &self,
        _request: Request<proto::ApproveCheckpointRequest>,
    ) -> Result<Response<proto::ApproveCheckpointResponse>, Status> {
        Err(Status::unimplemented(
            "ApproveCheckpoint is not yet implemented",
        ))
    }

    async fn pause_agent(
        &self,
        _request: Request<proto::PauseAgentRequest>,
    ) -> Result<Response<proto::PauseAgentResponse>, Status> {
        Err(Status::unimplemented("PauseAgent is not yet implemented"))
    }

    async fn resume_agent(
        &self,
        _request: Request<proto::ResumeAgentRequest>,
    ) -> Result<Response<proto::ResumeAgentResponse>, Status> {
        Err(Status::unimplemented("ResumeAgent is not yet implemented"))
    }

    /// Server-streaming response type for [`SynapseUi::subscribe_events`].
    type SubscribeEventsStream = Pin<Box<dyn Stream<Item = Result<proto::Event, Status>> + Send>>;

    async fn subscribe_events(
        &self,
        _request: Request<proto::SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        Err(Status::unimplemented(
            "SubscribeEvents is not yet implemented",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::synapse_ui_server::SynapseUi;

    async fn service() -> SynapseUiService {
        let url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let redis = shared_types::storage::RedisPool::connect(&url)
            .await
            .expect("connect to Redis for test");
        SynapseUiService::new(redis, None)
    }

    /// Helper to assert that a gRPC result is an `Unimplemented` status.
    fn assert_unimplemented<T: std::fmt::Debug>(result: Result<Response<T>, Status>) {
        let err = result.expect_err("expected Unimplemented status");
        assert_eq!(err.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL or redis://127.0.0.1:6379"]
    async fn list_tasks_returns_unimplemented() {
        let result = service()
            .await
            .list_tasks(Request::new(proto::ListTasksRequest::default()))
            .await;
        assert_unimplemented(result);
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL or redis://127.0.0.1:6379"]
    async fn get_task_returns_unimplemented() {
        let result = service()
            .await
            .get_task(Request::new(proto::GetTaskRequest::default()))
            .await;
        assert_unimplemented(result);
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL or redis://127.0.0.1:6379"]
    async fn list_agents_returns_unimplemented() {
        let result = service()
            .await
            .list_agents(Request::new(proto::ListAgentsRequest::default()))
            .await;
        assert_unimplemented(result);
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL or redis://127.0.0.1:6379"]
    async fn approve_checkpoint_returns_unimplemented() {
        let result = service()
            .await
            .approve_checkpoint(Request::new(proto::ApproveCheckpointRequest::default()))
            .await;
        assert_unimplemented(result);
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL or redis://127.0.0.1:6379"]
    async fn pause_agent_returns_unimplemented() {
        let result = service()
            .await
            .pause_agent(Request::new(proto::PauseAgentRequest::default()))
            .await;
        assert_unimplemented(result);
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL or redis://127.0.0.1:6379"]
    async fn resume_agent_returns_unimplemented() {
        let result = service()
            .await
            .resume_agent(Request::new(proto::ResumeAgentRequest::default()))
            .await;
        assert_unimplemented(result);
    }

    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL or redis://127.0.0.1:6379"]
    async fn subscribe_events_returns_unimplemented() {
        let result = service()
            .await
            .subscribe_events(Request::new(proto::SubscribeRequest::default()))
            .await;
        match result {
            Err(status) => assert_eq!(status.code(), tonic::Code::Unimplemented),
            Ok(_) => panic!("expected Unimplemented status"),
        }
    }
}

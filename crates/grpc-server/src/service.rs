//! `SynapseUI` gRPC service implementation.
//!
//! All RPCs currently return [`tonic::Status::unimplemented`] and will be
//! filled in by subsequent tasks (TASK-010, TASK-011, TASK-012).

use std::pin::Pin;

use futures_util::Stream;
use tonic::{Request, Response, Status};

use crate::proto;

/// Stub implementation of the `SynapseUI` gRPC service.
///
/// Each RPC method returns [`Status::unimplemented`] until the corresponding
/// task is completed.
#[derive(Debug, Default)]
pub struct SynapseUiService;

impl SynapseUiService {
    /// Creates a new [`SynapseUiService`].
    pub fn new() -> Self {
        Self
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

    fn service() -> SynapseUiService {
        SynapseUiService::new()
    }

    /// Helper to assert that a gRPC result is an `Unimplemented` status.
    fn assert_unimplemented<T: std::fmt::Debug>(result: Result<Response<T>, Status>) {
        let err = result.expect_err("expected Unimplemented status");
        assert_eq!(err.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn list_tasks_returns_unimplemented() {
        let result = service()
            .list_tasks(Request::new(proto::ListTasksRequest::default()))
            .await;
        assert_unimplemented(result);
    }

    #[tokio::test]
    async fn get_task_returns_unimplemented() {
        let result = service()
            .get_task(Request::new(proto::GetTaskRequest::default()))
            .await;
        assert_unimplemented(result);
    }

    #[tokio::test]
    async fn list_agents_returns_unimplemented() {
        let result = service()
            .list_agents(Request::new(proto::ListAgentsRequest::default()))
            .await;
        assert_unimplemented(result);
    }

    #[tokio::test]
    async fn approve_checkpoint_returns_unimplemented() {
        let result = service()
            .approve_checkpoint(Request::new(proto::ApproveCheckpointRequest::default()))
            .await;
        assert_unimplemented(result);
    }

    #[tokio::test]
    async fn pause_agent_returns_unimplemented() {
        let result = service()
            .pause_agent(Request::new(proto::PauseAgentRequest::default()))
            .await;
        assert_unimplemented(result);
    }

    #[tokio::test]
    async fn resume_agent_returns_unimplemented() {
        let result = service()
            .resume_agent(Request::new(proto::ResumeAgentRequest::default()))
            .await;
        assert_unimplemented(result);
    }

    #[tokio::test]
    async fn subscribe_events_returns_unimplemented() {
        let result = service()
            .subscribe_events(Request::new(proto::SubscribeRequest::default()))
            .await;
        match result {
            Err(status) => assert_eq!(status.code(), tonic::Code::Unimplemented),
            Ok(_) => panic!("expected Unimplemented status"),
        }
    }
}

//! gRPC server library for the Synapse multi-agent system.
//!
//! Provides a tonic gRPC server on a configurable port (default `:3001`) that
//! implements the `SynapseUI` service defined in `proto/synapse.proto`.
//!
//! # Architecture
//!
//! ```text
//! TUI (gRPC :3001) --> tonic --> SynapseUiService (stub)
//! ```
//!
//! The server connects to Redis and NATS via [`shared_types`] for state
//! storage and event streaming.  RPC implementations are added in subsequent
//! tasks (TASK-010, TASK-011, TASK-012).

/// Proto-generated types and server traits for the `SynapseUI` gRPC service.
pub mod proto {
    tonic::include_proto!("synapse");
}

pub mod server;
pub mod service;

pub use server::{GrpcServerConfig, ServerError};

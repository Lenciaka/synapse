//! gRPC server binary — tonic on :3001.
//!
//! Subscribes to NATS and streams events to the TUI client.
//! Full service implementation is done in TASK-009.

/// Proto-generated types and server traits for the `SynapseUI` gRPC service.
pub mod synapse {
    tonic::include_proto!("synapse");
}

fn main() {
    // Stub — full implementation in TASK-009.
}

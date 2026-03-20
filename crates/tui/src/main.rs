//! TUI binary — ratatui + crossterm, connects to gRPC :3001.
//!
//! Renders agent status, task list, and log stream for the human operator.
//! Full implementation is done in TASK-013.

/// Proto-generated types and client stubs for the `SynapseUI` gRPC service.
pub mod synapse {
    tonic::include_proto!("synapse");
}

fn main() {
    // Stub — full implementation in TASK-013.
}

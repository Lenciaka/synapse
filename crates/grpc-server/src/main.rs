//! gRPC server binary -- tonic on a configurable port (default :3001).
//!
//! Implements the `SynapseUI` service for the TUI client and streams events
//! from NATS.

use grpc_server::{GrpcServerConfig, ServerError};

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,grpc_server=debug".into()),
        )
        .init();

    let config = GrpcServerConfig::from_env();
    grpc_server::server::run(config).await
}

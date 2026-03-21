//! gRPC server setup and lifecycle management.
//!
//! [`GrpcServerConfig`] captures the configuration needed to start the server,
//! and [`run`] is the top-level entry point that builds the tonic service,
//! binds the TCP listener, and runs until a shutdown signal is received.

use std::net::SocketAddr;

use thiserror::Error;
use tokio_util::sync::CancellationToken;

use crate::proto::synapse_ui_server::SynapseUiServer;
use crate::service::SynapseUiService;

/// Default port the gRPC server listens on.
const DEFAULT_PORT: u16 = 3001;

/// Errors that can occur while running the gRPC server.
#[derive(Debug, Error)]
pub enum ServerError {
    /// Failed to start the tonic transport server.
    #[error("gRPC transport error: {0}")]
    Transport(#[from] tonic::transport::Error),
}

/// Configuration for the gRPC server.
#[derive(Debug, Clone)]
pub struct GrpcServerConfig {
    /// TCP port to listen on (overridden by `GRPC_PORT` env var).
    pub port: u16,
}

impl Default for GrpcServerConfig {
    fn default() -> Self {
        Self { port: DEFAULT_PORT }
    }
}

impl GrpcServerConfig {
    /// Creates a config by reading the `GRPC_PORT` environment variable,
    /// falling back to the default port (3001) when the variable is absent
    /// or not a valid `u16`.
    pub fn from_env() -> Self {
        let port = std::env::var("GRPC_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(DEFAULT_PORT);
        Self { port }
    }
}

/// Runs the gRPC server until a SIGTERM or SIGINT signal is received.
///
/// This is the main entry point for the server binary.
///
/// # Errors
///
/// Returns [`ServerError::Transport`] when the tonic server fails to start
/// or encounters a transport-level error.
pub async fn run(config: GrpcServerConfig) -> Result<(), ServerError> {
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    let ct = CancellationToken::new();
    let service = SynapseUiService::new();

    tracing::info!("gRPC server listening on {addr}");

    tonic::transport::Server::builder()
        .add_service(SynapseUiServer::new(service))
        .serve_with_shutdown(addr, shutdown_signal(ct))
        .await?;

    tracing::info!("gRPC server shut down gracefully");
    Ok(())
}

/// Waits for a SIGTERM or SIGINT signal and cancels the provided token.
async fn shutdown_signal(ct: CancellationToken) {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    tracing::info!("shutdown signal received, starting graceful shutdown");
    ct.cancel();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_port() {
        let cfg = GrpcServerConfig::default();
        assert_eq!(cfg.port, 3001);
    }

    #[test]
    fn config_from_env_falls_back_to_default() {
        // When GRPC_PORT is not set, from_env should return default 3001.
        // Note: env var manipulation in tests is inherently racy but
        // acceptable for unit tests.
        let saved = std::env::var("GRPC_PORT").ok();
        unsafe { std::env::remove_var("GRPC_PORT") };

        let cfg = GrpcServerConfig::from_env();
        assert_eq!(cfg.port, 3001);

        if let Some(v) = saved {
            unsafe { std::env::set_var("GRPC_PORT", v) };
        }
    }

    #[test]
    fn config_from_env_reads_port() {
        let saved = std::env::var("GRPC_PORT").ok();
        unsafe { std::env::set_var("GRPC_PORT", "4001") };

        let cfg = GrpcServerConfig::from_env();
        assert_eq!(cfg.port, 4001);

        match saved {
            Some(v) => unsafe { std::env::set_var("GRPC_PORT", v) },
            None => unsafe { std::env::remove_var("GRPC_PORT") },
        }
    }

    #[test]
    fn config_from_env_ignores_invalid() {
        let saved = std::env::var("GRPC_PORT").ok();
        unsafe { std::env::set_var("GRPC_PORT", "not_a_number") };

        let cfg = GrpcServerConfig::from_env();
        assert_eq!(cfg.port, 3001);

        match saved {
            Some(v) => unsafe { std::env::set_var("GRPC_PORT", v) },
            None => unsafe { std::env::remove_var("GRPC_PORT") },
        }
    }

    #[tokio::test]
    async fn server_starts_and_shuts_down() {
        // Use port 0 so the OS assigns an ephemeral port.
        // We run the server briefly and then cancel it.
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let ct = CancellationToken::new();
        let ct_clone = ct.clone();

        let service = SynapseUiService::new();

        let server = tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(SynapseUiServer::new(service))
                .serve_with_shutdown(addr, ct_clone.cancelled_owned())
                .await
        });

        // Give the server a moment to start.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Cancel and verify it shuts down cleanly.
        ct.cancel();
        let result = server.await.expect("server task should complete");
        assert!(result.is_ok(), "server should shut down without error");
    }
}

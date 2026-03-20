//! MCP server binary -- axum HTTP on a configurable port (default :3000).
//!
//! Accepts MCP tool calls from AI agents and publishes events to NATS.

use mcp_server::{McpServerConfig, ServerError};

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,mcp_server=debug".into()),
        )
        .init();

    let config = McpServerConfig::from_env();
    mcp_server::server::run(config).await
}

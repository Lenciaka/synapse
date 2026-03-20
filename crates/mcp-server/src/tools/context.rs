//! Context management MCP tools.
//!
//! Provides three tools for agent context storage backed by Redis:
//!
//! * [`ReadContext`] -- retrieve a value by key.
//! * [`WriteContext`] -- store a key/value pair.
//! * [`SearchMemory`] -- find keys matching a prefix.
//!
//! All keys are stored under the `synapse:ctx:` namespace in Redis to avoid
//! collisions with other Synapse subsystems.
//!
//! Phase 2 upgrade: `search_memory` will be enhanced with Qdrant vector search
//! via rig-core embeddings.  The current implementation performs a simple Redis
//! key prefix scan.

use std::borrow::Cow;

use rmcp::handler::server::router::tool::{AsyncTool, ToolBase};
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::handler::SynapseMcpHandler;

/// Redis key prefix used for all context entries.
const CTX_PREFIX: &str = "synapse:ctx:";

// ---------------------------------------------------------------------------
// read_context
// ---------------------------------------------------------------------------

/// MCP tool that reads a context value from Redis.
pub struct ReadContext;

/// Input parameters for the `read_context` tool.
#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct ReadContextInput {
    /// The context key to look up (without the `synapse:ctx:` prefix).
    pub key: String,
}

/// Output of the `read_context` tool.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ReadContextOutput {
    /// The stored value, or `null` when the key does not exist.
    pub value: Option<String>,
}

impl ToolBase for ReadContext {
    type Parameter = ReadContextInput;
    type Output = ReadContextOutput;
    type Error = rmcp::ErrorData;

    fn name() -> Cow<'static, str> {
        "read_context".into()
    }

    fn description() -> Option<Cow<'static, str>> {
        Some("Read a context value from Redis by key.".into())
    }
}

impl AsyncTool<SynapseMcpHandler> for ReadContext {
    async fn invoke(
        service: &SynapseMcpHandler,
        param: ReadContextInput,
    ) -> Result<ReadContextOutput, rmcp::ErrorData> {
        let redis = service.redis().ok_or_else(|| {
            rmcp::ErrorData::internal_error("Redis connection not available".to_string(), None)
        })?;

        let full_key = format!("{CTX_PREFIX}{}", param.key);
        let value = redis
            .get(&full_key)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(format!("Redis GET failed: {e}"), None))?;

        Ok(ReadContextOutput { value })
    }
}

// ---------------------------------------------------------------------------
// write_context
// ---------------------------------------------------------------------------

/// MCP tool that writes a context value to Redis.
pub struct WriteContext;

/// Input parameters for the `write_context` tool.
#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct WriteContextInput {
    /// The context key to store (without the `synapse:ctx:` prefix).
    pub key: String,
    /// The value to associate with the key.
    pub value: String,
}

/// Output of the `write_context` tool.
#[derive(Debug, Serialize, JsonSchema)]
pub struct WriteContextOutput {
    /// Whether the write succeeded.
    pub success: bool,
}

impl ToolBase for WriteContext {
    type Parameter = WriteContextInput;
    type Output = WriteContextOutput;
    type Error = rmcp::ErrorData;

    fn name() -> Cow<'static, str> {
        "write_context".into()
    }

    fn description() -> Option<Cow<'static, str>> {
        Some("Write a context key/value pair to Redis.".into())
    }
}

impl AsyncTool<SynapseMcpHandler> for WriteContext {
    async fn invoke(
        service: &SynapseMcpHandler,
        param: WriteContextInput,
    ) -> Result<WriteContextOutput, rmcp::ErrorData> {
        let redis = service.redis().ok_or_else(|| {
            rmcp::ErrorData::internal_error("Redis connection not available".to_string(), None)
        })?;

        let full_key = format!("{CTX_PREFIX}{}", param.key);
        redis
            .set(&full_key, &param.value)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(format!("Redis SET failed: {e}"), None))?;

        Ok(WriteContextOutput { success: true })
    }
}

// ---------------------------------------------------------------------------
// search_memory
// ---------------------------------------------------------------------------

/// MCP tool that searches for context keys matching a prefix.
///
/// Phase 2 upgrade: this will be enhanced with Qdrant vector search via
/// rig-core embeddings for semantic memory retrieval.  The current
/// implementation performs a Redis `KEYS` prefix scan.
pub struct SearchMemory;

/// Input parameters for the `search_memory` tool.
#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct SearchMemoryInput {
    /// Prefix query to match against context keys (without `synapse:ctx:`).
    pub query: String,
}

/// Output of the `search_memory` tool.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SearchMemoryOutput {
    /// Context keys (without prefix) that matched the query.
    pub keys: Vec<String>,
}

impl ToolBase for SearchMemory {
    type Parameter = SearchMemoryInput;
    type Output = SearchMemoryOutput;
    type Error = rmcp::ErrorData;

    fn name() -> Cow<'static, str> {
        "search_memory".into()
    }

    fn description() -> Option<Cow<'static, str>> {
        Some(
            "Search for context keys matching a prefix. \
             (Phase 2: will upgrade to Qdrant vector search.)"
                .into(),
        )
    }
}

impl AsyncTool<SynapseMcpHandler> for SearchMemory {
    async fn invoke(
        service: &SynapseMcpHandler,
        param: SearchMemoryInput,
    ) -> Result<SearchMemoryOutput, rmcp::ErrorData> {
        let redis = service.redis().ok_or_else(|| {
            rmcp::ErrorData::internal_error("Redis connection not available".to_string(), None)
        })?;

        let pattern = format!("{CTX_PREFIX}{}*", param.query);
        let full_keys: Vec<String> = redis.keys(&pattern).await.map_err(|e| {
            rmcp::ErrorData::internal_error(format!("Redis KEYS failed: {e}"), None)
        })?;

        // Strip the namespace prefix before returning to callers.
        let keys = full_keys
            .into_iter()
            .filter_map(|k| k.strip_prefix(CTX_PREFIX).map(String::from))
            .collect();

        Ok(SearchMemoryOutput { keys })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_context_tool_metadata() {
        assert_eq!(ReadContext::name(), "read_context");
        assert!(ReadContext::description().is_some());
    }

    #[test]
    fn write_context_tool_metadata() {
        assert_eq!(WriteContext::name(), "write_context");
        assert!(WriteContext::description().is_some());
    }

    #[test]
    fn search_memory_tool_metadata() {
        assert_eq!(SearchMemory::name(), "search_memory");
        assert!(SearchMemory::description().is_some());
    }

    /// Integration test: requires a live Redis server at `REDIS_URL`.
    /// Verifies the write_context -> read_context roundtrip.
    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL"]
    async fn roundtrip_write_then_read() {
        let url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let pool = shared_types::RedisPool::connect(&url)
            .await
            .expect("connect to Redis");
        let handler = SynapseMcpHandler::with_redis(pool.clone());

        // Write
        let write_result = WriteContext::invoke(
            &handler,
            WriteContextInput {
                key: "test:roundtrip".into(),
                value: "hello-context".into(),
            },
        )
        .await
        .expect("write_context");
        assert!(write_result.success);

        // Read back
        let read_result = ReadContext::invoke(
            &handler,
            ReadContextInput {
                key: "test:roundtrip".into(),
            },
        )
        .await
        .expect("read_context");
        assert_eq!(read_result.value.as_deref(), Some("hello-context"));

        // Clean up
        pool.del("synapse:ctx:test:roundtrip")
            .await
            .expect("cleanup");
    }

    /// Integration test: requires a live Redis server at `REDIS_URL`.
    /// Verifies that search_memory returns correct prefix-matched keys.
    #[tokio::test]
    #[ignore = "requires live Redis at REDIS_URL"]
    async fn search_memory_prefix_match() {
        let url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let pool = shared_types::RedisPool::connect(&url)
            .await
            .expect("connect to Redis");
        let handler = SynapseMcpHandler::with_redis(pool.clone());

        // Seed some keys
        pool.set("synapse:ctx:project:alpha", "a")
            .await
            .expect("set alpha");
        pool.set("synapse:ctx:project:beta", "b")
            .await
            .expect("set beta");
        pool.set("synapse:ctx:other:gamma", "c")
            .await
            .expect("set gamma");

        // Search for project: prefix
        let result = SearchMemory::invoke(
            &handler,
            SearchMemoryInput {
                query: "project:".into(),
            },
        )
        .await
        .expect("search_memory");

        assert!(result.keys.contains(&"project:alpha".to_string()));
        assert!(result.keys.contains(&"project:beta".to_string()));
        assert!(!result.keys.contains(&"other:gamma".to_string()));

        // Clean up
        pool.del("synapse:ctx:project:alpha")
            .await
            .expect("cleanup");
        pool.del("synapse:ctx:project:beta").await.expect("cleanup");
        pool.del("synapse:ctx:other:gamma").await.expect("cleanup");
    }
}

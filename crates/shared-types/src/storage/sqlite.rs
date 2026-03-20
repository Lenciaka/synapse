//! SQLite database connection and migration management for Synapse.
//!
//! Provides an [`SqliteDb`] handle backed by a `sqlx` connection pool.
//! On first use, call [`SqliteDb::migrate`] to apply all pending migrations
//! located in the `migrations/` directory relative to the crate root.
//!
//! The database path is read from the `DATABASE_URL` environment variable
//! (e.g. `sqlite:///data/synapse.db` or `sqlite::memory:`).

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::str::FromStr;
use thiserror::Error;

/// Errors that can occur when using the SQLite database.
#[derive(Debug, Error)]
pub enum SqliteError {
    /// The `DATABASE_URL` environment variable is missing.
    #[error("DATABASE_URL environment variable not set")]
    MissingUrl,

    /// An error returned by the underlying sqlx driver.
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// A migration error.
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

/// Lightweight handle over a SQLite connection pool.
///
/// Clone-cheap — the inner [`SqlitePool`] is reference-counted.
#[derive(Clone, Debug)]
pub struct SqliteDb {
    pool: SqlitePool,
}

impl SqliteDb {
    /// Opens the database at the URL contained in the `DATABASE_URL`
    /// environment variable.
    ///
    /// # Errors
    ///
    /// Returns [`SqliteError::MissingUrl`] when the variable is absent, or a
    /// [`SqliteError::Sqlx`] variant when the connection fails.
    pub async fn from_env() -> Result<Self, SqliteError> {
        let url = std::env::var("DATABASE_URL").map_err(|_| SqliteError::MissingUrl)?;
        Self::connect(&url).await
    }

    /// Opens (or creates) a SQLite database at `url`.
    ///
    /// The URL follows the sqlx format, e.g. `sqlite::memory:` or
    /// `sqlite:///path/to/db.sqlite`.
    ///
    /// # Errors
    ///
    /// Returns a [`SqliteError::Sqlx`] variant when the connection fails.
    pub async fn connect(url: &str) -> Result<Self, SqliteError> {
        let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
    }

    /// Applies all pending migrations from the embedded `migrations/`
    /// directory.
    ///
    /// Idempotent — already-applied migrations are skipped.
    ///
    /// # Errors
    ///
    /// Returns a [`SqliteError::Migrate`] variant if a migration fails.
    pub async fn migrate(&self) -> Result<(), SqliteError> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Returns a reference to the underlying connection pool.
    ///
    /// Prefer the typed helper methods on this struct where possible.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Inserts a row into the `audit_log` table.
    ///
    /// # Parameters
    ///
    /// * `event_type` — a short label such as `"task.status_changed"`.
    /// * `payload`    — a JSON string describing the event.
    ///
    /// Returns the `rowid` of the newly inserted row.
    ///
    /// # Errors
    ///
    /// Returns a [`SqliteError::Sqlx`] variant on database errors.
    pub async fn insert_audit_log(
        &self,
        event_type: &str,
        payload: &str,
    ) -> Result<i64, SqliteError> {
        let result = sqlx::query("INSERT INTO audit_log (event_type, payload) VALUES (?, ?)")
            .bind(event_type)
            .bind(payload)
            .execute(&self.pool)
            .await?;
        Ok(result.last_insert_rowid())
    }

    /// Fetches a single audit log row by its `id`.
    ///
    /// Returns `None` when no row with that id exists.
    ///
    /// # Errors
    ///
    /// Returns a [`SqliteError::Sqlx`] variant on database errors.
    pub async fn get_audit_log(&self, id: i64) -> Result<Option<AuditLogRow>, SqliteError> {
        let row =
            sqlx::query("SELECT id, event_type, payload, created_at FROM audit_log WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|r| AuditLogRow {
            id: r.get("id"),
            event_type: r.get("event_type"),
            payload: r.get("payload"),
            created_at: r.get("created_at"),
        }))
    }
}

/// A row from the `audit_log` table.
#[derive(Debug, Clone)]
pub struct AuditLogRow {
    /// Primary key.
    pub id: i64,
    /// Short label for the event category.
    pub event_type: String,
    /// JSON payload string.
    pub payload: String,
    /// ISO-8601 timestamp (UTC) set by the database at insert time.
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs an in-memory integration test: migrate, insert, read back.
    /// No external services needed — SQLite is embedded.
    #[tokio::test]
    async fn audit_log_insert_and_read() {
        let db = SqliteDb::connect("sqlite::memory:")
            .await
            .expect("open in-memory db");
        db.migrate().await.expect("run migrations");

        let row_id = db
            .insert_audit_log(
                "task.status_changed",
                r#"{"task_id":"t-1","status":"done"}"#,
            )
            .await
            .expect("insert audit log");

        assert!(row_id > 0, "should get a positive rowid");

        let row = db
            .get_audit_log(row_id)
            .await
            .expect("query audit log")
            .expect("row should exist");

        assert_eq!(row.event_type, "task.status_changed");
        assert_eq!(row.payload, r#"{"task_id":"t-1","status":"done"}"#);
        assert!(!row.created_at.is_empty());
    }

    /// Verifies that reading a non-existent id returns None.
    #[tokio::test]
    async fn audit_log_missing_id_returns_none() {
        let db = SqliteDb::connect("sqlite::memory:")
            .await
            .expect("open in-memory db");
        db.migrate().await.expect("run migrations");

        let result = db
            .get_audit_log(999_999)
            .await
            .expect("query should not fail");
        assert!(result.is_none());
    }

    /// Verifies that `from_env` fails cleanly when DATABASE_URL is absent.
    #[tokio::test]
    async fn missing_env_var_returns_error() {
        let saved = std::env::var("DATABASE_URL").ok();
        std::env::remove_var("DATABASE_URL");

        let result = SqliteDb::from_env().await;
        assert!(
            matches!(result, Err(SqliteError::MissingUrl)),
            "expected MissingUrl error"
        );

        if let Some(v) = saved {
            std::env::set_var("DATABASE_URL", v);
        }
    }

    /// Integration test: uses a real file-backed DB at DATABASE_URL.
    /// Run with `DATABASE_URL=sqlite:///tmp/synapse_test.db cargo test -- --ignored`
    #[tokio::test]
    #[ignore = "requires DATABASE_URL env var pointing to a writable path"]
    async fn file_backed_roundtrip() {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:///tmp/synapse_test.db".to_string());
        let db = SqliteDb::connect(&url).await.expect("open db");
        db.migrate().await.expect("run migrations");

        let row_id = db
            .insert_audit_log("test.event", r#"{"hello":"world"}"#)
            .await
            .expect("insert");
        let row = db.get_audit_log(row_id).await.expect("query").unwrap();
        assert_eq!(row.event_type, "test.event");
    }
}

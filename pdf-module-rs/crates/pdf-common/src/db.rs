//! Database connection pool and migration runner.
//!
//! Uses `sqlx` for async connection pooling and migration management.
//! Supports PostgreSQL and SQLite backends via feature flags.
//!
//! # Feature flag
//!
//! Enable with `features = ["database"]` in `Cargo.toml`.
//!
//! # Usage
//!
//! ```ignore
//! use pdf_common::db::{create_pool, run_migrations};
//!
//! let pool = create_pool("sqlite:./data/app.db?mode=rwc", 5).await?;
//! run_migrations(&pool).await?;
//! ```

use sqlx::migrate::Migrator;
use sqlx::pool::PoolOptions;
use sqlx::{AnyPool, SqlitePool};
use std::path::Path;
use std::time::Duration;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Create a SQLite connection pool with WAL mode and recommended pragmas.
pub async fn create_sqlite_pool(database_url: &str, max_connections: u32) -> Result<SqlitePool, sqlx::Error> {
    let pool = PoolOptions::<sqlx::Sqlite>::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await?;

    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA busy_timeout = 5000")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&pool)
        .await?;

    Ok(pool)
}

/// Create a generic connection pool (SQLite or PostgreSQL).
pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<AnyPool, sqlx::Error> {
    PoolOptions::<sqlx::Any>::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}

/// Run pending migrations.
pub async fn run_migrations(pool: &AnyPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}

/// Run pending migrations on a SQLite pool.
pub async fn run_sqlite_migrations(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_sqlite_pool_in_memory() {
        let pool = create_sqlite_pool("sqlite::memory:", 1).await;
        assert!(pool.is_ok());
    }

    #[tokio::test]
    async fn create_pool_in_memory() {
        let pool = create_pool("sqlite::memory:", 1).await;
        assert!(pool.is_ok());
    }
}
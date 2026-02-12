//! Time Capsule Database Service
//!
//! Provides database access for the quantum computer time capsule feature.
//! Uses a lazily-initialized connection pool that's only created when first accessed.
//!
//! Port of defuse.ca/src/libs/TimeCapsule.php

use sqlx::MySqlPool;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Global connection pool - lazily initialized on first use
static POOL: OnceLock<MySqlPool> = OnceLock::new();

/// Connect to the database eagerly at startup to fail fast if misconfigured.
pub async fn ensure_db_connection_works() -> Result<(), sqlx::Error> {
    get_pool().await?;
    Ok(())
}

/// Get or create the database connection pool
async fn get_pool() -> Result<&'static MySqlPool, sqlx::Error> {
    if let Some(pool) = POOL.get() {
        return Ok(pool);
    }

    let url = std::env::var("TIMECAPSULE_DATABASE_URL")
        .expect("TIMECAPSULE_DATABASE_URL must be set for time capsule page");
    let pool = MySqlPool::connect(&url).await?;

    // Race-safe: if another task initialized first, use theirs and drop ours
    Ok(POOL.get_or_init(|| pool))
}

/// Add an encrypted message entry to the time capsule
pub async fn add_entry(message: &str) -> Result<bool, sqlx::Error> {
    let pool = get_pool().await?;
    let timestamp = now();

    let result = sqlx::query("INSERT INTO timecapsule (timestamp, message) VALUES (?, ?)")
        .bind(timestamp)
        .bind(message)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Get the total count of messages in the archive
pub async fn get_message_count() -> Result<i64, sqlx::Error> {
    let pool = get_pool().await?;

    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) AS count FROM timecapsule")
        .fetch_one(pool)
        .await?;

    Ok(result.0)
}

/// Get the timestamp of the most recent message
pub async fn get_last_timestamp() -> Result<Option<i64>, sqlx::Error> {
    let pool = get_pool().await?;

    let result: Option<(i64,)> =
        sqlx::query_as("SELECT timestamp FROM timecapsule ORDER BY id DESC LIMIT 1")
            .fetch_optional(pool)
            .await?;

    Ok(result.map(|(ts,)| ts))
}

/// Get all messages in order (for archive download)
pub async fn get_all_entries_in_order() -> Result<Vec<String>, sqlx::Error> {
    let pool = get_pool().await?;

    // The message column is a BLOB in MySQL, so we need to read it as bytes
    let rows: Vec<(Vec<u8>,)> =
        sqlx::query_as("SELECT message FROM timecapsule ORDER BY id")
            .fetch_all(pool)
            .await?;

    Ok(rows
        .into_iter()
        .map(|(msg,)| String::from_utf8_lossy(&msg).into_owned())
        .collect())
}

/// Get current unix timestamp
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Convert seconds to human-readable format (matching PHP's time_for_human)
pub fn time_for_human(seconds: i64) -> String {
    if seconds > 24 * 3600 {
        format!("{} days", (seconds as f64 / (24.0 * 3600.0)).round() as i64)
    } else if seconds > 3600 {
        format!("{} hours", (seconds as f64 / 3600.0).round() as i64)
    } else if seconds > 60 {
        format!("{} minutes", (seconds as f64 / 60.0).round() as i64)
    } else {
        format!("{} seconds", seconds)
    }
}

/// Get current unix timestamp (public for use in handlers)
pub fn current_timestamp() -> i64 {
    now()
}

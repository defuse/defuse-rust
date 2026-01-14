//! PHPCount - Privacy-preserving hit counter
//!
//! Tracks page hits without storing IP addresses directly.
//! Uses SHA256(pageID + IP) to track unique visits.
//!
//! Port of defuse.ca/src/libs/phpcount.php

use sha2::{Digest, Sha256};
use sqlx::MySqlPool;
use std::time::{SystemTime, UNIX_EPOCH};

/// How long to remember a hit for unique tracking (30 days)
const HIT_OLD_AFTER_SECONDS: i64 = 2592000;

/// Bot detection keywords (case-insensitive)
const BOT_KEYWORDS: &[&str] = &[
    "bot", "spider", "spyder", "crawler", "walker", "search",
    "yahoo", "holmes", "htdig", "archive", "tineye", "yacy", "yeti",
];

/// IPs to ignore
/// Note: In production, requests come through a reverse proxy that sets X-Forwarded-For,
/// so we don't need to ignore localhost there. Empty for now.
const IP_IGNORE_LIST: &[&str] = &[];

/// Hit counts for a page and site totals.
#[derive(Clone, Debug, Default)]
pub struct HitCounts {
    pub page_hits: u32,
    pub unique_hits: u32,
    pub total_hits: u32,
    pub total_unique_hits: u32,
}

#[derive(Clone)]
pub struct PhpCountService {
    pool: MySqlPool,
}

impl PhpCountService {
    /// Create a new PHPCount service with the given database pool
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Connect to the database and create a new service
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = MySqlPool::connect(database_url).await?;
        Ok(Self::new(pool))
    }

    /// Record a hit for a page. Returns true if the hit was counted.
    ///
    /// Skips counting for:
    /// - Search bots (based on user agent)
    /// - Ignored IPs (localhost)
    pub async fn add_hit(
        &self,
        page_id: &str,
        client_ip: &str,
        user_agent: &str,
    ) -> Result<bool, sqlx::Error> {
        // Skip search bots
        if Self::is_search_bot(user_agent) {
            return Ok(false);
        }

        // Skip ignored IPs
        if IP_IGNORE_LIST.contains(&client_ip) {
            return Ok(false);
        }

        // Clean up old entries periodically
        self.cleanup().await?;

        // Ensure page has counter entries
        self.create_counts_if_not_present(page_id).await?;

        // Check if this is a unique hit
        if self.is_unique_hit(page_id, client_ip).await? {
            self.count_hit(page_id, true).await?;
            self.log_hit(page_id, client_ip).await?;
        }

        // Always count non-unique hits
        self.count_hit(page_id, false).await?;

        Ok(true)
    }

    /// Get hit count for a specific page (internal helper)
    async fn get_hits(&self, page_id: &str, unique: bool) -> Result<u32, sqlx::Error> {
        // Ensure page exists first
        self.create_counts_if_not_present(page_id).await?;

        let is_unique: i8 = if unique { 1 } else { 0 };
        let result: Option<(u32,)> = sqlx::query_as(
            "SELECT hitcount FROM hits WHERE pageid = ? AND isunique = ?"
        )
        .bind(page_id)
        .bind(is_unique)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(count,)| count).unwrap_or(0))
    }

    /// Get total hits across all pages (internal helper)
    ///
    /// Note: When unique=true, this returns the SUM of each page's unique hits,
    /// NOT site-wide unique visitors.
    async fn get_total_hits(&self, unique: bool) -> Result<u32, sqlx::Error> {
        let is_unique: i8 = if unique { 1 } else { 0 };
        // Cast to UNSIGNED to avoid DECIMAL type from SUM
        let result: Option<(u64,)> = sqlx::query_as(
            "SELECT CAST(COALESCE(SUM(hitcount), 0) AS UNSIGNED) FROM hits WHERE isunique = ?"
        )
        .bind(is_unique)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(count,)| count as u32).unwrap_or(0))
    }

    /// Get all hit counts for a page (page hits, unique hits, and site totals).
    pub async fn get_hit_counts(&self, page_id: &str) -> Result<HitCounts, sqlx::Error> {
        let page_hits = self.get_hits(page_id, false).await?;
        let unique_hits = self.get_hits(page_id, true).await?;
        let total_hits = self.get_total_hits(false).await?;
        let total_unique_hits = self.get_total_hits(true).await?;

        Ok(HitCounts {
            page_hits,
            unique_hits,
            total_hits,
            total_unique_hits,
        })
    }

    /// Check if user agent belongs to a search bot
    fn is_search_bot(user_agent: &str) -> bool {
        let ua_lower = user_agent.to_lowercase();
        BOT_KEYWORDS.iter().any(|keyword| ua_lower.contains(keyword))
    }

    /// Generate privacy-preserving hash of page + IP
    fn id_hash(page_id: &str, client_ip: &str) -> String {
        // PHP does: hash("SHA256", $pageID . $visitorID)
        let mut hasher = Sha256::new();
        hasher.update(page_id.as_bytes());
        hasher.update(client_ip.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Get current unix timestamp
    fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    /// Check if this is a unique hit (not seen in last 30 days)
    async fn is_unique_hit(&self, page_id: &str, client_ip: &str) -> Result<bool, sqlx::Error> {
        let ids_hash = Self::id_hash(page_id, client_ip);

        // time column is BIGINT UNSIGNED
        let result: Option<(u64,)> = sqlx::query_as(
            "SELECT time FROM nodupes WHERE ids_hash = ?"
        )
        .bind(&ids_hash)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some((time,)) => {
                // If the recorded time is older than 30 days, count as unique
                Ok((time as i64) <= Self::now() - HIT_OLD_AFTER_SECONDS)
            }
            None => Ok(true), // Never seen before
        }
    }

    /// Log a unique hit (insert or update nodupes table)
    async fn log_hit(&self, page_id: &str, client_ip: &str) -> Result<(), sqlx::Error> {
        let ids_hash = Self::id_hash(page_id, client_ip);
        let now = Self::now();

        // Check if entry exists (time column is BIGINT UNSIGNED)
        let exists: Option<(u64,)> = sqlx::query_as(
            "SELECT time FROM nodupes WHERE ids_hash = ?"
        )
        .bind(&ids_hash)
        .fetch_optional(&self.pool)
        .await?;

        if exists.is_some() {
            sqlx::query("UPDATE nodupes SET time = ? WHERE ids_hash = ?")
                .bind(now)
                .bind(&ids_hash)
                .execute(&self.pool)
                .await?;
        } else {
            sqlx::query("INSERT INTO nodupes (ids_hash, time) VALUES (?, ?)")
                .bind(&ids_hash)
                .bind(now)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Increment hit counter for a page
    async fn count_hit(&self, page_id: &str, unique: bool) -> Result<(), sqlx::Error> {
        let is_unique: i8 = if unique { 1 } else { 0 };

        sqlx::query(
            "UPDATE hits SET hitcount = hitcount + 1 WHERE pageid = ? AND isunique = ?"
        )
        .bind(page_id)
        .bind(is_unique)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Ensure page has entries in hits table (both unique and non-unique)
    async fn create_counts_if_not_present(&self, page_id: &str) -> Result<(), sqlx::Error> {
        // Check/create non-unique entry
        let exists: Option<(String,)> = sqlx::query_as(
            "SELECT pageid FROM hits WHERE pageid = ? AND isunique = 0"
        )
        .bind(page_id)
        .fetch_optional(&self.pool)
        .await?;

        if exists.is_none() {
            sqlx::query("INSERT INTO hits (pageid, isunique, hitcount) VALUES (?, 0, 0)")
                .bind(page_id)
                .execute(&self.pool)
                .await?;
        }

        // Check/create unique entry
        let exists: Option<(String,)> = sqlx::query_as(
            "SELECT pageid FROM hits WHERE pageid = ? AND isunique = 1"
        )
        .bind(page_id)
        .fetch_optional(&self.pool)
        .await?;

        if exists.is_none() {
            sqlx::query("INSERT INTO hits (pageid, isunique, hitcount) VALUES (?, 1, 0)")
                .bind(page_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Remove old entries from nodupes table
    async fn cleanup(&self) -> Result<(), sqlx::Error> {
        let cutoff = Self::now() - HIT_OLD_AFTER_SECONDS;

        sqlx::query("DELETE FROM nodupes WHERE time < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

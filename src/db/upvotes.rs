//! Upvote System - Page voting with privacy-preserving tracking
//!
//! Allows users to upvote/downvote pages. Votes are tracked per IP
//! using SHA256(page_id + IP) to preserve privacy.
//!
//! Rate limiting: Users can only change their vote once per 24 hours.
//! After 24 hours, their vote history is cleared and they can vote again.
//!
//! Port of defuse.ca/src/libs/Upvote.php

use sha2::{Digest, Sha256};
use sqlx::MySqlPool;
use std::time::{SystemTime, UNIX_EPOCH};

/// Let IP addresses vote again after this many seconds (24 hours)
const VOTE_OLD_AFTER_SECONDS: i64 = 86400;

/// Errors that can occur during voting
#[derive(Debug)]
pub enum VoteError {
    InvalidDirection,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for VoteError {
    fn from(e: sqlx::Error) -> Self {
        VoteError::Database(e)
    }
}

impl std::fmt::Display for VoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoteError::InvalidDirection => write!(f, "Invalid vote direction"),
            VoteError::Database(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl std::error::Error for VoteError {}

/// User's current vote action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteAction {
    Upvote,
    Downvote,
}

impl VoteAction {
    fn as_str(&self) -> &'static str {
        match self {
            VoteAction::Upvote => "upvote",
            VoteAction::Downvote => "downvote",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "upvote" => Some(VoteAction::Upvote),
            "downvote" => Some(VoteAction::Downvote),
            "" => None,
            other => panic!("BUG: Invalid vote action in database: {:?}", other),
        }
    }
}

/// Vote state: aggregate counts and current user's vote.
/// Used by templates to display vote UI.
#[derive(Debug, Clone, Default)]
pub struct VoteState {
    pub upvotes: i32,
    pub downvotes: i32,
    pub user_vote: Option<VoteAction>,
}

impl VoteState {
    /// Net vote total (upvotes - downvotes)
    pub fn total(&self) -> i32 {
        self.upvotes - self.downvotes
    }

    /// Whether the current user has upvoted
    pub fn user_upvoted(&self) -> bool {
        self.user_vote == Some(VoteAction::Upvote)
    }

    /// Whether the current user has downvoted
    pub fn user_downvoted(&self) -> bool {
        self.user_vote == Some(VoteAction::Downvote)
    }
}

/// Page info for the top pages list
#[derive(Debug, Clone)]
pub struct PageVoteInfo {
    pub permanent_id: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub canonical_url: String,
    pub upvotes: i32,
    pub downvotes: i32,
}

impl PageVoteInfo {
    pub fn total(&self) -> i32 {
        self.upvotes - self.downvotes
    }
}

#[derive(Clone)]
pub struct UpvoteService {
    pool: MySqlPool,
}

impl UpvoteService {
    /// Create a new Upvote service with the given database pool
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Connect to the database and create a new service
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = MySqlPool::connect(database_url).await?;
        Ok(Self::new(pool))
    }

    /// Process a vote (up or down) for a page
    ///
    /// Accepts direction as "up" or "down" string (matching form values).
    ///
    /// Logic:
    /// - If user hasn't voted: add vote
    /// - If user voted same direction: undo vote (toggle off)
    /// - If user voted opposite direction: change vote
    pub async fn process_vote(
        &self,
        permanent_id: &str,
        client_ip: &str,
        direction: &str,
    ) -> Result<VoteState, VoteError> {
        let direction = match direction {
            "up" => VoteAction::Upvote,
            "down" => VoteAction::Downvote,
            _ => return Err(VoteError::InvalidDirection),
        };

        // Clean up old vote history first
        self.remove_old_vote_history().await?;

        let existing = self.get_user_action(permanent_id, client_ip).await?;

        match (direction, existing) {
            // Clicking up when already upvoted: undo
            (VoteAction::Upvote, Some(VoteAction::Upvote)) => {
                self.undo_upvote(permanent_id).await?;
                self.clear_user_action(permanent_id, client_ip).await?;
            }
            // Clicking up when downvoted: change to upvote
            (VoteAction::Upvote, Some(VoteAction::Downvote)) => {
                self.give_upvote(permanent_id, true).await?;
                self.set_user_action(permanent_id, client_ip, VoteAction::Upvote).await?;
            }
            // Clicking up with no vote: add upvote
            (VoteAction::Upvote, None) => {
                self.give_upvote(permanent_id, false).await?;
                self.set_user_action(permanent_id, client_ip, VoteAction::Upvote).await?;
            }
            // Clicking down when already downvoted: undo
            (VoteAction::Downvote, Some(VoteAction::Downvote)) => {
                self.undo_downvote(permanent_id).await?;
                self.clear_user_action(permanent_id, client_ip).await?;
            }
            // Clicking down when upvoted: change to downvote
            (VoteAction::Downvote, Some(VoteAction::Upvote)) => {
                self.give_downvote(permanent_id, true).await?;
                self.set_user_action(permanent_id, client_ip, VoteAction::Downvote).await?;
            }
            // Clicking down with no vote: add downvote
            (VoteAction::Downvote, None) => {
                self.give_downvote(permanent_id, false).await?;
                self.set_user_action(permanent_id, client_ip, VoteAction::Downvote).await?;
            }
        }

        // Return updated state
        Ok(self.get_vote_state(permanent_id, client_ip).await?)
    }

    /// Get current vote counts and user's vote for a page
    pub async fn get_vote_state(
        &self,
        permanent_id: &str,
        client_ip: &str,
    ) -> Result<VoteState, sqlx::Error> {
        // Clean up old history so user sees correct state
        self.remove_old_vote_history().await?;

        let upvotes = self.get_upvotes(permanent_id).await?;
        let downvotes = self.get_downvotes(permanent_id).await?;
        let user_vote = self.get_user_action(permanent_id, client_ip).await?;

        Ok(VoteState {
            upvotes,
            downvotes,
            user_vote,
        })
    }

    /// Get top voted pages for display
    pub async fn get_top_pages(
        &self,
        limit: u32,
        category: Option<&str>,
    ) -> Result<Vec<PageVoteInfo>, sqlx::Error> {
        let pages = if let Some(cat) = category {
            sqlx::query_as::<_, (String, String, String, String, String, i32, i32)>(
                "SELECT permanent_id, category, title, description, canonical_url, upvotes, downvotes
                 FROM counts
                 WHERE category = ?
                 ORDER BY (upvotes - downvotes) DESC
                 LIMIT ?"
            )
            .bind(cat)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (String, String, String, String, String, i32, i32)>(
                "SELECT permanent_id, category, title, description, canonical_url, upvotes, downvotes
                 FROM counts
                 ORDER BY (upvotes - downvotes) DESC
                 LIMIT ?"
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(pages
            .into_iter()
            .map(|(permanent_id, category, title, description, canonical_url, upvotes, downvotes)| {
                PageVoteInfo {
                    permanent_id,
                    category,
                    title,
                    description,
                    canonical_url,
                    upvotes,
                    downvotes,
                }
            })
            .collect())
    }

    /// Ensure a page exists in the counts table, creating or updating as needed
    pub async fn ensure_page(
        &self,
        permanent_id: &str,
        category: &str,
        title: &str,
        description: &str,
        canonical_url: &str,
    ) -> Result<(), sqlx::Error> {
        // Check if page exists
        let existing: Option<(String, String, String, String)> = sqlx::query_as(
            "SELECT category, title, description, canonical_url FROM counts WHERE permanent_id = ?"
        )
        .bind(permanent_id)
        .fetch_optional(&self.pool)
        .await?;

        match existing {
            Some((old_cat, old_title, old_desc, old_url)) => {
                // Update if anything changed
                if old_cat != category || old_title != title || old_desc != description || old_url != canonical_url {
                    sqlx::query(
                        "UPDATE counts SET category = ?, title = ?, description = ?, canonical_url = ?
                         WHERE permanent_id = ?"
                    )
                    .bind(category)
                    .bind(title)
                    .bind(description)
                    .bind(canonical_url)
                    .bind(permanent_id)
                    .execute(&self.pool)
                    .await?;
                }
            }
            None => {
                // Insert new page
                sqlx::query(
                    "INSERT INTO counts (permanent_id, category, title, description, canonical_url, upvotes, downvotes)
                     VALUES (?, ?, ?, ?, ?, 0, 0)"
                )
                .bind(permanent_id)
                .bind(category)
                .bind(title)
                .bind(description)
                .bind(canonical_url)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    /// Generate privacy-preserving hash of page + IP
    fn vote_hash(permanent_id: &str, client_ip: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(permanent_id.as_bytes());
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

    /// Get upvote count for a page
    async fn get_upvotes(&self, permanent_id: &str) -> Result<i32, sqlx::Error> {
        let result: Option<(i32,)> = sqlx::query_as(
            "SELECT upvotes FROM counts WHERE permanent_id = ?"
        )
        .bind(permanent_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(v,)| v).unwrap_or(0))
    }

    /// Get downvote count for a page
    async fn get_downvotes(&self, permanent_id: &str) -> Result<i32, sqlx::Error> {
        let result: Option<(i32,)> = sqlx::query_as(
            "SELECT downvotes FROM counts WHERE permanent_id = ?"
        )
        .bind(permanent_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(v,)| v).unwrap_or(0))
    }

    /// Add an upvote, optionally undoing a downvote
    async fn give_upvote(&self, permanent_id: &str, undo_downvote: bool) -> Result<(), sqlx::Error> {
        let query = if undo_downvote {
            "UPDATE counts SET upvotes = upvotes + 1, downvotes = downvotes - 1 WHERE permanent_id = ?"
        } else {
            "UPDATE counts SET upvotes = upvotes + 1 WHERE permanent_id = ?"
        };

        sqlx::query(query)
            .bind(permanent_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Undo an upvote
    async fn undo_upvote(&self, permanent_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE counts SET upvotes = upvotes - 1 WHERE permanent_id = ?")
            .bind(permanent_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Add a downvote, optionally undoing an upvote
    async fn give_downvote(&self, permanent_id: &str, undo_upvote: bool) -> Result<(), sqlx::Error> {
        let query = if undo_upvote {
            "UPDATE counts SET downvotes = downvotes + 1, upvotes = upvotes - 1 WHERE permanent_id = ?"
        } else {
            "UPDATE counts SET downvotes = downvotes + 1 WHERE permanent_id = ?"
        };

        sqlx::query(query)
            .bind(permanent_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Undo a downvote
    async fn undo_downvote(&self, permanent_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE counts SET downvotes = downvotes - 1 WHERE permanent_id = ?")
            .bind(permanent_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get user's current vote action (if any, and not expired)
    async fn get_user_action(
        &self,
        permanent_id: &str,
        client_ip: &str,
    ) -> Result<Option<VoteAction>, sqlx::Error> {
        let hash = Self::vote_hash(permanent_id, client_ip);

        let result: Option<(String,)> = sqlx::query_as(
            "SELECT action FROM history WHERE hash = ?"
        )
        .bind(&hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|(action,)| VoteAction::from_str(&action)))
    }

    /// Set user's vote action
    async fn set_user_action(
        &self,
        permanent_id: &str,
        client_ip: &str,
        action: VoteAction,
    ) -> Result<(), sqlx::Error> {
        let hash = Self::vote_hash(permanent_id, client_ip);
        let now = Self::now();

        // Check if entry exists
        let exists: Option<(String,)> = sqlx::query_as(
            "SELECT hash FROM history WHERE hash = ?"
        )
        .bind(&hash)
        .fetch_optional(&self.pool)
        .await?;

        if exists.is_some() {
            sqlx::query("UPDATE history SET action = ?, time_added = ? WHERE hash = ?")
                .bind(action.as_str())
                .bind(now)
                .bind(&hash)
                .execute(&self.pool)
                .await?;
        } else {
            sqlx::query("INSERT INTO history (hash, action, time_added) VALUES (?, ?, ?)")
                .bind(&hash)
                .bind(action.as_str())
                .bind(now)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Clear user's vote action (when they undo their vote)
    async fn clear_user_action(
        &self,
        permanent_id: &str,
        client_ip: &str,
    ) -> Result<(), sqlx::Error> {
        let hash = Self::vote_hash(permanent_id, client_ip);

        // PHP sets action to empty string, we'll delete the row
        sqlx::query("DELETE FROM history WHERE hash = ?")
            .bind(&hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Remove old vote history entries (> 24 hours)
    async fn remove_old_vote_history(&self) -> Result<(), sqlx::Error> {
        let cutoff = Self::now() - VOTE_OLD_AFTER_SECONDS;

        sqlx::query("DELETE FROM history WHERE time_added < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

//! Upvote System - Page voting with privacy-preserving tracking
//!
//! Allows users to upvote/downvote pages. Votes are tracked per IP
//! using SHA256(page_id + IP) to preserve privacy.
//!
//! Vote memory: Each vote is remembered for 24 hours, during which the
//! user can toggle it off or switch direction. After 24 hours, the vote
//! history is cleared and they can vote on the same page again.
//!
//! Port of defuse.ca/src/libs/Upvote.php

use sha2::{Digest, Sha256};
use sqlx::MySqlPool;
use std::time::{SystemTime, UNIX_EPOCH};

use super::util::{html_escape, js_escape};

/// Let IP addresses vote again after this many seconds (24 hours)
const VOTE_OLD_AFTER_SECONDS: i64 = 86400;

/// Errors that can occur during voting
#[derive(Debug)]
pub enum VoteError {
    InvalidDirection,
    InvalidPermanentId,
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
            VoteError::InvalidPermanentId => write!(f, "Invalid permanent ID"),
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
            other => {
                tracing::error!("Invalid vote action in database: {:?}", other);
                None
            }
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
    // TODO: Make all-pages break down by category at some point in the future
    #[allow(dead_code)]
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
        if !crate::registry::is_valid_upvote_id(permanent_id) {
            return Err(VoteError::InvalidPermanentId);
        }

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

        let hash = Self::vote_hash(permanent_id, client_ip);

        // Single query to get counts and user's action
        // LIMIT 1 on counts subqueries tolerates duplicate rows (no UNIQUE constraint)
        let result: (Option<i32>, Option<i32>, Option<String>) = sqlx::query_as(
            "SELECT
                (SELECT upvotes FROM counts WHERE permanent_id = ? LIMIT 1),
                (SELECT downvotes FROM counts WHERE permanent_id = ? LIMIT 1),
                (SELECT action FROM history WHERE hash = ?)"
        )
        .bind(permanent_id)
        .bind(permanent_id)
        .bind(&hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(VoteState {
            upvotes: result.0.unwrap_or(0),
            downvotes: result.1.unwrap_or(0),
            user_vote: result.2.and_then(|s| VoteAction::from_str(&s)),
        })
    }

    /// Get top voted pages for display
    pub async fn get_top_pages(
        &self,
        limit: Option<u32>,
        category: Option<&str>,
    ) -> Result<Vec<PageVoteInfo>, sqlx::Error> {
        let base = "SELECT permanent_id, category, title, description, canonical_url, upvotes, downvotes FROM counts";
        let query = match (category, limit) {
            (Some(_), Some(_)) => format!("{base} WHERE category = ? ORDER BY (upvotes - downvotes) DESC LIMIT ?"),
            (Some(_), None)    => format!("{base} WHERE category = ? ORDER BY (upvotes - downvotes) DESC"),
            (None, Some(_))    => format!("{base} ORDER BY (upvotes - downvotes) DESC LIMIT ?"),
            (None, None)       => format!("{base} ORDER BY (upvotes - downvotes) DESC"),
        };

        let mut q = sqlx::query_as::<_, (String, String, String, String, String, i32, i32)>(&query);
        if let Some(cat) = category {
            q = q.bind(cat);
        }
        if let Some(lim) = limit {
            q = q.bind(lim);
        }
        let pages = q.fetch_all(&self.pool).await?;

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

    /// Get all pages, optionally filtered by category
    pub async fn get_all_pages(
        &self,
        category: Option<&str>,
    ) -> Result<Vec<PageVoteInfo>, sqlx::Error> {
        self.get_top_pages(None, category).await
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
        // Atomic insert-if-not-exists to avoid race condition creating duplicate rows
        // (the counts table has no UNIQUE constraint on permanent_id)
        let result = sqlx::query(
            "INSERT INTO counts (permanent_id, category, title, description, canonical_url, upvotes, downvotes)
             SELECT ?, ?, ?, ?, ?, 0, 0
             FROM DUAL
             WHERE NOT EXISTS (SELECT 1 FROM counts WHERE permanent_id = ?)"
        )
        .bind(permanent_id)
        .bind(category)
        .bind(title)
        .bind(description)
        .bind(canonical_url)
        .bind(permanent_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            // Row already exists, update metadata
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

        Ok(())
    }

    /// Delete a page from the upvote system by its permanent_id.
    /// Removes the page from the counts table and its vote history.
    async fn delete_page(&self, permanent_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM counts WHERE permanent_id = ?")
            .bind(permanent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Sync all registered pages with upvote configs to the database.
    /// Call at startup to ensure categories and metadata are up-to-date.
    pub async fn sync_all_pages(&self) -> Result<(), sqlx::Error> {
        use crate::registry::PAGE_REGISTRY;

        // Remove pages that have been removed from the upvote system.
        let removed_ids = ["pphos", "writing_tips", "auditencfsold"];
        for id in removed_ids {
            self.delete_page(id).await?;
        }

        for page in PAGE_REGISTRY.values() {
            if let Some(ref upvote) = page.upvote {
                let title = upvote.title.unwrap_or(page.title_or_default());
                let description = upvote.description.unwrap_or(page.description_or_default());
                let url = page.relative_url();
                self.ensure_page(upvote.id, upvote.category, title, description, &url).await?;
            }
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // HTML Rendering (matches PHP's Upvote::render_list)
    // -------------------------------------------------------------------------

    /// Get user vote actions for multiple pages at once.
    /// Returns a map from permanent_id to the user's vote action.
    pub async fn get_user_actions_batch(
        &self,
        pages: &[PageVoteInfo],
        client_ip: &str,
    ) -> Result<std::collections::HashMap<String, Option<VoteAction>>, sqlx::Error> {
        use std::collections::HashMap;

        let mut result = HashMap::new();
        for page in pages {
            let action = self.get_user_action(&page.permanent_id, client_ip).await?;
            result.insert(page.permanent_id.clone(), action);
        }
        Ok(result)
    }

    /// Render a table of page links with upvote arrows (synchronous version).
    /// Matches PHP's Upvote::render_list() output exactly.
    ///
    /// # Arguments
    /// * `pages` - List of pages to render
    /// * `page_url` - URL for form actions (current page's canonical URL)
    /// * `user_actions` - Pre-fetched map of permanent_id -> user's vote action
    pub fn render_list(
        pages: &[PageVoteInfo],
        page_url: &str,
        user_actions: &std::collections::HashMap<String, Option<VoteAction>>,
    ) -> String {
        let mut html = String::new();
        html.push_str("<table class=\"upvote_pagelist\">");

        for (i, page) in pages.iter().enumerate() {
            let user_action = user_actions
                .get(&page.permanent_id)
                .copied()
                .flatten();

            if i == 0 {
                // First row: 12 spaces before <tr>, same line as table
                html.push_str("            <tr>\n");
            } else {
                // Subsequent rows: 20 spaces before <tr>
                html.push_str("                    <tr>\n");
            }

            // Render the row content
            Self::render_list_row(&mut html, page, page_url, user_action);

            // Close the row with 12 spaces
            html.push_str("            </tr>\n");
        }

        // Close table with 8 spaces
        html.push_str("        </table>");
        html
    }

    /// Render a single row of the upvote list table.
    fn render_list_row(
        html: &mut String,
        page: &PageVoteInfo,
        page_url: &str,
        user_action: Option<VoteAction>,
    ) {
        let safe_title = html_escape(&page.title);
        let safe_description = html_escape(&page.description);
        let safe_url = html_escape(&page.canonical_url);

        // Arrow cell
        html.push_str("                <td class=\"upvote_list_arrowcell\">\n");
        Self::render_arrows_in_list(html, page, page_url, user_action);
        html.push_str("</td>\n");

        // Title cell
        html.push_str("                <td class=\"upvote_list_titlecell\">\n");
        html.push_str(&format!(
            "                    <a class=\"upvote_list_title\" href=\"{}\">\n",
            safe_url
        ));
        html.push_str(&format!(
            "                        {}                    </a>\n",
            safe_title
        ));
        html.push_str("                    <div class=\"upvote_list_desc\">\n");
        html.push_str(&format!(
            "                        {}                    </div>\n",
            safe_description
        ));
        html.push_str("                </td>\n");
    }

    /// Render the upvote arrows for list view (up arrow, count, down arrow).
    fn render_arrows_in_list(
        html: &mut String,
        page: &PageVoteInfo,
        page_url: &str,
        user_action: Option<VoteAction>,
    ) {
        html.push_str("                                <div class=\"upvotearrowsinlist\">\n");
        Self::render_uparrow(html, &page.permanent_id, page_url, user_action);
        Self::render_count(html, &page.permanent_id, page.total(), user_action);
        Self::render_downarrow(html, &page.permanent_id, page_url, user_action);
        html.push_str("                </div>\n");
        html.push_str("                        ");
    }

    /// Render the up arrow form.
    fn render_uparrow(
        html: &mut String,
        permanent_id: &str,
        page_url: &str,
        user_action: Option<VoteAction>,
    ) {
        // TODO: safe_id is HTML-escaped but used directly in CSS class names and JS
        // identifiers. This is safe because permanent_id comes from the hardcoded page
        // registry. If the upvote system is ever extended to accept untrusted IDs,
        // these concatenations would need proper CSS/JS identifier sanitization.
        let safe_id = html_escape(permanent_id);
        let js_id = js_escape(permanent_id);
        let up_form_name = format!("upvoteUpForm{}", safe_id);
        let up_image_name = format!("upvoteUpImage{}", safe_id);

        let is_selected = user_action == Some(VoteAction::Upvote);
        let image = if is_selected {
            "/images/upvote-selected.gif"
        } else {
            "/images/upvote.gif"
        };
        // Note: PHP adds a trailing space after alt="Upvote" when selected
        let alt_suffix = if is_selected { " " } else { "" };

        html.push_str("                    <div class=\"upvoteuparrow\">\n");
        html.push_str(&format!(
            "            <form \n                action=\"{}\" \n                method=\"post\"\n                onsubmit=\"return upvote.submit('{}', 'up')\"\n                class=\"upvoteform {}\"\n            >\n",
            html_escape(page_url),
            js_id,
            up_form_name
        ));
        html.push_str(
            "                <input type=\"hidden\" name=\"upvotes_direction\" value=\"up\" />\n",
        );
        html.push_str(&format!(
            "                <input type=\"hidden\" name=\"upvotes_id\" value=\"{}\" />\n",
            safe_id
        ));
        html.push_str(&format!(
            "                                    <input\n                        type=\"image\" src=\"{}\" alt=\"Upvote\"{}\n                        name=\"{}\"\n                    />\n",
            image, alt_suffix, up_image_name
        ));
        html.push_str("                            </form>\n");
        html.push_str("        </div>\n");
    }

    /// Render the vote count display.
    fn render_count(
        html: &mut String,
        permanent_id: &str,
        total: i32,
        user_action: Option<VoteAction>,
    ) {
        let safe_id = html_escape(permanent_id);
        let counter_name = format!("upvoteCounter{}", safe_id);

        let count_class = match user_action {
            Some(VoteAction::Upvote) => format!("upvotecount_upvoted {}", counter_name),
            Some(VoteAction::Downvote) => format!("upvotecount_downvoted {}", counter_name),
            None => format!("upvotecount {}", counter_name),
        };

        html.push_str(&format!(
            "            <div class=\"{}\" >\n            {} \n        </div>\n",
            count_class, total
        ));
    }

    /// Render the down arrow form.
    fn render_downarrow(
        html: &mut String,
        permanent_id: &str,
        page_url: &str,
        user_action: Option<VoteAction>,
    ) {
        let safe_id = html_escape(permanent_id);
        let js_id = js_escape(permanent_id);
        let down_form_name = format!("upvoteDownForm{}", safe_id);
        let down_image_name = format!("upvoteDownImage{}", safe_id);

        let is_selected = user_action == Some(VoteAction::Downvote);
        let image = if is_selected {
            "/images/downvote-selected.gif"
        } else {
            "/images/downvote.gif"
        };

        html.push_str("            <div class=\"upvotedownarrow\">\n");
        html.push_str(&format!(
            "            <form \n                action=\"{}\" \n                method=\"post\"\n                onsubmit=\"return upvote.submit('{}', 'down')\"\n                class=\"upvoteform {}\"\n            >\n",
            html_escape(page_url),
            js_id,
            down_form_name
        ));
        html.push_str(
            "                <input type=\"hidden\" name=\"upvotes_direction\" value=\"down\" />\n",
        );
        html.push_str(&format!(
            "                <input type=\"hidden\" name=\"upvotes_id\" value=\"{}\" />\n",
            safe_id
        ));
        html.push_str(&format!(
            "                                    <input \n                        type=\"image\" src=\"{}\" alt=\"Downvote\"\n                        name=\"{}\"\n                    />\n",
            image, down_image_name
        ));
        html.push_str("                            </form>\n");
        html.push_str("        </div>\n");
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

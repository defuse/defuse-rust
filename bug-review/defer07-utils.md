# Bug Review 07: Vim Highlight & Utilities

Reviewer: Claude Opus 4.6
Date: 2026-02-18
Scope: vim_highlight.rs, util.rs, html_escape.rs, special_endpoints.rs, phpcount.rs, upvotes.rs, bibliography.rs

---

## ISSUE: Upvote `set_user_action` has SELECT-then-INSERT race condition

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/upvotes.rs`, lines 644-678
**Severity:** Low (would cause duplicate key error)

`set_user_action` does a SELECT to check if a row exists, then either UPDATEs or INSERTs. Two concurrent votes from the same user for different pages could race, causing both to attempt INSERT with the same hash, resulting in a duplicate key error.

```rust
let exists: Option<(String,)> = sqlx::query_as(
    "SELECT hash FROM history WHERE hash = ?"
).bind(&hash).fetch_optional(&self.pool).await?;

if exists.is_some() {
    // UPDATE
} else {
    // INSERT -- can fail with duplicate key if concurrent request also INSERTs
}
```

**Fix:** Use `INSERT ... ON DUPLICATE KEY UPDATE action = ?, time_added = ?` (same pattern already used correctly in `phpcount.rs::log_hit`).

---

## ISSUE: PHPCount `create_counts_if_not_present` race condition can create duplicate rows

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/phpcount.rs`, lines 224-258
**Severity:** Low (already documented with TODO, causes over-counting in site totals)

The code already has TODO comments acknowledging this. Two concurrent requests for a never-before-seen page can both see `exists.is_none()` and both INSERT, creating duplicate rows. The `get_hit_counts` method uses `LIMIT 1` for per-page queries (tolerant of duplicates), but the site-wide totals use `SUM` across all rows, which will over-count if duplicates exist.

**Fix:** Use `INSERT IGNORE` or `INSERT ... ON DUPLICATE KEY UPDATE pageid = pageid` (no-op update) to make the insert idempotent. Requires a UNIQUE constraint on `(pageid, isunique)`.

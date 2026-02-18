# Bug Review 07: Vim Highlight & Utilities

Reviewer: Claude Opus 4.6
Date: 2026-02-18
Scope: vim_highlight.rs, util.rs, html_escape.rs, special_endpoints.rs, phpcount.rs, upvotes.rs, bibliography.rs

---

## BUG: Special endpoints return proxy IP, not client IP

**Files:** `/home/taylor/defuse-rewrite/defuse-rust/src/special_endpoints.rs`
**Severity:** Medium (functional bug in production)

All three IP-returning endpoints (`ip_php`, `ip_insecure_php`, `getmyip_php`) use raw `ConnectInfo<SocketAddr>` directly, never calling `util::client_ip()`. In production behind a reverse proxy, these endpoints will always return the proxy's IP address (e.g., `127.0.0.1`) instead of the real client IP.

Every other handler in the codebase (registered_page_handler.rs, upvote.rs, middleware/upvote_post.rs) correctly uses `util::client_ip(connection_ip, headers)` to extract the real IP from `X-Forwarded-For`. These three endpoints are the only ones that skip this.

```rust
// special_endpoints.rs line 25 - returns proxy IP in production
pub async fn ip_php(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    let ip = addr.ip().to_string();  // BUG: should use client_ip()
```

**Fix:** Accept `HeaderMap` and call `client_ip(addr.ip(), &headers)` in all three handlers, matching the pattern used elsewhere.

---

## ISSUE: Upvote vote counts can go negative

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/upvotes.rs`
**Severity:** Low (data integrity, requires race condition)

The `undo_upvote`, `undo_downvote`, `give_upvote(undo_downvote=true)`, and `give_downvote(undo_upvote=true)` methods decrement counters with `upvotes - 1` / `downvotes - 1` without any floor constraint. If vote history is deleted (by the 24-hour cleanup in `remove_old_vote_history`) between `get_user_action` and the actual count update, or if the same user sends concurrent identical requests before history is recorded, counts can drift below zero.

Concrete scenario:
1. User upvotes page (upvotes=1, history records "upvote")
2. 24 hours pass, `remove_old_vote_history` deletes the history row
3. User clicks upvote again; `get_user_action` returns `None` (history deleted), so `give_upvote(false)` is called (upvotes=2)
4. But the count already included their previous vote -- the system cannot distinguish a returning user from a new one

This is an inherent limitation of the 24-hour history window design (same as the PHP version), but the negative-count scenario is more concerning:
1. Two concurrent "undo upvote" requests both read history as "upvote"
2. Both call `undo_upvote` which does `upvotes = upvotes - 1`
3. Result: upvotes decremented twice, going to -1

**Mitigation:** Add `WHERE upvotes > 0` / `WHERE downvotes > 0` guards to the decrement queries, or use `GREATEST(upvotes - 1, 0)`.

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

---

## NOT A BUG: `util::html_escape` uses `&#x27;` for apostrophes

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/util.rs`, line 58
**Severity:** None

The project memory notes concern about `&#x27;` vs PHP's `&#039;`. After review, this is NOT a bug:

- `util::html_escape` (uses `&#x27;`) is used for security escaping in attribute values, URLs, and rendered HTML fragments (special_endpoints, bibliography, upvotes render, vim_highlight cache info, pastebin_view).
- `html_escape::html_special_chars` (uses `&#039;`) is used inside `escape_text()` for visual text display in the pastebin/code viewer, where matching PHP's `htmlspecialchars` output exactly matters for test compatibility.

Both `&#x27;` and `&#039;` are valid HTML escapes for single quotes. They are functionally identical for XSS prevention. The two modules serve different purposes and their distinct escaping is correct. Askama's default escaper also uses `&#x27;`, so `util::html_escape` is consistent with template output.

---

## NOT A BUG (in practice): `js_escape` multi-byte character handling

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/util.rs`, lines 180-194
**Severity:** Informational (documented TODO, not exploitable)

The TODO warns that multi-byte characters are not properly handled. The function escapes them by iterating over individual UTF-8 bytes and emitting `\xHH` for each byte. For example, the character U+00E9 (two UTF-8 bytes) becomes `\xC3\xA9` instead of `\u00E9`.

This is actually safe from XSS because JavaScript string literals with `\xHH` byte sequences will reconstruct the original bytes. The JS engine interprets `\xC3\xA9` in a single-quoted string as the two-character Latin-1 string, which may display differently than the original UTF-8 character depending on context. However, it will never produce executable JavaScript.

The function is only called from `upvotes.rs::render_uparrow` and `render_downarrow`, where it escapes `permanent_id` values. These are static alphanumeric strings from the page registry, so multi-byte characters will never actually appear in practice. The TODO is technically correct but does not represent a real-world issue.

---

## NOT A BUG: Vim highlight command injection

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/vim_highlight.rs`
**Severity:** None

Reviewed for three injection vectors:

1. **Shell injection via `Command::new().args()`**: Not possible. Rust's `Command` does not use a shell; arguments are passed directly to `execvp`. No shell metacharacter interpretation occurs.

2. **Vim command injection via `file_type` or `color_scheme`**: The `file_type` field is interpolated into vim's `-c "set filetype=..."` command (line 241). If a malicious value like `ruby | !rm -rf /` were provided, it would be passed as a single `-c` argument to vim. However, all callers use hardcoded string literals ("ruby", "text", "python", "c", "php"). The `file_type` is never set from user input. Same for `color_scheme` (always "default").

3. **Cache poisoning**: Cache file paths are based on MD5 of content (for strings) or MD5 of canonical path (for files). Settings are validated on cache read via `encode_info()`/`extract_info()`. Different settings for the same content share a cache path, causing unnecessary re-computation but not incorrect results -- the settings validation prevents serving stale output.

---

## NOT A BUG: XSS in special endpoints

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/special_endpoints.rs`
**Severity:** None

All three HTML-rendering endpoints properly escape user-controlled data:

- `ip_insecure_php`: Escapes IP address with `html_escape()` (line 40)
- `getmyip_php`: Escapes IP, hostname, and user-agent with `html_escape()` (lines 107-109)
- `shout_php`: Escapes decoded base64 text with `html_escape()` (line 160)

The form in `shout_php` uses a hardcoded action URL (`action="s.php"`), and the `?e=` parameter redirects via URL encoding. No injection vectors found.

---

## INFORMATIONAL: PHPCount cleanup runs on every request

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/phpcount.rs`, line 74
**Severity:** Informational (performance, matches PHP behavior)

`add_hit()` calls `self.cleanup().await?` on every page hit, executing `DELETE FROM nodupes WHERE time < ?` on every request. Under high traffic, this means every page view triggers a DELETE scan on the nodupes table.

The PHP version has the same behavior. For a low-traffic personal site this is fine, but it would be more efficient to run cleanup periodically (e.g., with a background task or probabilistic trigger like `if rand() < 0.01`). This matches the original PHP behavior and is not a regression.

---

## INFORMATIONAL: Upvotes cleanup runs on every vote AND every page view

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/upvotes.rs`, lines 163, 211
**Severity:** Informational (performance)

`remove_old_vote_history()` is called both in `process_vote()` (line 163) and `get_vote_state()` (line 211). Since `process_vote` calls `get_vote_state` at the end (line 201), every vote triggers TWO cleanup deletes. Additionally, every page view that displays vote state also triggers cleanup. Same observation as PHPCount above -- matches PHP behavior but could be optimized.

---

## Summary

| Finding | Severity | Action Needed |
|---------|----------|---------------|
| Special endpoints return proxy IP | Medium | Fix: use `client_ip()` |
| Upvote counts can go negative | Low | Fix: add floor guards |
| Upvote `set_user_action` race | Low | Fix: use UPSERT |
| PHPCount duplicate row race | Low | Fix: use INSERT IGNORE |
| `util::html_escape` apostrophe format | None | No action |
| `js_escape` multi-byte handling | None | No action |
| Vim command injection | None | No action |
| XSS in special endpoints | None | No action |

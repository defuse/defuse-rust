# Final Pre-Deployment Security Review: Data & File Handling

Reviewed: 2026-02-19
Scope: File serving / path traversal, database operations, file upload handling, time capsule, context/IP handling

## Summary

No showstopper or critical security issues were found. The codebase demonstrates careful security practices across all reviewed areas. Details of each area follow.

---

## 1. File Serving / Path Traversal

**Status: CLEAN**

All file serving uses `tower_http::services::ServeDir`, which has built-in path traversal protection (normalizes `..` components and rejects escapes before resolving to the filesystem). The routes in `src/storage_routes.rs` correctly scope each `ServeDir` to a specific subdirectory under `extras/`:

- `/files` -> `extras/files`
- `/files2` -> `extras/files2`
- `/mirrors` -> `extras/mirrors`
- `/upload` -> `extras/upload`

The comment on line 26 explicitly notes that `/storage` itself (which contains credentials) must never be served -- only `extras/` subdirectories are exposed.

Static files are served via `ServeDir::new("static")` in `main.rs`, also with built-in traversal protection.

The `vim_highlight.rs` module has two file-serving functions (`highlight_file` and `highlight_storage_file`) but both are only called with hardcoded paths from page handlers, never with user input. The `file_type` and `color_scheme` fields that get interpolated into vim commands are documented as requiring hardcoded literals and are never populated from user input.

The blog slug redirect in `url_canonicalization.rs` (`check_blog_slug_redirect`) properly uses `canonicalize()` + `starts_with(static_dir)` to prevent path traversal, and the early check for `.` in the path segment prevents traversal attempts like `/blog/../../etc/passwd`.

## 2. Database Operations (SQL Injection)

**Status: CLEAN**

All SQL queries across the reviewed modules use parameterized queries via sqlx's `?` bind placeholders:

- **`src/libs/pastebin.rs`**: All 5 queries use `?` placeholders with `.bind()`. No string interpolation into SQL.
- **`src/libs/trent.rs`**: All 4 queries use `?` placeholders. No string interpolation into SQL.
- **`src/libs/upvotes.rs`**: All queries use `?` placeholders. The `get_top_pages` method uses `format!` to build query structure (choosing between 4 fixed query variants based on `Option<category>` and `Option<limit>`), but the actual values are always bound via `?` -- no user input is interpolated into SQL strings. The `category` parameter is never sourced from user input in practice (always `None` from callers).
- **`src/libs/phpcount.rs`**: All queries use `?` placeholders. No string interpolation into SQL.
- **`src/libs/timecapsule.rs`**: All 4 queries use `?` placeholders. No string interpolation into SQL.

## 3. File Upload Handling

**Status: CLEAN**

Two areas handle file uploads:

**Checksums page (`src/pages/services/checksums.rs`)**:
- File data is read into memory from the multipart stream (via `registered_page_handler.rs` which caps total body at 100 MB).
- A 5 MB per-file limit is enforced at line 172.
- File data is only used as input to hash functions -- never written to disk, never used as a filename, never executed.
- No temp file cleanup needed (data stays in memory only).

**TRENT page (`src/pages/services/trent.rs`)**:
- Uploaded files are validated: 10 MB size limit, content hash verification, Latin-1 encoding check.
- Temp file paths use `format!("/tmp/trent-{}-{}", drawing_num, hash)` where `drawing_num` is an `i32` (from DB auto-increment) and `hash` is validated as exactly 64 hex characters via `trent::is_sha256_hex()`. The assertion on line 528 provides defense-in-depth.
- File content from uploads never flows into filenames -- the user-provided filename is only used for display (`build_file_infos`), HTML-escaped via Askama templating.
- Temp files are cleaned up after drawing completion (line 397). On error they are intentionally left for retry (documented in comment).

**Body size limits**: The `registered_page_handler.rs` enforces a 100 MB body limit for all POST requests (line 123). The `/bin/add.php` route has an explicit `DefaultBodyLimit::max(100 * 1024 * 1024)` layer. Caddy is documented as the primary defense for oversized bodies.

## 4. Time Capsule

**Status: CLEAN**

The time capsule (`src/libs/timecapsule.rs` + `src/pages/services/quantum_computer_time_capsule.rs`):

- All database queries use parameterized `?` placeholders.
- Input validation at lines 116-132 ensures all saved fields (algorithm, keys, ciphertext) contain only printable ASCII without spaces (`b > 0x20 && b < 0x7F`), preventing control characters and newlines from corrupting the one-message-per-line archive format.
- Message size is capped at 200,000 bytes (line 149).
- reCAPTCHA verification is required before database writes (line 165).
- The archive download handler (`download_archive`) reads from hardcoded static file paths (`static/timecapsule/...`) and database entries -- no user input flows into file paths.
- HTML escaping is applied to displayed content via `html_escape()`.

No archive extraction or zip handling exists -- the "archive" is a plain text file generated from database entries. No path traversal risk.

## 5. Context / IP Handling

**Status: CLEAN**

The `client_ip` function in `src/libs/util.rs` implements proper trusted-proxy validation:

- `TRUSTED_PROXIES` is hardcoded to `127.0.0.1` and `::1` only.
- `X-Forwarded-For` and `X-Real-IP` headers are ONLY trusted when the TCP connection comes from a trusted proxy IP (lines 21-43).
- Direct connections (non-proxy) always use the actual connection IP, ignoring any forwarding headers.
- The `is_https` function applies the same trusted-proxy check for `X-Forwarded-Proto`.

The IP is used for:
- Hit counting (privacy hash: `SHA256(page_id + IP)`) -- spoofing only affects analytics, not security.
- Vote deduplication (privacy hash: `SHA256(permanent_id + IP)`) -- spoofing only allows re-voting, not privilege escalation.
- reCAPTCHA verification (`remoteip` parameter) -- Google uses this for risk scoring, not hard enforcement.
- Logging (debug level).

None of these uses create a security bypass if the IP were somehow spoofed. The architecture of running behind a reverse proxy on localhost with only localhost as a trusted proxy is correct.

---

## Conclusion

No showstopper or critical security issues were identified. The codebase is clean for deployment in the reviewed areas. Key security properties verified:

1. Path traversal is prevented by tower_http's ServeDir and explicit path validation where manual file operations occur.
2. All SQL queries use parameterized placeholders -- no SQL injection vectors exist.
3. File uploads are size-limited, content-validated, and stored with safe filenames derived from hashes.
4. The time capsule enforces strict ASCII-only input validation and uses reCAPTCHA gating.
5. IP header trust is properly scoped to connections from localhost only.

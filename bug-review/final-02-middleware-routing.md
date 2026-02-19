# Final Pre-Deployment Security Review: Middleware & Routing

**Reviewer:** Claude Opus 4.6
**Date:** 2026-02-19
**Scope:** URL canonicalization, security headers, blocking middleware, upvote POST middleware, main router, registered page handler, storage routes, special endpoints

## Summary

No showstopper security bugs were found in the reviewed areas. The codebase demonstrates careful security engineering with explicit mitigations for the common vulnerability classes (open redirects, header injection, path traversal, IP spoofing, CSRF, DNS rebinding). Below is a detailed walkthrough of the analysis for each area.

---

## Area-by-Area Analysis

### 1. URL Canonicalization Middleware (`src/middleware/url_canonicalization.rs`)

**Open Redirect:** Not vulnerable. Step 1 (host canonicalization) ensures the host is either `MASTER_HOST` ("defuse.ca") or a dev host before Step 3 uses `&host` in `build_redirect_url`. The comment at line 143-144 correctly documents this invariant. An attacker sending `Host: evil.com` would be caught by Step 1 and redirected to `defuse.ca`, never reaching Step 3.

**Header Injection (CRLF):** Not vulnerable. Axum's HTTP parser rejects CRLFs in URI paths. `HeaderValue` construction would also reject CRLF in Location headers.

**Empty Host:** Handled at line 120-125 -- returns 400 Bad Request, preventing malformed redirects like `https:///path`.

**Blog Slug Oracle (line 217-231):** The `check_blog_slug_redirect` function does a filesystem existence check, which could theoretically be a timing oracle for file existence. However, the guard at line 208 (`after_blog.contains('.')`) prevents path traversal (no `..` possible), and `canonicalize()` + `starts_with()` adds belt-and-suspenders protection. The oracle is limited to files in `static/blog/` ending in `.html`, which are all public static content anyway. Not a concern.

### 2. Security Headers Middleware (`src/middleware/security_headers.rs`)

**Headers present and correct:**
- `X-Frame-Options: SAMEORIGIN` -- present
- `X-Content-Type-Options: nosniff` -- present
- `Referrer-Policy: strict-origin-when-cross-origin` -- present, protects paste URLs from leaking cross-origin
- `Strict-Transport-Security` -- present with `includeSubDomains; preload`, correctly gated on `is_https && !is_dev`
- `Cache-Control: no-cache, no-store, must-revalidate` -- applied to pages marked `no_cache` (e.g., passgen)
- `Content-Disposition: attachment` -- correctly applied to `/files/`, `/mirrors/`, `/upload/` but not `/files2/`

**Missing CSP:** There is no `Content-Security-Policy` header. This is worth noting but is not a showstopper for a site that already uses Askama's auto-escaping and `html_escape()` consistently. Adding CSP would be a defense-in-depth improvement for a future iteration.

### 3. Blocking Middleware (`src/middleware/blocking.rs`)

This middleware moves request handling to `spawn_blocking` for CPU preemption. No security concerns -- it is a DoS mitigation, not a security gate. The `expect()` on line 29 will panic (and be caught by `CatchPanicLayer`) if a handler panics, which is the correct behavior.

### 4. Upvote POST Middleware (`src/middleware/upvote_post.rs`)

**CSRF:** Protected via `csrf::check_origin()` at line 110, which validates the `Origin` (or `Referer`) header matches an accepted host. The CSRF check also guards against DNS rebinding by verifying the request Host is an accepted host (line 80 of csrf.rs).

**Open Redirect in POST redirect:** Not vulnerable. `redirect_url` (line 75) is `request.uri().to_string()`, which in HTTP/1.1 is just the path+query (e.g., `/about.htm`). The app runs behind Caddy over HTTP/1.1, so this will always be a relative URL.

**Body size:** Guarded by Content-Length check at line 61 (>100KB skips upvote processing) and a 10MB hard limit at line 89 for requests without Content-Length. Reasonable.

**`process_vote` panic at line 118-119:** The `.expect()` will panic if the database operation fails. `CatchPanicLayer` in the middleware stack will convert this to a 500 response. This is an availability concern (noisy), but not a security bug.

### 5. Main Router Setup (`src/main.rs`)

**Middleware ordering:** Correct. From outermost to innermost: `CatchPanicLayer` -> `SecurityHeadersLayer` -> `UrlCanonicalizationLayer` -> upvote POST -> `blocking_middleware`. Security headers wrap everything. URL canonicalization runs before page dispatch. Panic catching is outermost.

**Route ordering:** Static files (via `ServeDir`) take precedence over the registered page handler (which is the fallback). Explicit routes (`/bin/add.php`, `/ip.php`, etc.) take precedence over both. This is correct.

**Body limit on `/bin/add.php`:** 100 MB at the Axum layer, with the handler enforcing 50 MB. The comment explains this allows the handler to return a useful error message rather than a connection reset. Caddy is also noted as responsible for a 100 MB limit (line 157).

### 6. Registered Page Handler (`src/registered_page_handler.rs`)

**Path Traversal:** Not vulnerable. `resolve_path()` only matches pages registered in `PAGE_REGISTRY` (a static compile-time map). Arbitrary filesystem paths cannot be reached through this handler.

**Injection via `query_string` or `url_prefix`:** The `query_string` is passed through to page handlers but is not used in any SQL queries (those use parameterized queries via sqlx). The `url_prefix` is built from `scheme` (hardcoded "http"/"https") and the validated `host`.

**`X-Captcha-Bypass` header:** Extracted and passed through to handlers, but the recaptcha module validates it against a SHA-256 hash of a 256-bit random secret (line 9 of recaptcha.rs). Brute-forcing the preimage is infeasible. Even if the reverse proxy doesn't strip this header, an attacker would need to know the secret.

**Redirect panic at line 99:** If a redirect result reaches the dispatcher (meaning middleware failed), this panics. Caught by `CatchPanicLayer`. This is a correctness assertion, not a security issue.

### 7. Storage Routes (`src/storage_routes.rs`)

**Path Traversal:** Not vulnerable. `ServeDir` (tower-http) sanitizes paths internally, rejecting `..` traversals. The storage router only serves from `extras/files`, `extras/files2`, `extras/mirrors`, `extras/upload` -- never from the `storage/` root (which contains credentials, per comment on line 26).

**404 Handling:** Uses `not_found_handler` (dedicated 404 renderer) instead of the full page dispatcher. This was an explicit fix for the known bug where `nest_service` prefix stripping could cause path collisions with registered page slugs.

### 8. Special Endpoints (`src/special_endpoints.rs`)

**XSS:** All user-controlled output (`ip`, `hostname`, `user_agent`, decoded shout text) is passed through `html_escape()`. No XSS.

**Info leakage via `/ip.php`, `/getmyip.php`:** These expose the client's IP address, which is expected behavior matching the original PHP site. The `/getmyip.php` endpoint also does a reverse DNS lookup, which is standard.

**Shout page (`/s.php`):** The `?e=` parameter is base64-encoded and redirected to `?s=`. The redirect URL is built from `urlencoding::encode()`, preventing header injection. The decoded text is `html_escape()`d before rendering.

### 9. Supporting Code

**IP Spoofing (`src/libs/util.rs`):** X-Forwarded-For and X-Real-IP are only trusted from `TRUSTED_PROXIES` (localhost only). Direct connections use the socket address. Correct.

**HTTPS Detection (`src/libs/util.rs`):** X-Forwarded-Proto is only trusted from `TRUSTED_PROXIES`. Correct.

**CSRF (`src/libs/csrf.rs`):** Origin/Referer validation with DNS rebinding protection (request host must be an accepted host). The `hosts_match` function also accepts `MASTER_HOST` as a valid origin even when the request host is a dev host, which is needed for testing. Well-designed.

**SQL Injection:** All database queries in `upvotes.rs` use sqlx parameterized queries (`.bind()`). No string concatenation in queries.

---

## Verdict

**No showstopper issues found.** The middleware and routing layers are well-engineered for production deployment. The most impactful defense-in-depth improvement would be adding a `Content-Security-Policy` header, but its absence is not a showstopper given the consistent output escaping throughout the codebase.

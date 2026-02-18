# Bug Review: Routing & Page Registry

Reviewer scope: `main.rs`, `registered_page_handler.rs`, `registry/mod.rs`,
`registry/pages.rs`, `storage_routes.rs`, `context.rs`, `app_state.rs`, plus
cross-reference against `URLParse.php`.

---

## BUG-04-01: Wrong `legacy_hit_count_id` for pastebin page (hit counter continuity break)

**Severity: Medium** -- Will split the hit counter into two independent counters.

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registry/pages.rs`, line 260

The PHP version uses `P_FILE => "services/pastebin.html"` for the pastebin page
(URLParse.php line 780), making the hit counter key `pages/services/pastebin.html`.
The Rust registry has:

```rust
legacy_hit_count_id: "pages/services/pastebin.php",
```

This should be:

```rust
legacy_hit_count_id: "pages/services/pastebin.html",
```

Without this fix, the Rust version will start a new hit counter for the pastebin
page instead of continuing the existing count.

---

## BUG-04-02: Duplicate `bh2016`/`BH2016` aliases silently collide in HashMap

**Severity: Low** -- Functionally harmless since both point to the same target,
but represents a latent bug in the registry construction.

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registry/pages.rs`, lines 722-723

```rust
alias!("bh2016" => "side-channel-attacks-on-everyday-applications"),
alias!("BH2016" => "side-channel-attacks-on-everyday-applications"),
```

Both slugs are lowercased to `"bh2016"` when inserted into the HashMap. The second
entry silently overwrites the first. Since both point to the same target this has
no visible effect, but it masks a design flaw. The TODO comment at line 1457
acknowledges this:

```rust
// TODO: make it a loud error if there are entries with colliding lowercased slugs
```

**Recommendation:** Remove the `BH2016` alias (it's redundant since lookup is
case-insensitive) and add the collision check.

---

## BUG-04-03: 404 page `url_prefix` is hardcoded to `https://defuse.ca`

**Severity: Low** -- Affects development and any future multi-domain setup.

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registered_page_handler.rs`, line 285

```rust
fn render_not_found(client_ip: String, dnt_enabled: bool) -> Response {
    let ctx = PageContext {
        // ...
        url_prefix: "https://defuse.ca".to_string(),
    };
```

The `render_not_found` function constructs a `PageContext` with a hardcoded
`url_prefix` instead of deriving it from the request's Host header like the normal
path does (lines 73-78). If the 404 template (or base template) uses `url_prefix`
to build absolute URLs, they will be wrong on localhost during development.

**Fix:** Pass the `url_prefix` (or `host` and `scheme`) into `render_not_found`.

---

## BUG-04-04: 404 pages do not record hit counts

**Severity: Low** -- Breaks parity with the PHP version's analytics.

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registered_page_handler.rs`, lines 276-289

In the PHP version, 404 pages are hit-counted using the key `"pages/404.php"`
(URLParse.php line 811, index.php line 359). The Rust version uses
`HitCounts::default()` for 404 pages and never calls `record_and_get_hits`.
Additionally, `NOT_FOUND_PAGE_INFO` has `legacy_hit_count_id: ""` (registry/mod.rs
line 249), so even if someone tried to count 404 hits, the key would be empty.

If 404 hit analytics are desired, the `render_not_found` function should be made
async, call `record_and_get_hits("pages/404.php", ...)`, and set
`NOT_FOUND_PAGE_INFO.legacy_hit_count_id` to `"pages/404.php"`.

---

## BUG-04-05: Three PHP pages missing from Rust registry

**Severity: Varies** -- The `ip` page is a real missing page. The other two may be
intentionally dropped.

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registry/pages.rs`

Comparing the PHP `$PAGE_INFO` array against the Rust `PAGE_REGISTRY`:

### 5a. `"ip"` page -- **Missing** (Severity: Medium)

PHP has a registered page at `"ip"` (URLParse.php line 347-352):
```php
"ip" => array(
    P_FILE => "services/ip.php",
    P_TITL => "Your IP Address",
    P_METD => "Your IP Address!",
    P_METK => "online IP address, what is my ip, ip address, ssl ip address",
),
```

This was a full page (with site chrome/template) accessible at `/ip.htm` that showed
both the HTTPS and HTTP IP addresses side by side, with an explanatory paragraph.
The Rust version only has `/ip.php` as a plain-text endpoint returning just the raw
IP (special_endpoints.rs), but the `/ip.htm` page with full template is missing.

Anyone who has bookmarked `https://defuse.ca/ip.htm` will get a 404.

### 5b. `"peerreview"` page -- **Missing** (Severity: Low)

PHP has `"peerreview"` (URLParse.php line 399-404). This is an old service page
with a contact form. If this was intentionally removed, consider adding an alias
redirecting to `software-security-auditing` to avoid 404s for bookmarked links.

### 5c. `"passwordblocks"` page -- **Missing** (Severity: Low)

PHP has `"passwordblocks"` (URLParse.php line 785-790). This was a client-side
password generator using JavaScript. If intentionally removed, consider adding an
alias to `passgen` to avoid 404s.

---

## BUG-04-06: POST body buffered (up to 100 MB) before checking method support

**Severity: Medium** -- Potential denial-of-service vector.

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registered_page_handler.rs`, lines 110-146

For any POST request to a registered page, the handler reads the entire request body
(up to 100 MB) into memory before dispatching to the page handler. If the handler
does not support POST (returns `None` from `post()`), a 405 is returned -- but only
after the entire body has been buffered. The code itself flags this concern:

```rust
// AUDIT: Is this limit only being applied after this has all been loaded into memory?
```

To clarify the audit comment: `axum::body::to_bytes` streams and rejects when the
limit is exceeded, so it does not read beyond 100 MB. However, an attacker can
still force 100 MB of memory allocation per connection for any page, including
pages that do not accept POST at all. Most pages only need a few KB for upvote
forms.

There is also no global body limit applied before this handler runs (the TODO at
main.rs lines 145-146 acknowledges this). The `/bin/add.php` route has its own
`DefaultBodyLimit::max(100 * 1024 * 1024)`, but no global limit exists for other
routes.

**Recommendation:** Either:
1. Check whether the page handler supports POST *before* reading the body (look up
   the handler, check if `handler.post()` returns None on a dummy call, then
   return 405 early), or
2. Apply a smaller global body limit (e.g. 2 MB) and only raise it for specific
   routes that need more (checksums, pastebin_add, etc.).

---

## BUG-04-07: Storage route `not_found_service` receives prefix-stripped path

**Severity: Low** -- Likely harmless in practice but architecturally fragile.

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/storage_routes.rs`, lines 33-48

The storage routes use `nest_service` which strips the URL prefix before passing to
`ServeDir`. When a file is not found, the `not_found_service`
(`registered_page_handler::handle`) receives the request with the *stripped* path.
For example, a request to `/files/nonexistent` arrives at the handler as
`/nonexistent`.

This means:
- `resolve_path("/nonexistent")` returns `NotFound`, which correctly renders the
  404 page in most cases.
- However, if a file path happened to collide with a registered page slug (e.g.,
  `/files/about` would arrive as `/about`), it would serve the page instead of
  returning 404. No current file paths collide, but this is fragile.

Additionally, the 404 page rendered in this context will have the wrong path in
logging and the hardcoded `url_prefix` (per BUG-04-03).

---

## BUG-04-08: `resolve_alias` can infinite-loop on circular alias chains

**Severity: Low** -- Would cause a stack overflow at startup validation or on
first request, so it's loud. But there's no protection against accidental
circular aliases added in the future.

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registry/mod.rs`, lines 355-362

```rust
fn resolve_alias(page: &'static PageInfo) -> &'static PageInfo {
    if let Some(target) = page.redirect {
        let target_page = lookup_page(target).expect("BUG: redirect target must exist");
        resolve_alias(target_page) // Handle chains
    } else {
        page
    }
}
```

If two aliases point to each other (e.g., `alias!("a" => "b")` and
`alias!("b" => "a")`), this function will recurse infinitely and crash with a
stack overflow. There is no cycle detection or depth limit.

**Recommendation:** Add a max-depth counter (e.g., 10) or detect cycles by
tracking visited slugs.

---

## BUG-04-09: `fallback_service` serves files from `static/` including `.html` files as pages

**Severity: Informational** -- Not exploitable but worth understanding.

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/main.rs`, lines 126-135

The `ServeDir::new("static")` fallback will serve any file in the `static/`
directory, including `longcat.html`, `googlec56659c80ebb2d30.html`, and blog posts.
This is intentional (per the comments), but there's a nuance: these files bypass
the page registry entirely, so they:
- Do not get hit-counted
- Do not get security headers tailored to registered pages (no-cache, etc.)
- Do get the generic security headers from `SecurityHeadersMiddleware`

Files like `static/source/*.php` and `static/source/*.c` are served with
`Content-Disposition: attachment` by the `SecurityHeadersMiddleware` (which checks
`path.starts_with("/source/")`), so those are handled correctly.

No sensitive files (`.env`, `.git`, etc.) were found in the `static/` directory.

---

## BUG-04-10: `url_prefix` scheme detection does not account for `X-Forwarded-Proto`

**Severity: Low** -- In production behind Caddy this works correctly due to HTTPS
enforcement, but the logic is inconsistent.

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registered_page_handler.rs`, lines 70-78

```rust
let host = request
    .headers()
    .get(header::HOST)
    .and_then(|v| v.to_str().ok())
    .unwrap_or("defuse.ca");

let scheme = if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
    "http"
} else {
    "https"
};
```

The scheme is determined by checking if the host looks like localhost. However, the
middleware already extracts `X-Forwarded-Proto` to determine `is_https`
(url_canonicalization.rs line 97-102). The registered page handler does not check
`X-Forwarded-Proto` and instead uses a heuristic based on the hostname.

In production behind Caddy with HTTPS, the URL prefix will correctly be
`https://defuse.ca`. But if someone accesses the site through a non-standard
reverse proxy setup where the Host header is not `localhost` but the connection is
HTTP, the `url_prefix` will incorrectly say `https://`. The opposite case is also
possible: connecting to localhost over a TLS-terminating local proxy would
incorrectly use `http://`.

**Recommendation:** Check `X-Forwarded-Proto` header for consistency with the
middleware.

---

## Summary

| ID | Severity | Description |
|----|----------|-------------|
| 04-01 | **Medium** | Wrong `legacy_hit_count_id` for pastebin (`.php` should be `.html`) |
| 04-02 | Low | Duplicate BH2016 aliases silently collide |
| 04-03 | Low | 404 page `url_prefix` hardcoded to `https://defuse.ca` |
| 04-04 | Low | 404 pages do not record hit counts (PHP did) |
| 04-05 | **Medium/Low** | Three PHP pages missing: `ip` (Medium), `peerreview` (Low), `passwordblocks` (Low) |
| 04-06 | **Medium** | POST body (up to 100 MB) buffered before checking method support |
| 04-07 | Low | Storage route not_found_service receives prefix-stripped path |
| 04-08 | Low | `resolve_alias` can infinite-loop on circular alias chains |
| 04-09 | Info | Static file fallback bypasses hit counting and registry metadata |
| 04-10 | Low | `url_prefix` scheme detection doesn't use `X-Forwarded-Proto` |

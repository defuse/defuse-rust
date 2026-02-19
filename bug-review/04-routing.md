# Bug Review: Routing & Page Registry

Reviewer scope: `main.rs`, `registered_page_handler.rs`, `registry/mod.rs`,
`registry/pages.rs`, `storage_routes.rs`, `context.rs`, `app_state.rs`, plus
cross-reference against `URLParse.php`.

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

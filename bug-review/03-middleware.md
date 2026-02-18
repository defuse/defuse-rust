# Middleware Stack Review

## Middleware Ordering Comment is Incorrect (Actual Order is Reversed)
**Severity**: Medium
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/main.rs:137-140
**Description**: The comment says "the outermost layers come first" but in Axum/Tower, `.layer()` wraps the existing service, so the **last** `.layer()` call is the outermost. The actual request processing order is:

```
blocking (outermost) -> UrlCanonicalization -> upvote_post -> SecurityHeaders -> CatchPanic (innermost) -> handler
```

The comment implies the opposite. This matters because the intended design (per the comment) doesn't match the actual behavior. The developer appears to have gotten the correct ordering by coincidence or by testing, but the comment will mislead future maintainers.

## Security Headers Missing on All Redirect Responses
**Severity**: High
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/main.rs:149-159
**Description**: Because `SecurityHeadersLayer` is an inner layer relative to `UrlCanonicalizationLayer`, redirect responses from URL canonicalization (host redirects, HTTPS enforcement, path normalization) never pass through SecurityHeaders. These 301 redirect responses are missing:

- `X-Frame-Options: SAMEORIGIN`
- `X-Content-Type-Options: nosniff`
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Strict-Transport-Security` (on HTTPS-to-HTTPS redirects like host canonicalization)

This affects every user whose first request triggers a redirect (non-canonical URLs, wrong host, HTTP-to-HTTPS). The HSTS header is particularly important on redirect responses because that is often the first response a browser receives.

Similarly, the 302 redirect response from `upvote_post_middleware` (line 122 of upvote_post.rs) also bypasses SecurityHeaders because upvote_post is also an outer layer relative to SecurityHeaders.

**Fix**: Move `SecurityHeadersLayer` to be the outermost layer (or at least outside `UrlCanonicalizationLayer`), so it wraps all responses including redirects. Alternatively, add security headers directly in the redirect-generating code.

## CatchPanicLayer is Innermost -- Cannot Catch Middleware Panics
**Severity**: High
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/main.rs:143
**Description**: `CatchPanicLayer` is the innermost middleware (added first), so it only catches panics from the handler itself. Panics in `SecurityHeadersMiddleware`, `upvote_post_middleware`, or `UrlCanonicalizationMiddleware` propagate uncaught to `blocking_middleware`, where `JoinHandle::await.expect()` re-panics on the async worker thread. This can crash active connections.

This is not theoretical: `upvote_post_middleware` explicitly panics at line 119 (`state.upvotes.process_vote(...).expect(...)`) when the upvote database operation fails. A database timeout during an upvote POST would crash the connection instead of returning a 500 error.

**Fix**: Move `CatchPanicLayer` to be the outermost layer (added last), or at minimum, outside the layers that can panic.

## Upvote Middleware Panics on Database Failure
**Severity**: High
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/middleware/upvote_post.rs:117-119
**Description**: The upvote middleware uses `.expect("Failed to process upvote")` which panics if the database operation fails. Combined with the CatchPanicLayer ordering issue above, this panic is uncaught and will crash the connection. Any transient database error (timeout, connection reset, deadlock) during an upvote POST would trigger this.

The comment says "panic on failure so user doesn't think vote succeeded" but a 500 error response would accomplish the same goal without crashing.

**Fix**: Replace `.expect()` with error handling that returns a 500 Internal Server Error response instead of panicking.

## X-Forwarded-Proto Trusted Without Proxy Verification
**Severity**: Medium
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/middleware/url_canonicalization.rs:97-102 and security_headers.rs:79-84
**Description**: Both `UrlCanonicalizationMiddleware` and `SecurityHeadersMiddleware` trust the `X-Forwarded-Proto` header from any client without verifying the connection comes from a trusted proxy. In contrast, the `client_ip()` function in `libs/util.rs` correctly checks `TRUSTED_PROXIES` before trusting `X-Forwarded-For`.

If the application is directly exposed to the internet (not behind Caddy), an attacker could send `X-Forwarded-Proto: https` to:
1. Bypass HTTPS enforcement (Step 2 of URL canonicalization)
2. Cause HSTS headers to be set on HTTP responses (not a security issue itself, but incorrect)

In the expected deployment behind Caddy, this is mitigated because Caddy sets the header. But defense-in-depth says the application should not trust headers it cannot verify.

**Fix**: Only trust `X-Forwarded-Proto` when the connection comes from a trusted proxy IP, similar to the `client_ip()` function.

## Blocking Middleware Holds Blocking Threads During I/O Waits
**Severity**: Medium
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/middleware/blocking.rs:19-23
**Description**: The blocking middleware runs `handle.block_on(next.run(request))` inside `spawn_blocking`. This means each request occupies a blocking thread for its entire duration, including time spent waiting on async I/O (database queries, network calls). The Tokio blocking thread pool defaults to 512 threads max.

The middleware was designed for CPU-bound preemption, but the implementation ties up blocking threads during I/O waits. Under sustained load (e.g., 500+ concurrent requests all waiting on database queries), the blocking thread pool could be exhausted, causing new requests to queue indefinitely until a thread becomes available.

This is the outermost middleware, so ALL requests (including static file serving, which should be fast) go through the blocking pool.

**Fix**: Consider applying the blocking middleware only to CPU-intensive handlers (checksums, pastebin crypto) rather than globally. Alternatively, increase `max_blocking_threads` or use `tokio::task::block_in_place` instead (which reuses the current thread).

## Blog Slug Redirect Uses Relative Filesystem Path
**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/middleware/url_canonicalization.rs:186-188
**Description**: `check_blog_slug_redirect` constructs a relative path `format!("static{}.html", path)` and checks if the file exists. This uses the current working directory, which could differ from the expected directory if the binary is started from a different location. If the CWD is wrong, all blog slug redirects would silently stop working (returning 404 instead of redirecting to the .html version).

The dot-check on line 176 (`after_blog.contains('.')`) prevents path traversal via `..` because the dots in `../` are caught. URL-encoded traversal (`%2e%2e`) is also safe because the filesystem would not find a literal `%2e%2e` directory. So there is no path traversal vulnerability, but the CWD dependency is fragile.

**Fix**: Use an absolute path based on a known root (e.g., the CARGO_MANIFEST_DIR or an environment variable) instead of a relative path.

## Tower poll_ready/clone Contract Violation
**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/middleware/url_canonicalization.rs:80-85 and security_headers.rs:63-68
**Description**: Both manual Tower `Service` implementations call `self.inner.poll_ready(cx)` in `poll_ready`, but then clone `self.inner` in `call` and use the clone instead of the polled-ready instance. This violates the Tower Service contract which states that `call` should be invoked on the same instance that was polled ready.

The correct pattern is:
```rust
fn call(&mut self, req: Request<Body>) -> Self::Future {
    let inner = std::mem::replace(&mut self.inner, self.inner.clone());
    // `inner` is the polled-ready instance; self.inner is the fresh clone
}
```

In practice, Axum's inner services (Router, Handler) are always-ready, so this does not cause a runtime bug today. But if a rate-limiting or buffering layer is ever added to the stack, this would cause subtle failures (requests dropped or deadlocked).

## Empty Host Header Causes Malformed HTTPS Redirect
**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/middleware/url_canonicalization.rs:114-124
**Description**: When the Host header is empty or missing, `host_without_port` is `""`. Step 1 is skipped due to the `!host_without_port.is_empty()` guard. Step 2 then redirects to `https:///path` (note the triple slash -- empty host), which is a malformed URL. While empty Host headers are rare (HTTP/1.0 clients only, and Axum's HTTP parser may reject them), the redirect would be broken if it occurs.

**Fix**: Also skip Step 2 when `host_without_port` is empty, or return a 400 Bad Request for missing Host headers.

## No Global Body Size Limit for Non-Registered Routes
**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/main.rs:145-146
**Description**: The TODO at lines 145-146 notes that a global body size limit middleware is missing. Currently, only `registered_page_handler` enforces a 100MB limit (line 123 of registered_page_handler.rs), and `/bin/add.php` has an explicit 100MB limit. But other routes (storage routes, special endpoints) lack body size limits, which could allow memory exhaustion via large POST bodies to those endpoints. The upvote middleware's 10MB limit partially mitigates this for form-urlencoded POSTs to registered pages, but GET/PUT/PATCH/DELETE to non-registered routes have no protection.

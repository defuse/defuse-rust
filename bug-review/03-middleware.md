# Middleware Stack Review

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
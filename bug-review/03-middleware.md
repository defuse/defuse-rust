# Middleware Stack Review

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
# Pre-Deployment Security Review: Errors, DoS, and Concurrency

Reviewed: 2026-02-19
Scope: Panic/crash risks, resource exhaustion, error info leakage, concurrency issues

---

## SHOWSTOPPER Issues

### 1. SHOWSTOPPER: Vim syntax highlighting has no timeout -- can hang a blocking thread forever

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/vim_highlight.rs`, line 315

**Issue:** `VimHighlight::run_vim()` uses `std::process::Command` (synchronous, blocking) with no timeout mechanism whatsoever. If vim hangs (e.g., on pathological input, a broken pipe, or a system resource issue), the blocking thread it runs on will be occupied forever. Unlike gcc/objdump/ruby, which all use `tokio::process::Command` with `timeout()` and `kill_on_drop(true)`, vim has neither.

Although vim is currently only called on trusted (hardcoded) input for syntax highlighting static strings and source files, the lack of timeout still means:
- A single vim process that hangs (e.g., due to a corrupted .vimrc, a system-level issue, or a stale NFS mount for the cache directory) blocks one of the 4096 blocking threads permanently.
- Multiple pages render syntax highlighting (source code display pages), so if vim consistently hangs, the blocking thread pool can be exhausted, taking down the entire server.

**Fix:** Use `tokio::process::Command` with a timeout (e.g., 60 seconds) and `kill_on_drop(true)`, matching the pattern used by the x86 assembler and big number calculator. Alternatively, if synchronous execution is required, spawn the process and call `wait_timeout` via `std::process::Child`.

---

### 2. SHOWSTOPPER: `assert!()` in big number calculator request path will panic on crafted Ruby output

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/big_number_calculator/mod.rs`, lines 112-116

```rust
let escaped_value = crate::libs::util::html_escape(&value);
assert!(
    escaped_value == value,
    "BUG: Ruby output contains HTML-special characters: {:?}",
    value
);
```

**Issue:** This `assert!()` runs in the hot request path of the big number calculator POST handler. If Ruby produces output containing `<`, `>`, `&`, `"`, or `'`, this will panic. While the input filter and parser are designed to prevent this, the explicit purpose of this assert is to catch regressions in those filters. If such a regression occurs (or if a Ruby version change alters output formatting), the panic will crash the request handler.

Since `CatchPanicLayer` is in place, the panic is caught and returns a 500, so it will not crash the server. However, an `assert!()` for defense-in-depth in production code should be a graceful error return, not a panic. This is borderline -- it degrades to a 500 error rather than a crash, but the intent is clearly wrong for production.

**Fix:** Replace the `assert!()` with a check that returns an error result:
```rust
if escaped_value != value {
    tracing::error!("BUG: Ruby output contains HTML-special characters: {:?}", value);
    return CalculatorResult {
        output: "Internal error processing result.".to_string(),
        is_error: true,
    };
}
```

---

### 3. SHOWSTOPPER: No timeout on HTTP request to Google reCAPTCHA API

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/recaptcha.rs`, lines 50-61

```rust
let client = reqwest::Client::new();
// ...
let resp = client
    .post("https://www.google.com/recaptcha/api/siteverify")
    .form(&params)
    .send()
    .await?;
```

**Issue:** The reqwest client is created with default settings, which means no connect timeout and no request timeout. If Google's reCAPTCHA API becomes slow or unreachable, the request will hang indefinitely. This blocks the blocking thread handling that request. Since reCAPTCHA verification is called on every time capsule submission and potentially other POST forms, a Google API outage could exhaust the blocking thread pool as pending requests accumulate.

Additionally, a new `reqwest::Client` is created on every reCAPTCHA call. Each `Client::new()` creates a new connection pool with its own TLS session cache. This is wasteful.

**Fix:** Create a shared `reqwest::Client` with explicit timeouts:
```rust
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build reqwest client")
});
```

---

### 4. SHOWSTOPPER: No timeouts on any database queries

**Files:** All database pools in `phpcount.rs`, `upvotes.rs`, `pastebin.rs`, `trent.rs`, `timecapsule.rs`

**Issue:** All `MySqlPool::connect()` calls use default sqlx connection settings. The sqlx MySQL driver defaults have no query timeout (statements can run indefinitely) and no explicit acquire timeout for the connection pool. If the MySQL server becomes slow or unresponsive:
- Every request handler that does database I/O (which is every registered page, due to phpcount) will hang waiting for a database response.
- The blocking thread pool (4096 threads) will be exhausted as requests pile up.
- The server becomes completely unresponsive.

This is arguably the single most impactful DoS vector because it affects ALL pages, not just specific endpoints.

**Fix:** Configure connection pool options with timeouts:
```rust
let pool = MySqlPoolOptions::new()
    .acquire_timeout(Duration::from_secs(5))
    .max_connections(20)
    .connect(database_url)
    .await?;
```

Also consider adding a statement-level timeout by setting `wait_timeout` in the MySQL connection string or via `after_connect`.

---

## CRITICAL Issues

### 5. CRITICAL: `panic!()` in vim cache directory check is reachable in production

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/vim_highlight.rs`, line 133

```rust
} else {
    panic!("Cache directory {:?} doesn't exist, skipping cache", cache_dir_path);
}
```

**Issue:** In `VimHighlight::process_text()`, if the cache directory (derived from `STORAGE_PATH/vimhl`) does not exist at request time, this panics. The directory is checked at runtime, not at startup. If the storage directory is deleted, unmounted, or if permissions change while the server is running, every request that triggers syntax highlighting will panic. The code itself has a TODO comment acknowledging this should be handled more gracefully.

Since `CatchPanicLayer` catches this and returns 500, it won't crash the server, but it will cause all pages with syntax highlighting to return 500 errors instead of degrading gracefully.

**Fix:** Return an error instead of panicking, or fall back to uncached execution.

### 6. CRITICAL: TRENT `select_random_lines` has documented O(N^2) CPU DoS

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs`, lines 533-554

**Issue:** The code has a TODO comment documenting this: when `allow_repeat = false`, selecting N lines from a file with N lines uses rejection sampling that degrades to O(N^2) expected iterations. With the 1000-line limit on `randlines` and 10MB file limit, an attacker could upload a file with exactly 1000 lines and request 1000 unique random lines. The last few lines would require many retries (approaching birthday-paradox behavior), but with 1000 max lines this is bounded at roughly 1000 * 1000/2 = 500K iterations, each involving a crypto RNG call. This is expensive but probably not catastrophic given the 1000 cap -- it would take seconds, not minutes.

**Severity note:** The 1000-line cap limits the practical impact. This is annoying but not a full server takedown. Rated CRITICAL rather than SHOWSTOPPER because the cap bounds the worst case.

### 7. CRITICAL: Error messages from gcc/objdump may leak internal paths

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/x86_assembler/executor.rs`, line 75

```rust
AssemblerError::InternalError(msg) => write!(f, "Internal error: {}", msg),
```

**Issue:** The `InternalError` variant includes system error messages like "Failed to create temp dir: No space left on device" or "Failed to run gcc: No such file or directory". These are passed through `format_error()` in the page handler (`online_x86_assembler.rs` line 97) which HTML-escapes them, then displayed to the user. While these don't contain file paths (the temp paths are cleaned separately for `AssemblyFailure`), they can leak information about the server's internal state (disk space, installed software, etc.).

**Fix:** Log the detailed error internally and return a generic message to the user:
```rust
AssemblerError::InternalError(msg) => write!(f, "An internal error occurred. Please try again later."),
```

---

## Areas that look clean

**Body size limits:** The pastebin has a 100MB Axum layer limit with a 50MB application-level check. The registered page handler has a 100MB limit. The checksums page has a 5MB file limit. Caddy is documented as enforcing a 100MB limit. These are reasonable.

**Subprocess timeouts (except vim):** gcc, objdump, and ruby all use `tokio::process::Command` with `timeout()` and `kill_on_drop(true)`. The ruby evaluator additionally uses `ulimit -t` and `ulimit -v` as kernel-enforced backstops. These are well-defended.

**Regex patterns:** All regex patterns used are simple and bounded. The `(?m)^ ` pattern in html_escape, the `\d+\.\d+` in x86 filter, and the hex bytes pattern in parser are all safe from ReDoS. The `regex` crate uses a finite automaton engine that is inherently ReDoS-resistant.

**Concurrency in vim cache:** The vim highlight cache uses file-based locking (`flock`) with a double-check pattern (check cache, acquire lock, re-check cache). This correctly handles concurrent requests for the same cache key.

**Concurrency in TRENT:** The `complete_drawing` function uses `AND complete = 0` in the UPDATE query as an atomic check-and-set, preventing TOCTOU races where two concurrent requests try to complete the same drawing.

**CatchPanicLayer:** The panic-catching layer is correctly positioned as the outermost middleware layer, so it catches panics from any middleware or handler. The `blocking_middleware`'s `.expect()` re-panics on handler panics, which is then caught by `CatchPanicLayer`. This is verified by the panic_test page.

**Shared mutable state:** The application uses `AppState` which contains `PhpCountService` and `UpvoteService`, both holding `MySqlPool` (which is internally an `Arc<Pool>`, safe for concurrent use). Global `OnceLock` pools for pastebin, trent, and timecapsule are also safe. There is no shared mutable state outside of the database pools and the file-lock-protected vim cache.

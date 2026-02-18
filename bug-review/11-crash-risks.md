# Error Handling & Crash Risks

Pre-deployment review of panic/crash risks in the defuse-rust codebase. The app
has `CatchPanicLayer` which converts panics to 500 errors rather than process
crashes, but panics still cause failed requests and should be avoided.

Findings are grouped by severity: **HIGH** = reachable from normal user
requests/bad input, **MEDIUM** = reachable under plausible operational
conditions, **LOW** = theoretically possible but extremely unlikely.

---

## HIGH: Panics reachable from user requests

### 1. `home.rs` and `all_pages.rs` -- `.expect()` on database queries

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/home.rs:20-25`
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/all_pages.rs:19-24`

```rust
let top_pages = upvotes
    .get_top_pages(Some(8), None)
    .await
    .expect("BUG: Failed to get top pages from database");

let user_actions = upvotes
    .get_user_actions_batch(&top_pages, &client_ip)
    .await
    .expect("BUG: Failed to get user actions");
```

**Risk:** Any transient database connectivity issue (connection pool exhaustion,
MySQL restart, network blip) causes a panic on every request to the homepage or
the all-pages listing. These are the two most-visited pages on the site.

**Fix:** Replace `.expect()` with `.unwrap_or_else()` and return an empty list
or render an error message, similar to how `registered_page_handler.rs` handles
hit count failures gracefully.

---

### 2. `upvote_post.rs:119` -- `.expect()` on vote processing

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/middleware/upvote_post.rs:119`

```rust
state.upvotes.process_vote(id, &client_ip, direction).await
    .expect("Failed to process upvote");
```

**Risk:** A database error during upvote processing (transient connectivity,
constraint violation from malformed `id` input, etc.) panics in middleware that
runs on every POST request to a registered page. The comment says "panic on
failure so user doesn't think vote succeeded" but a 500 error is not a good
user experience either.

**Fix:** Return a user-facing error response instead of panicking.

---

### 3. `trent.rs` (page handler) -- `assert!()` on user-controlled hash

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/trent.rs:528`

```rust
fn temp_path(drawing_num: i32, hash: &str) -> String {
    // Defense-in-depth against path traversal attacks.
    assert!(trent::is_sha256_hex(hash));
    format!("/tmp/trent-{}-{}", drawing_num, hash)
}
```

**Risk:** `temp_path()` is called from `delete_temp_files()` at line 544, which
iterates over `files` from the validated drawing params. At that point the hash
*should* be validated, but the assert creates a panic path. More importantly,
`temp_path()` is also called from `save_files_to_temp()` at line 467, which
iterates over `params.files` where `file.hash` was set by `trent::sha256_hex()`
(line 354), so it should always pass. However, there is also a path where
`file.hash` comes from the form's `file1hash` field (line 333) -- when
`confirmed == "true"`, the hash comes directly from the form POST data
(`form.file1hash`). The `load_temp_file()` function at line 534 checks
`is_sha256_hex` before calling `temp_path`, but `delete_temp_files()` at line
544 also checks before calling. So in practice the callers validate first.

**Severity note:** Callers all validate before calling, so this is actually
safe, but using `assert!()` for defense-in-depth against user input is
fragile -- a future caller might forget the check.

**Fix:** Return an error or skip the file instead of asserting. Or at minimum,
use `debug_assert!` since callers already validate.

---

### 4. `trent.rs` (page handler) -- `assert!()` on database result

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/trent.rs:226`

```rust
assert!(drawing_num == drawing.drawingnum);
```

**Risk:** If the database returns a drawing with a mismatched `drawingnum` (e.g.
due to a bug in a future SQL change), this panics during a normal GET request
to view a drawing. The assertion is checking a database invariant on every page
view.

**Fix:** Log an error and return a user-facing error message instead of panicking.

---

### 5. `trent.rs` (lib) -- `assert!()` and `.expect()` in `build_printout`

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs:360,397,401,453,456,464,470`

Multiple asserts and expects in the printout-building path:

```rust
// Line 360 (validate_files)
assert!(file.randlines >= 0); // checked in validate_create_request

// Line 397 (build_printout)
assert!(is_sha256_hex(&file.hash), "validated file {} has invalid hash", file_num);

// Line 401 (build_printout)
let content = file.content.as_ref().expect("validated file has no content");

// Line 453 (select_random_number)
assert!(high >= low, ...);

// Line 456 (select_random_number)
.expect("select_random_number: range overflows i64")

// Line 464 (reduce_mod)
assert!(divisor > 0, "reduce_mod: divisor must be > 0");

// Line 470 (reduce_mod)
.expect("reduce_mod: overflow in modular reduction");
```

**Risk:** These are all behind validation, but the validation-then-use pattern
across two separate modules (the page handler validates, then the lib acts)
makes it possible for a logic error to cause a panic triggered by user input.
For example, if the validation were ever relaxed to allow `lowval == highval`
with `numgen > 0`, `select_random_number` would still work (range of 1), but
the concern is the fragility of assert-based contracts.

The `.expect("reduce_mod: overflow in modular reduction")` at line 470 is
actually safe because `remainder < divisor <= u64::MAX`, so
`remainder * 256 + byte <= (u64::MAX - 1) * 256 + 255` which overflows.
Wait -- let me reconsider. `remainder` is at most `divisor - 1`. If `divisor`
is close to `u64::MAX`, then `remainder * 256` overflows. In practice, `divisor`
is `(high - low + 1)` where both are `i32` values cast to `i64` then to `u64`,
so `divisor` is at most ~2 billion, and `remainder * 256` is at most ~512 billion
which is fine. But mathematically, the function has a `checked_mul(256)` because
the overflow IS possible for large divisors. With the current i32 range
validation (MAX_RANGE_VAL = 1 billion), this is safe.

**Assessment:** All individually safe given current validation, but the defensive
asserts would turn into panics if validation constraints are changed in the future.

---

### 6. `registered_page_handler.rs:107` -- `.unwrap()` on handler option

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registered_page_handler.rs:107`

```rust
let handler = page_info.handler.unwrap();
```

**Risk:** If a page is registered with `handler: None` (redirect-only pages are
supposed to have `None` handler), but the middleware fails to redirect it, this
would panic. The `panic!` at line 99 for `PathLookupResult::Redirect` handles
one such case, but this `.unwrap()` is a second defense that also panics.

**Assessment:** This should be unreachable because `resolve_path` returns
`Redirect` for redirect-only pages and `Canonical` only for pages with handlers.
But if the registry ever has a page with `handler: None` and no redirect alias
pointing at it, this would panic. Currently safe.

---

### 7. `registered_page_handler.rs:178` -- `.unwrap()` on post_body

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registered_page_handler.rs:178`

```rust
Method::POST => match handler.post(ctx, &state, post_body.unwrap()) {
```

**Risk:** `post_body` is `None` when `method != Method::POST`, but this branch
is only entered when `method == Method::POST`. So `post_body` is always `Some`
here.

**Assessment:** Safe. The `unwrap()` is logically unreachable.

---

## MEDIUM: Panics under plausible operational conditions

### 8. `blocking_middleware.rs:23` -- `.expect()` on spawn_blocking result

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/middleware/blocking.rs:23`

```rust
tokio::task::spawn_blocking(move || handle.block_on(next.run(request)))
    .await
    .expect("blocking execution of a handler panicked")
```

**Risk:** `spawn_blocking` returns `Err` if the task panics. `CatchPanicLayer`
is positioned ABOVE this middleware in the stack (outermost = first), so panics
inside handlers are caught by `CatchPanicLayer` before they propagate to this
`.expect()`. However, the ordering is: CatchPanic -> SecurityHeaders ->
UpvotePost -> UrlCanonicalization -> Blocking -> handler. Since Blocking is
innermost and CatchPanic is outermost, a panic in the handler would propagate
through spawn_blocking, hit this `.expect()`, and... wait.

Actually, `CatchPanicLayer` catches panics at the tower-service level. The
handler runs inside `spawn_blocking`, so if it panics, `spawn_blocking` returns
`Err(JoinError)`. The `.expect()` then panics AGAIN, and *that* panic is caught
by `CatchPanicLayer`. So this is a double-panic scenario that still results in a
500 rather than a crash.

**Assessment:** Works in practice but the `.expect()` message is misleading --
it says the handler panicked, but the panic has already been converted to a
JoinError. The real risk is if `spawn_blocking` fails for other reasons (e.g.
runtime shutting down), but that's an edge case during graceful shutdown.

---

### 9. `checksums.rs:126,183` -- `spawn_blocking` returning default on panic

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/checksums.rs:126-128,183-185`

```rust
let results = tokio::task::spawn_blocking(move || compute_hashes(&data))
    .await
    .unwrap_or_default();
```

**Risk:** If `compute_hashes` panics, the user gets an empty results page with
no indication of what happened. This isn't a crash but it silently swallows
errors.

**Assessment:** LOW. The hash computations are purely deterministic on user
input and should not panic. But any future bug in a hash implementation would be
silently swallowed.

---

### 10. `vim_highlight.rs:33` -- `panic!()` on missing env var in lazy init

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/vim_highlight.rs:33`

```rust
Err(_) => panic!("STORAGE_PATH env var not set!"),
```

**Risk:** The `cache_dir()` function uses `OnceLock` and panics if
`STORAGE_PATH` is not set. Since `STORAGE_PATH` is checked at startup in
`main.rs:58`, this would only fire if the env var was somehow unset after
startup (not possible in normal operation). However, the function at line 527
has the same pattern:

```rust
.expect("STORAGE_PATH env var not set!")
```

**Assessment:** Safe in production since main.rs validates this at startup. Only
a concern in test environments.

---

### 11. `recaptcha.rs:41` -- `.expect()` on missing env var

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/recaptcha.rs:41`

```rust
let secret = std::env::var("RECAPTCHA_SECRET_KEY")
    .expect("RECAPTCHA_SECRET_KEY must be set for reCAPTCHA verification");
```

**Risk:** Unlike `STORAGE_PATH`, `RECAPTCHA_SECRET_KEY` is NOT validated at
startup. If this env var is missing, the first user who submits a form with
reCAPTCHA (time capsule page) will trigger a panic. This is read on EVERY
reCAPTCHA verification call, even ones that will be bypassed -- the env var
check is done before the response check intentionally (per the comment).

**Fix:** Either validate at startup (add to `main.rs` startup checks) or return
an error instead of panicking.

---

### 12. Database env vars in lazy-init pools -- `.expect()` on missing env vars

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/pastebin.rs:41`
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs:427`
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/timecapsule.rs:28`

```rust
let url = std::env::var("PASTEBIN_DATABASE_URL")
    .expect("PASTEBIN_DATABASE_URL must be set for pastebin");
```

**Risk:** These use `OnceLock` with lazy initialization. The `ensure_db_connection_works()`
calls in `main.rs` trigger the initialization at startup, so these `.expect()`
calls execute during startup where panicking is appropriate. If the startup
check succeeds, the `OnceLock` is populated and the `.expect()` is never reached
again.

**Assessment:** Safe -- startup validation ensures these run exactly once during
init.

---

### 13. `trent.rs:297` -- `timestamp_opt().unwrap()` in `format_date`

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs:297`

```rust
let dt = Utc.timestamp_opt(timestamp as i64, 0).unwrap();
```

**Risk:** `timestamp_opt` returns `LocalResult::Single`, `Ambiguous`, or `None`.
For UTC timestamps, it always returns `Single` for valid timestamps. With a `u32`
input cast to `i64`, the range is 0..4294967295 which is always valid. However,
the file has a TODO noting that `u32` timestamps overflow in 2106.

**Assessment:** Safe until 2106. The `unwrap()` is fine for all `u32` values.

---

## LOW: Extremely unlikely or currently unreachable

### 14. `registered_page_handler.rs:99` -- `panic!()` on redirect reaching dispatcher

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registered_page_handler.rs:99`

```rust
PathLookupResult::Redirect { canonical_path } => {
    panic!(
        "BUG: Redirect reached dispatcher - middleware failed to redirect {} -> {}",
        path, canonical_path
    );
}
```

**Assessment:** This is intentionally a bug-detection panic. It should be
unreachable because the URL canonicalization middleware handles all redirects.
If this ever fires, it indicates a genuine bug in the middleware that should
be fixed. The panic is appropriate here since it signals a programming error.

---

### 15. `passgen.rs:93` -- `panic!()` on RNG failure

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/passgen.rs:93`

```rust
if iterations >= iter_limit {
    panic!("There's something seriously wrong with the random number generator!");
}
```

**Assessment:** This requires the OS CSPRNG to produce astronomically biased
output (128 consecutive rejections per character). If this fires, the system
has much bigger problems than a panicked web request.

---

### 16. `passgen.rs:54` -- `assert!(charset_len <= 255)`

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/passgen.rs:54`

```rust
assert!(charset_len <= 255);
```

**Assessment:** Only called with hardcoded charsets (ASCII=94, ALPHANUM=62,
HEX=16). Unreachable unless someone adds a new charset with >255 characters.

---

### 17. Static string parsing `.unwrap()` calls

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/middleware/security_headers.rs:119,126,132,141,150,180`
- `/home/taylor/defuse-rewrite/defuse-rust/src/middleware/url_canonicalization.rs:219`
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/pastebin_view.rs:123,464`
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/pastebin_add.rs:91`

```rust
"text/html; charset=utf-8".parse().unwrap()
"SAMEORIGIN".parse().unwrap()
```

**Assessment:** These are all parsing static string literals into header values
or building `Response::builder()` responses with known-good parameters. They
cannot fail at runtime.

---

### 18. Regex `.unwrap()` in `LazyLock` initializers

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/x86_assembler/parser.rs:75`
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/x86_assembler/filter.rs:87`
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/x86_assembler/executor.rs:246`
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/html_escape.rs:21`

```rust
LazyLock::new(|| Regex::new(r"...").unwrap())
```

**Assessment:** These are compile-time-known regex patterns. If they were
invalid, they would panic on first use but they have been tested. Not a runtime
risk.

---

### 19. `SystemTime::now().duration_since(UNIX_EPOCH).unwrap()`

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs:289`
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/upvotes.rs:569`
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/timecapsule.rs:92`
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/phpcount.rs:164`
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/pastebin.rs:259`

**Assessment:** `duration_since(UNIX_EPOCH)` only fails if the system clock is
set before 1970. Not a realistic production concern.

---

### 20. `pastebin_crypto.rs:60,73` -- HMAC `.expect()` calls

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/pastebin_crypto.rs:60,73`

```rust
Hmac::<Sha256>::new_from_slice(url_key.as_bytes()).expect("HMAC can take key of any size")
```

**Assessment:** The comment is correct. HMAC-SHA256 accepts keys of any length
(it hashes keys longer than the block size). This cannot fail.

---

### 21. DES `.unwrap()` in LM hash

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/checksums.rs:391-392`

```rust
let cipher1 = des::Des::new_from_slice(&key1).unwrap();
let cipher2 = des::Des::new_from_slice(&key2).unwrap();
```

**Assessment:** `key1` and `key2` are always exactly 8 bytes (produced by
`seven_to_eight_bytes` which returns `[u8; 8]`). DES requires an 8-byte key.
Cannot fail.

---

### 22. `registry/pages.rs:1474` -- `.unwrap()` on `.next()` after empty check

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registry/pages.rs:1474`

```rust
let first = chars.next().unwrap();
```

**Assessment:** The function `is_valid_css_class` checks `s.is_empty()` at line
1470 and returns `false` before reaching this line. The unwrap is safe.

---

## Summary of recommended fixes

| Priority | File | Issue | Fix |
|----------|------|-------|-----|
| **HIGH** | `home.rs:20,25` | `.expect()` on DB queries for homepage | Use `.unwrap_or_else()` with fallback |
| **HIGH** | `all_pages.rs:19,24` | `.expect()` on DB queries for all-pages | Use `.unwrap_or_else()` with fallback |
| **HIGH** | `upvote_post.rs:119` | `.expect()` on vote processing | Return error response |
| **HIGH** | `recaptcha.rs:41` | `.expect()` on missing env var (not startup-checked) | Add startup check or return error |
| **MEDIUM** | `trent.rs` (page handler `:226`) | `assert!()` on DB result in GET handler | Log + error response |
| **MEDIUM** | `trent.rs` (page handler `:528`) | `assert!()` on user-derived hash | Return error or use `debug_assert!` |

The remaining findings (LOW priority) are either logically unreachable or
represent acceptable risk for static/compile-time values.

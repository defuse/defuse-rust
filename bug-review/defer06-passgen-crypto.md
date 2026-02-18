# Bug Review 06: Password Generator & Crypto/Auth Libraries

**Scope**: `libs/passgen.rs`, `libs/csrf.rs`, `libs/recaptcha.rs`, `pages/software/passgen.rs`, `upvote.rs`, `libs/upvotes.rs`

## Findings

### BUG: Pastebin POST endpoint has no CSRF protection

**File**: `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/pastebin_add.rs`
**Severity**: Low-Medium

The `POST /bin/add.php` endpoint has no CSRF check at all. An attacker could create a page that auto-submits a form to `/bin/add.php`, causing any visitor to unknowingly create a paste with attacker-controlled content.

The upvote endpoints (`/upvote.php` and the upvote POST middleware) both call `csrf::check_origin()`. The pastebin add handler does not.

Impact is limited because:
- The pastebin is anonymous (no user accounts), so there is no privilege escalation.
- The paste content comes from the attacker's form, not the victim's data.
- The worst outcome is junk pastes being created under the victim's IP, consuming storage.

However, in the original PHP, the pastebin also lacked CSRF protection, so this matches the original behavior. Still worth noting for a security review.

**Recommendation**: Consider adding `csrf::check_origin()` to the pastebin add handler. This would be a low-effort improvement. The client-side JavaScript would need to ensure the form submit includes the Origin header, which browsers do by default for same-origin POSTs.

---

### BUG: Upvote vote processing has a race condition (TOCTOU)

**File**: `/home/taylor/defuse-rewrite/defuse-rust/src/libs/upvotes.rs`, lines 146-201
**Severity**: Low

The `process_vote` method performs a read-then-write sequence without a transaction:

```rust
let existing = self.get_user_action(permanent_id, client_ip).await?;  // READ
match (direction, existing) {
    (VoteAction::Upvote, None) => {
        self.give_upvote(permanent_id, false).await?;  // WRITE 1
        self.set_user_action(permanent_id, client_ip, VoteAction::Upvote).await?;  // WRITE 2
    }
    // ...
}
```

If two concurrent requests arrive for the same user+page (e.g., user double-clicks rapidly), both could read `existing = None` and both would call `give_upvote`, incrementing the count twice while only one history record survives. This can permanently corrupt vote counts by +1 or -1.

The original PHP code has the same race condition, so this is a faithful port. But the Rust version with async could actually be more susceptible since requests are handled concurrently within the same process, whereas PHP typically uses separate processes per request.

**Recommendation**: Wrap the read-check-write sequence in a MySQL transaction, or use `INSERT ... ON DUPLICATE KEY UPDATE` with atomic operations. For the upvote system's low stakes, this is not critical.

---

### BUG: `ensure_page` race condition can cause duplicate rows

**File**: `/home/taylor/defuse-rewrite/defuse-rust/src/libs/upvotes.rs`, lines 316-330
**Severity**: Low

There is a TODO acknowledging this:
```rust
// TODO: A race condition could lead to duplicate rows being inserted for new pages
```

On first startup or when adding a new page to the registry, multiple concurrent requests could each see that the page doesn't exist and try to INSERT it, potentially causing duplicate rows if `permanent_id` is not a UNIQUE constraint.

**Recommendation**: Use `INSERT IGNORE` or `INSERT ... ON DUPLICATE KEY UPDATE` to make this idempotent. Alternatively, add a UNIQUE constraint on `permanent_id` in the `counts` table (which it probably should have anyway) and handle the duplicate key error gracefully.

---

### BUG: `set_user_action` has the same TOCTOU race as `ensure_page`

**File**: `/home/taylor/defuse-rewrite/defuse-rust/src/libs/upvotes.rs`, lines 644-678
**Severity**: Low

```rust
let exists: Option<(String,)> = sqlx::query_as("SELECT hash FROM history WHERE hash = ?")
    .bind(&hash).fetch_optional(&self.pool).await?;
if exists.is_some() {
    // UPDATE
} else {
    // INSERT
}
```

Two concurrent requests could both see `exists = None` and both try to INSERT, potentially causing a duplicate key error or duplicate rows.

**Recommendation**: Use `INSERT ... ON DUPLICATE KEY UPDATE` (MySQL's upsert).

---

### INFO: reCAPTCHA bypass hash comparison is not timing-safe (acceptable)

**File**: `/home/taylor/defuse-rewrite/defuse-rust/src/libs/recaptcha.rs`, lines 30-33
**Severity**: None (informational)

```rust
if hash == BYPASS_HASH {
    return Ok(true);
}
```

The comparison of the computed hash with the expected hash uses standard string equality, which may leak timing information. However, this is explicitly acceptable because:

1. The bypass key is described as "random 256 bits", so brute-forcing it is infeasible regardless of timing leaks.
2. The comparison is between two SHA-256 hex strings. The variable-time comparison reveals at most the length of the common prefix, which does not meaningfully help an attacker who must find a 256-bit preimage.
3. The header is only used in automated testing environments.

No action needed.

---

### INFO: Password generator is correct and well-implemented

**File**: `/home/taylor/defuse-rewrite/defuse-rust/src/libs/passgen.rs`
**Severity**: None (positive finding)

The password generator correctly implements:
- **Rejection sampling**: Uses `get_minimal_bit_mask` to create the tightest possible mask, then rejects values outside the charset range. This produces a perfectly uniform distribution.
- **Constant-time indexing**: Uses the `subtle` crate's `ConditionallySelectable` and `ConstantTimeEq`, which is a better implementation than the PHP version's hand-rolled bitmask approach. The `subtle` crate's types are designed to prevent the compiler from optimizing away the constant-time property.
- **Edge case handling**: `get_minimal_bit_mask(0)` returns 0, which correctly handles a 1-character charset (mask=0, so `byte & 0 = 0`, which is always `< 1`, selecting index 0 every time).
- **Iteration limit**: Prevents DoS from a broken RNG.

The charsets match the PHP originals exactly. The `assert!(charset_len <= 255)` is correct because `constant_time_index` casts to `u8`. (It could be `<= 256` since the maximum index is `charset_len - 1 = 255`, which fits in `u8`, but all charsets are well under this limit so it is not a practical issue.)

---

### INFO: `no_cache` is properly configured for the passgen page

**File**: `/home/taylor/defuse-rewrite/defuse-rust/src/registry/pages.rs`, line 1315
**Severity**: None (positive finding)

The passgen page entry includes `no_cache: true`, and the security headers middleware at `security_headers.rs` lines 156-172 correctly handles this by setting:
- `Expires: Mon, 01 Jan 1990 00:00:00 GMT`
- `Cache-Control: no-cache, no-store, must-revalidate`
- `Pragma: no-cache`

This prevents generated passwords from being cached by the browser or any intermediate proxy.

---

### INFO: CSRF protection covers all vote-related POST endpoints

**File**: `/home/taylor/defuse-rewrite/defuse-rust/src/upvote.rs`, `/home/taylor/defuse-rewrite/defuse-rust/src/middleware/upvote_post.rs`
**Severity**: None (positive finding)

Both vote submission paths are protected:
1. **AJAX path** (`POST /upvote.php` in `upvote.rs`): Calls `csrf::check_origin()` at line 32.
2. **Non-JS fallback** (upvote POST middleware in `upvote_post.rs`): Calls `csrf::check_origin()` at line 110.

The CSRF implementation properly:
- Checks Origin header first, falling back to Referer.
- Validates that the request Host is an accepted host (preventing DNS rebinding).
- Strips ports before comparison (handles `defuse.ca:443` vs `defuse.ca`).
- Allows the master host as Origin regardless of the request Host (for dev→prod cross-posting).

---

### INFO: Other POST endpoints without CSRF (by design)

Several page handlers accept POST but do not have CSRF protection:

- **checksums** (`pages/services/checksums.rs`): Accepts file/text for hashing. No state change, purely computational, no reason for CSRF.
- **big_number_calculator** (`pages/services/big_number_calculator.rs`): Accepts expressions for calculation. No state change.
- **html_sanitize** (`pages/services/html_sanitize.rs`): Accepts text for escaping. No state change.
- **online_x86_assembler** (`pages/services/online_x86_assembler.rs`): Accepts assembly code. No state change.
- **trent** (`pages/services/trent.rs`): Accepts drawing parameters. Creates database records but is protected by its own validation logic and the drawing is submitted by the user intentionally.
- **quantum_computer_time_capsule** (`pages/services/quantum_computer_time_capsule.rs`): Protected by reCAPTCHA verification.
- **passgen** (`pages/software/passgen.rs`): POST just regenerates passwords (same as GET). No state change.

These are all acceptable: CSRF protection is only needed for endpoints that cause state changes on behalf of the user (votes, message creation, etc.).

---

## Summary

| # | Finding | Severity | Status |
|---|---------|----------|--------|
| 1 | Pastebin POST has no CSRF check | Low-Medium | Matches PHP; consider adding |
| 2 | Upvote `process_vote` race condition (TOCTOU) | Low | Matches PHP; wrap in transaction |
| 3 | `ensure_page` race condition on INSERT | Low | Use `INSERT ... ON DUPLICATE KEY UPDATE` |
| 4 | `set_user_action` race condition on INSERT | Low | Use `INSERT ... ON DUPLICATE KEY UPDATE` |
| 5 | reCAPTCHA bypass uses non-constant-time comparison | Info | Acceptable (256-bit preimage) |
| 6 | Password generator correctness | Info | Correct and well-implemented |
| 7 | `no_cache` on passgen page | Info | Properly configured |
| 8 | CSRF coverage on vote endpoints | Info | All paths covered |

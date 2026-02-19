# Data Loss & Correctness Review

## 1. Encoding Issues

### 1.1 CRITICAL: Pastebin null-byte padding truncates pastes ending with null bytes

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/pastebin_crypto.rs`, lines 169-171

```rust
// Strip trailing null bytes (mcrypt zero-byte padding)
let end = decrypted.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
decrypted.truncate(end);
```

The decryption routine strips **all** trailing null bytes from the plaintext. If a paste legitimately contains trailing `\0` bytes, they will be silently removed on read. The codebase comments acknowledge this: "It uses null-byte padding (which is fine for this use case since we only officially support text inputs, not files.)"

**Impact:** Low in practice -- pastebin input arrives via a form-urlencoded `String`, which cannot contain null bytes (they'd be `%00`-encoded into the string `"%00"`, not an actual `0x00` byte). The line ending normalization also operates on Rust `String`s, not raw bytes. However, the jscrypt path posts client-encrypted ciphertext as the paste content -- since SJCL ciphertext is JSON/base64, it also won't contain null bytes. This is a design limitation rather than a live bug, but the code should document this assumption more explicitly.

**Severity:** Low (no realistic data loss path for current inputs)

### 1.2 MODERATE: `String::from_utf8_lossy` silently corrupts TRENT drawing data

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs`, lines 160-163

```rust
// TODO: Even given the latin1 check (which just checks <=255) this is NOT safe
// since you could encode invalid utf8 codepoints by controlling arbitrary bytes.
printout: String::from_utf8_lossy(&printout).into_owned(),
userprintout: String::from_utf8_lossy(&userprintout).into_owned(),
```

The `printout` and `userprintout` columns are fetched as `Vec<u8>` from MySQL. If the database contains bytes that are not valid UTF-8 (e.g., Latin-1 encoded data from the old PHP version, where bytes 0x80-0xFF are single-byte characters), `from_utf8_lossy` will replace them with the Unicode replacement character U+FFFD. This silently corrupts the stored data on read.

The code itself has a TODO comment acknowledging this problem. The `is_latin1_safe()` check (line 557-559) only validates that characters have code points <= 255, but some Latin-1 characters when encoded as UTF-8 are multi-byte sequences, whereas the MySQL column may store them as single bytes (Latin-1 encoding).

**Impact:** Existing TRENT drawings that were created with the PHP version and contain non-ASCII Latin-1 characters (e.g., accented names like "Jose") will display with replacement characters instead of the original characters. New drawings go through the Latin-1 check which is UTF-8 safe, so this only affects legacy data.

**Severity:** Moderate (corrupts display of historical data)

### 1.4 LOW: `data_as_string()` in TRENT form parsing uses `from_utf8_lossy`

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/trent.rs`, lines 515-518

```rust
impl FormFieldExt for FormField {
    fn data_as_string(&self) -> String {
        // TODO: could this be losing info?
        String::from_utf8_lossy(&self.data).into_owned()
    }
}
```

Multipart form fields for TRENT (name, description, passcode, etc.) are converted from raw bytes to String using `from_utf8_lossy`. If a browser sends non-UTF-8 data in a multipart form field, the data would be silently corrupted. In practice, modern browsers always send UTF-8 for text fields, and the TRENT form subsequently validates Latin-1 safety, so corrupted data would be rejected. But the passcode field could theoretically be corrupted before comparison.

**Severity:** Low (unlikely in practice, browsers send UTF-8)

## 2. Overflow/Truncation

### 2.1 CRITICAL: TRENT `u32` timestamps overflow on 2106-02-07

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs`, lines 8-9 and 286-291

```rust
// TODO: This uses 32-bit integers for timestamps, needs to be updated before
// 32-bit timestamps overflow!

pub fn now() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32
}
```

The `as u32` cast will truncate after 2106-02-07 03:28:15 UTC. Before that, in release mode (wrapping arithmetic), `draw_date()` at line 39 can overflow silently:

```rust
pub fn draw_date(&self) -> u32 {
    self.starttime + self.reviewtime
}
```

If `starttime + reviewtime` overflows `u32`, the draw date wraps around to a past date, allowing the drawing to be completed immediately and bypassing the review period. The TODO on line 254 of `trent.rs` (the page handler) confirms this: "a large value can overflow draw_date() (starttime + reviewtime wraps in release mode)."

Additionally, `reserve_drawing` accepts any `u32` for `review_time` without validation against the allowed dropdown values. A user could POST `prereview=4294967295` which, when added to the current timestamp, would overflow.

**Impact:** The review period bypass could be exploited today by setting an extremely large review time. The general `u32` overflow is decades away but the code is marked as needing a fix.

**Severity:** High (exploitable review period bypass via overflow)

### 2.2 LOW: `last_insert_id()` truncation

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs`, line 187

```rust
drawing_num: result.last_insert_id() as i32,
```

`last_insert_id()` returns `u64`, cast to `i32`. If the auto-increment counter exceeds 2^31 - 1 (2,147,483,647), the drawing number would wrap to negative, causing confusing behavior. Unlikely in practice.

**Severity:** Low (would require billions of drawings)

## 3. Race Conditions

### 3.1 MODERATE: Pastebin key generation race (check-then-insert without transaction)

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/pastebin.rs`, lines 150-188

```rust
// Check if token already exists
let exists: Option<(String,)> =
    sqlx::query_as("SELECT token FROM pastes WHERE token = ?")
        .bind(&token)
        .fetch_optional(self.pool)
        .await?;

if exists.is_none() {
    url_key = candidate;
    break;
}
// ...later...
sqlx::query("INSERT INTO pastes (token, data, time, jscrypt) VALUES (?, ?, ?, ?)")
```

The uniqueness check (SELECT) and the INSERT are not in a transaction. Two concurrent requests could both SELECT the same token as available, then both try to INSERT, causing a duplicate key error. Given that keys are 22 random alphanumeric characters (62^22 possible values), collision probability is astronomically low. But with short URLs (8 chars, 62^8 = ~218 trillion), the probability is higher though still negligible.

The proper fix would be to attempt the INSERT directly and catch the duplicate key error, or use a transaction with a unique constraint.

**Severity:** Low (astronomically unlikely with random keys, but architecturally incorrect)

### 3.2 MODERATE: Upvote process_vote is not atomic (count drift)

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/upvotes.rs`, lines 146-201

The `process_vote` method reads the user's existing vote, then performs a count update and a history update as separate operations with no transaction. If two concurrent requests arrive:

1. Request A: reads "no vote exists"
2. Request B: reads "no vote exists"
3. Request A: increments upvotes, inserts history
4. Request B: increments upvotes, tries to insert history (duplicate?)

The count update (`UPDATE counts SET upvotes = upvotes + 1`) and the history tracking (`INSERT/UPDATE history`) are separate queries. If request B also sees "no vote" and both increment, the upvote count increases by 2 for a single user's click. The `history` table update may also fail or create duplicate entries.

Over time, this can cause vote count drift (counts don't match actual voting history). This matches the PHP behavior ("Same bug exists in the PHP version" -- phpcount.rs line 80).

**Severity:** Low (minor count inaccuracies, matches original behavior)

### 3.3 LOW: Hit counter duplicate row creation

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/phpcount.rs`, lines 224-255

```rust
// TODO: A race condition could result in duplicate rows.
sqlx::query("INSERT INTO hits (pageid, isunique, hitcount) VALUES (?, 0, 0)")
```

The `create_counts_if_not_present` method checks for existence then inserts, without a transaction or unique constraint. Two concurrent first-visits to a new page could create duplicate rows. The code is aware of this (line 234) and the `get_hit_counts` method uses `LIMIT 1` to tolerate duplicates. However, site-wide totals use `SUM` which would double-count if duplicates exist (noted on line 124).

**Severity:** Low (only affects first visit to a brand new page, site totals slightly inflated)

### 3.4 INFO: TRENT drawing completion race is properly handled

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs`, lines 200-203

```rust
// SECURITY: "AND complete = 0" prevents a TOCTOU race where two concurrent
// requests both pass validation and try to complete the same drawing.
let result = sqlx::query(
    "UPDATE drawings SET complete = 1, printout = ?, userprintout = ? WHERE drawingnum = ? AND complete = 0"
```

This is correctly handled. The `AND complete = 0` clause acts as an atomic compare-and-swap, ensuring only one request can complete a drawing. The second concurrent request sees `rows_affected() == 0` and returns an error.

## 4. Data Corruption

## 5. Silent Failures

### 5.1 MODERATE: Hit counting silently fails and returns default counts

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registered_page_handler.rs`, lines 197-209

```rust
async fn record_and_get_hits(...) -> HitCounts {
    // Record the hit (errors logged but don't block page render)
    if let Err(e) = state.phpcount.add_hit(page_id, client_ip, user_agent).await {
        error!("Failed to record hit for {}: {}", page_id, e);
    }

    // Get hit counts
    state.phpcount.get_hit_counts(page_id).await
        .unwrap_or_else(|e| {
            error!("Failed to get hit counts for {}: {}", page_id, e);
            HitCounts::default()
        })
}
```

If the PHPCount database becomes unavailable:
- Hits are silently lost (not retried or queued)
- The page renders with zero hit counts (`HitCounts::default()`)
- The only evidence is a log line

This is a reasonable design choice (don't block page renders on hit counting), but during a database outage, all hit count data for that period is permanently lost. There is no recovery mechanism.

**Severity:** Low (acceptable tradeoff for availability, but data is permanently lost)

### 5.2 LOW: Vote state silently defaults on database error

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registered_page_handler.rs`, lines 263-273

```rust
state.upvotes
    .get_vote_state(upvote_config.id, client_ip)
    .await
    .unwrap_or_else(|e| {
        error!("Failed to get vote counts for {}: {}", upvote_config.id, e);
        VoteState::default()
    })
```

Similar to hit counting -- vote state defaults to zeros on database error. This means users could see incorrect vote counts during database issues. If they vote during this time, the vote processing (in `upvote.rs`) would also fail and return an error to the client, so no phantom votes would be created.

**Severity:** Low (temporary display issue during outages)

### 5.3 LOW: `ensure_page` silently fails for upvote registration

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registered_page_handler.rs`, lines 245-260

```rust
if let Err(e) = state
    .upvotes
    .ensure_page(...)
    .await
{
    error!("Failed to ensure page {} in upvotes database: {}", upvote_config.id, e);
}
```

If `ensure_page` fails (e.g., database error), the page still renders, but the upvote data for that page might not exist in the database. A subsequent vote attempt for that page could fail if the page was never created. This is only an issue for brand-new pages that haven't been visited yet, and only during database issues.

**Severity:** Low

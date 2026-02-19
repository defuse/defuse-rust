# TRENT System Review

## 1. CPU DoS via Random Line Selection Without Replacement
**Severity**: High
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs:537-551
**Description**: When `allow_repeat = false`, `select_random_lines` uses a rejection-sampling loop: it picks a random line, checks if it was already drawn, and retries if it was. If a user uploads a file with N lines and requests N random lines without replacement, the expected number of random number generation calls is O(N * H_N) where H_N is the Nth harmonic number, but more importantly each iteration does a linear scan of `excluded` (which grows to size N), making the total work O(N^2 * H_N). With `num_lines = 1000` (the max) and a 1001-line file, this could produce significant CPU load. Since each call to `select_random_number` reads 32 bytes from `/dev/urandom` and does modular arithmetic, an attacker can trigger thousands of OS entropy reads and CPU cycles per request. The code already has a TODO acknowledging this. Mitigation: replace with Fisher-Yates shuffle or sampling from a shrinking pool.

## 2. Silent Temp File Loss Between Confirmation and Completion
**Severity**: High
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs:394-409
**Description**: In `build_printout`, the file SHA256 hash is only included in the printout if `file.content.is_some()` (line 396). During the confirmation-to-completion flow, files are loaded from `/tmp` by `load_temp_file`. If the temp files were deleted between confirmation and completion (e.g., system reboot, `/tmp` cleanup by `systemd-tmpfiles`, or manual deletion), `load_temp_file` returns `None`, and `validate_files` silently skips files with `None` content and `randlines == 0`. For files with `randlines > 0`, a `MissingFile` error is returned, which is good. But for files uploaded only for hash recording (randlines == 0), the file hash silently disappears from the completed drawing's printout with no error to the user. The user expects the file's SHA256 to be recorded, but it is not. The completed drawing is then missing evidence that those files existed. This undermines the integrity guarantee that TRENT provides for file checksums.

## 3. Printout Missing "ALLOW REPEAT LINES" Compared to Confirmation Display
**Severity**: Medium
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs:386-388
**Description**: The Rust printout includes `ALLOW REPEAT LINES: Yes/No` (line 387-388) which is a new field not present in the original PHP. This is an improvement. However, if `chosentwice` is false (the default), the printout says `ALLOW REPEAT LINES: No` but there is no corresponding validation that later prevents someone from claiming the drawing allowed repeats. This is informational only, but the wording is confusing: "ALLOW REPEAT LINES: No" appears in the printout even when no files are uploaded and no lines are being selected, making it misleading to verifiers.

## 4. u32 Timestamp Overflow in `draw_date()` Allows Review Period Bypass
**Severity**: Medium
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs:38-40
**Description**: `draw_date()` computes `self.starttime + self.reviewtime` using u32 arithmetic. In release mode (no overflow checks), a sufficiently large `reviewtime` causes the sum to wrap around to a value less than the current time. For example, with `starttime ~= 1740000000` and `reviewtime = u32::MAX (4294967295)`, the result wraps to `starttime - 1`, which is in the past. This causes the review period check at line 244 (`now() < drawing.draw_date()`) to pass immediately. The TODO at line 253 notes that `review_time` is not validated against the allowed dropdown values. While instant review (0) is already allowed so this is not a security escalation, the user sees a nonsensical drawing date (a date in the 1970s) and the behavior is silently wrong. The `reserve_drawing` function at line 189 has the same overflow: `format_date(starttime + review_time)`.

## 5. `last_insert_id()` Truncation to i32
**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs:187
**Description**: `result.last_insert_id()` returns `u64` but is cast to `i32` via `as i32`. If the MySQL auto-increment ID exceeds `i32::MAX` (2,147,483,647), the cast silently wraps, returning a wrong (possibly negative) drawing number to the user. The user would receive a drawing number that doesn't correspond to their actual database record, making it impossible to complete the drawing. The `drawing_num` type is `i32` throughout the codebase, so this would need a broader type change to fix properly. Unlikely to be hit in practice but is a correctness bug.

## 9. No Temp File Cleanup for Abandoned Drawings
**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/pages/services/trent.rs:464-472
**Description**: Temp files are written during the confirmation step and deleted only after successful completion. If a user confirms a drawing but never completes it (closes the browser, network error, etc.), the temp files remain in `/tmp` indefinitely until the OS cleans them. Over time, with many abandoned drawings, up to 30 MB per drawing (3 files x 10 MB) could accumulate. There is no background cleanup mechanism. The code comment at line 389-392 acknowledges that temp files are intentionally left around for retry, but there should be a periodic cleanup of temp files older than some threshold.

## 11. `from_utf8_lossy` Can Corrupt Legacy Database Records
**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs:161-162
**Description**: When reading `printout` and `userprintout` from the database (stored as `Vec<u8>`), `String::from_utf8_lossy` replaces any non-UTF-8 bytes with the Unicode replacement character (U+FFFD). The original PHP stored Latin-1 encoded data directly. Bytes in the range 0x80-0xFF that form valid Latin-1 characters but invalid UTF-8 sequences (e.g., accented characters like e-acute = 0xE9) would be corrupted when displayed by the Rust version. This only affects legacy drawings created by the PHP version that contain non-ASCII Latin-1 characters. New drawings pass through `is_latin1_safe` which requires valid UTF-8, so they are not affected. The code has a TODO at line 159-160 acknowledging this.

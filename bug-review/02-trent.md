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

## 6. Printout Format Differs from PHP: "RANDOM NUMBER, NUMBER" vs "RANDOM NUMBER NUMBER"
**Severity**: Medium
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs:414
**Description**: The Rust printout uses `"RANDOM NUMBER, NUMBER {}: {}\n"` (with a comma) while the PHP uses `"RANDOM NUMBER NUMBER $i: $randnum\n"` (no comma). Anyone parsing TRENT printouts programmatically (or comparing output format across old and new drawings) would see a difference. This is a behavioral regression from the PHP. It should either match the PHP format exactly or be documented as an intentional change.

## 7. Unrecognized Form Field Error Leaks Field Name
**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/pages/services/trent.rs:453
**Description**: When a multipart form submission includes an unrecognized field name, the error message `"Unrecognized form field: {}"` reflects the attacker-controlled field name back to the user. While Askama's auto-escaping prevents XSS, this strict rejection is a departure from the PHP (which silently ignores extra fields) and could cause issues with browsers or extensions that add hidden fields (e.g., some password managers). A more defensive approach would be to silently ignore unknown fields like the PHP does.

## 8. Temp Files World-Readable in /tmp
**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/pages/services/trent.rs:468
**Description**: `tokio::fs::write` creates files with default permissions (typically 0644), making uploaded file contents readable by any user on the system. While the file paths are unpredictable (they contain a SHA-256 hash), a local attacker who can list `/tmp` (possible since the filenames follow a predictable pattern `trent-{num}-{hash}` where `num` is a sequential integer) could enumerate temp files and read their contents. This is only exploitable by users with local shell access to the server. Mitigation: create temp files with mode 0600 using `std::fs::OpenOptions` with explicit permissions, or use a private temp directory.

## 9. No Temp File Cleanup for Abandoned Drawings
**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/pages/services/trent.rs:464-472
**Description**: Temp files are written during the confirmation step and deleted only after successful completion. If a user confirms a drawing but never completes it (closes the browser, network error, etc.), the temp files remain in `/tmp` indefinitely until the OS cleans them. Over time, with many abandoned drawings, up to 30 MB per drawing (3 files x 10 MB) could accumulate. There is no background cleanup mechanism. The code comment at line 389-392 acknowledges that temp files are intentionally left around for retry, but there should be a periodic cleanup of temp files older than some threshold.

## 10. `now()` Panics After Year 2106
**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs:286-291
**Description**: The `now()` function casts `SystemTime::now().duration_since(UNIX_EPOCH).as_secs()` to `u32`. After February 7, 2106 (when Unix timestamps exceed u32::MAX = 4,294,967,295), this cast will silently wrap in release mode, producing incorrect timestamps, or panic in debug mode. The file header (line 8-9) has a TODO noting this. The `starttime` and `reviewtime` database columns also use u32. While 2106 is far away, a migration path should be planned. This affects all timestamp comparisons, date formatting, and database storage.

## 11. `from_utf8_lossy` Can Corrupt Legacy Database Records
**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs:161-162
**Description**: When reading `printout` and `userprintout` from the database (stored as `Vec<u8>`), `String::from_utf8_lossy` replaces any non-UTF-8 bytes with the Unicode replacement character (U+FFFD). The original PHP stored Latin-1 encoded data directly. Bytes in the range 0x80-0xFF that form valid Latin-1 characters but invalid UTF-8 sequences (e.g., accented characters like e-acute = 0xE9) would be corrupted when displayed by the Rust version. This only affects legacy drawings created by the PHP version that contain non-ASCII Latin-1 characters. New drawings pass through `is_latin1_safe` which requires valid UTF-8, so they are not affected. The code has a TODO at line 159-160 acknowledging this.

## 12. File Hash Check Bypassed for Files With randlines == 0 on Confirmation
**Severity**: Medium
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/libs/trent.rs:348-371 and /home/taylor/defuse-rewrite/defuse-rust/src/pages/services/trent.rs:344-349
**Description**: In `validate_files` (line 348-371), the hash verification `sha256_hex(content) != file.hash` only runs if `file.content` is `Some`. During the confirmation flow, `load_temp_file` loads files by their claimed hash from the form's hidden fields. The content is loaded from `/tmp/trent-{drawing_num}-{hash}` where `hash` comes from user input. If the temp file exists and contains the expected content, this is fine. But consider: during the initial submission, the hash is computed server-side from the upload (line 354). During confirmation, the hash comes from a hidden form field that the user can tamper with. If the user changes the hash in the hidden field to point to a temp file from a different upload to the same drawing number, `load_temp_file` loads that file's content, and `validate_files` checks `sha256_hex(loaded_content) != tampered_hash`. Since the file was stored under its own real hash, the loaded content's actual hash would match the filename (the real hash) but not the tampered hash in the form field. So the hash check WOULD catch this. This is actually safe -- noting for completeness that the defense works correctly.

Upon closer inspection, the real concern is: since temp files are named `trent-{drawing_num}-{hash}`, and the user supplies both `drawing_num` and `hash` in the confirmation POST, could they reference temp files from a different drawing? They would need to know another drawing's number and file hash. Even then, `validate_create_request` checks the password against the drawing record, so they'd need the password too. This is safe.

**Revising severity to N/A** -- removed from final count. This analysis confirms the defense-in-depth works.

## Summary

| # | Issue | Severity |
|---|-------|----------|
| 1 | CPU DoS via rejection sampling | High |
| 2 | Silent temp file loss drops file hashes from printout | High |
| 3 | Misleading "ALLOW REPEAT LINES" when no files used | Medium |
| 4 | u32 overflow in draw_date allows review period bypass | Medium |
| 5 | last_insert_id truncation to i32 | Low |
| 6 | Printout format differs from PHP (comma in label) | Medium |
| 7 | Unrecognized form field error leaks field name | Low |
| 8 | Temp files world-readable in /tmp | Low |
| 9 | No cleanup for abandoned drawing temp files | Low |
| 10 | now() wraps/panics after 2106 | Low |
| 11 | from_utf8_lossy corrupts legacy Latin-1 records | Low |

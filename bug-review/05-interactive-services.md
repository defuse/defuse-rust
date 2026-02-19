# Bug Review: Interactive Services

Scope: checksums, html_sanitize, online_x86_assembler, quantum_computer_time_capsule, big_number_calculator, web_server_scan, and all supporting libraries (big_number_calculator/*, x86_assembler/*, html_escape, timecapsule, breach).

## BUG-05-04: Time capsule archive format has a subtle difference from PHP [Medium Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/quantum_computer_time_capsule.rs` (lines 249-295)
- `/home/taylor/defuse-rewrite/defuse.ca/src/timecapsule/quantum-computer-time-capsule-download.php`

**Description:**
The code has a comment: "DO NOT CHANGE THIS. We must produce output that is byte-for-byte identical to past versions, because various hashes of the first N lines of the file are written to blockchains."

However, the Rust version reads `archive-header.txt` with `tokio::fs::read_to_string`, which will include the file contents as-is. The PHP version uses `file_get_contents("archive-header.txt")` from the `timecapsule/` directory (relative to the PHP script's location).

The Rust version reads from `static/timecapsule/archive-header.txt` (relative to the Rust binary's CWD). If the file content is byte-for-byte identical, the output will match. But there is a structural concern: the messages from the database are read with `String::from_utf8_lossy`, which replaces invalid UTF-8 bytes with the Unicode replacement character (U+FFFD). The PHP version echoes the raw bytes from the database. If any message contains non-UTF-8 bytes, the Rust archive output will differ from the PHP version, breaking the blockchain hash verification.

**Recommendation:**
Use raw bytes (`Vec<u8>`) for the archive output instead of `String` to guarantee byte-for-byte compatibility with the PHP version. The messages should be written as raw bytes, not converted through `String::from_utf8_lossy`.

---

## BUG-05-06: Time capsule form fields not server-side validated [Low Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/quantum_computer_time_capsule.rs` (lines 118-125)

**Description:**
The `algorithm`, `present_public_key`, `future_public_key`, and `ciphertext` form fields are taken directly from the POST body and concatenated into the encrypted message line. The PHP version does the same. While the fields are checked for newlines and a 200KB size limit, there is no validation that these fields contain expected values (e.g., `algorithm` should be a known algorithm name, public keys should be valid base64).

This matches the PHP behavior, so it is not a regression. The CAPTCHA and size limit provide some abuse protection. However, an attacker could submit arbitrary data as the "encrypted message," which will be stored in the database and included in the archive.

**Recommendation:**
This is a pre-existing design choice inherited from PHP. Consider adding basic validation for the algorithm field (e.g., must be one of a known set) and checking that public keys and ciphertext look like valid base64. Low priority since the PHP version has the same behavior.

---

## BUG-05-07: x86 assembler -- `check_code_safety` case-sensitive directive matching [Low Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/x86_assembler/filter.rs` (lines 78-81)

**Description:**
The safe directive removal uses exact string replacement: `filtered = filtered.replace(directive, "");`. GAS directives are case-insensitive, so `.BYTE` or `.Ascii` would not be matched by the whitelist and would be rejected (because the `.` remains after filtering). This is actually the SAFE direction -- it means `.INCLUDE` and `.Fill` are also rejected. The PHP version behaves identically (`str_replace` is case-sensitive). This is correct behavior: being overly strict is safe.

No action needed. Documented for completeness.

---

## BUG-05-08: x86 assembler allows GCC preprocessor directives via `#` comments [Medium Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/x86_assembler/filter.rs` (lines 91-93)
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/x86_assembler/executor.rs` (lines 106, 108)

**Description:**
The filter only checks for `#APP` and `#NO_APP`. The source file uses a lowercase `.s` extension (which tells GCC to skip the C preprocessor), so `#include`, `#define`, etc. would not be processed. This is the correct mitigation.

However, note that the source is written as `.intel_syntax noprefix\n_main:\n{user_code}\n`. If GCC's assembler (`as`) interprets `#` as a comment character (which it does for x86), then `#include` in user code is just a comment, not a preprocessor directive. The `.s` extension is the key defense here.

No action needed. The `.s` extension correctly prevents CPP processing.

---

## BUG-05-09: x86 assembler -- no input size limit on disassembly hex input string [Low Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/x86_assembler/mod.rs` (lines 85-101, 114-146)

**Description:**
The `disassemble` function has a `MAX_BINARY_INPUT_SIZE` check (1MB) on the _parsed binary data_, but the input hex _string_ itself has no size limit. A hex string representing 1MB of binary is 2MB of hex characters (plus separators). The `parse_hex_input` function processes the entire string in memory. An attacker could submit a very large hex string (e.g., 100MB) that would be fully processed before the binary size check occurs.

The PHP version also lacks this check, but the PHP version has PHP's `post_max_size` as a global limit. The Rust version should have a body size limit configured at the framework level, but the hex parsing itself is memory-intensive (collecting all chars, filtering, chunking).

**Recommendation:**
Add an early check on `hex_input.len()` before parsing. For example, reject inputs longer than `MAX_BINARY_INPUT_SIZE * 3` (accounting for spaces between hex bytes).

---

## BUG-05-11: HTML sanitize page -- error handling erases user input [Low Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/html_sanitize.rs` (lines 50-61)

**Description:**
When the tab width is invalid (< 1), the page returns with `data: "ERROR: Invalid tab width."` which replaces whatever the user typed. The code even has a TODO comment acknowledging this: "this method of error handling is not ideal, since it erases the contents the user entered."

The PHP version has the same behavior (it outputs "Invalid tab width." into the textarea, erasing user input).

**Recommendation:**
Store the original user input separately from the error message so both can be displayed. Low priority since it matches PHP behavior.

---

## BUG-05-13: Big number calculator -- `group_digits` produces trailing space for small numbers [Low Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/big_number_calculator/formatter.rs` (lines 33-39, 73-76)

**Description:**
When the number of digits is less than the grouping interval (e.g., the number "5" with interval 3), `group_digits` produces `"&nbsp;&nbsp;5 "` -- with a trailing space. The test explicitly documents this: "PHP adds trailing space after first partial group when no more digits follow."

This matches PHP behavior, so it is correct. However, the trailing space causes a small visual artifact when "add spaces" is enabled for small results. Since this is a PHP-compatible behavior, it should be kept.

No action needed.

---

## BUG-05-14: Big number calculator -- negative hex numbers may be formatted incorrectly [Low Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/big_number_calculator/formatter.rs` (lines 18-23)

**Description:**
The `group_digits` function handles negative numbers by splitting on the first `-` character: `if text.starts_with('-') { ("-", &text[1..]) }`. This works for decimal but may produce unexpected results for hex output of negative numbers. Ruby's `to_s(16)` for negative integers produces strings like `"-ff"` (not two's complement). The formatter would produce `-&nbsp;&nbsp;ff ` for this, which is correct.

However, the slice `&text[1..]` is safe only because the output from Ruby consists of ASCII characters. If `text` were empty-after-minus (e.g., just `"-"`), `&text[1..]` would be an empty string, and the function would produce `"-"` with nothing else, which is harmless.

TODO BUT THERE ACTUALLY IS A BUG, -5 IN HEX/DEC/OCTAL HAVE DIFFERENT AMOUNTS OF LEADING SPACES??

No action needed.

---

## BUG-05-17: Checksums -- LM hash parity bit calculation difference from PHP [Not a Bug]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/checksums.rs` (lines 409-426)
- `/home/taylor/defuse-rewrite/defuse.ca/src/pages/services/checksums.php` (lines 72-99)

**Description:**
The Rust `seven_to_eight_bytes` function computes DES key bytes differently from the PHP `LMhash_DESencrypt` function. The Rust version extracts 7-bit values then shifts left and adds parity bits. The PHP version puts key bits in bits 7-1 directly without explicit parity.

Analysis confirms the key bits (bits 7-1) are identical between both implementations. Only the parity bit (bit 0) differs. DES ignores parity bits during key scheduling, so both implementations produce the same encryption output.

No action needed.

---

## BUG-05-19: x86 assembler error messages not HTML-escaped in error display path [Not a Bug]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/online_x86_assembler.rs` (lines 96-98)
- `/home/taylor/defuse-rewrite/defuse-rust/templates/pages/services/online_x86_assembler.html` (line 58)

**Description:**
Error messages are HTML-escaped via `html_escape::escape_text()` in `format_error()`, then rendered with `{{ err|safe }}` in the template. The `escape_text()` function converts `<`, `>`, `&`, `"`, `'` to HTML entities. This is correct.

Assembly results (`hex_zero_bold`) contain raw HTML (`<b>00</b>`) and are rendered with `|safe`. The `hex_zero_bold` field is constructed from objdump output that has been processed to contain only hex characters and the literal strings `<b>` and `</b>`. The objdump output is controlled by the server (not user input), so this is safe.

The `string_literal` and `array_literal` fields are rendered WITHOUT `|safe` (Askama auto-escapes), which is correct since they contain user-influenced hex data.

TODO: add the same asserting that escaping the string results in an identical string

No action needed. Escaping is handled correctly.

---

## BUG-05-20: Checksums page -- file upload with empty file name still processed [Informational]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/checksums.rs` (lines 151-167)

**Description:**
The file upload handler checks `field.filename.is_some()` to determine if a file was uploaded. Some browsers submit multipart forms with a file field that has an empty filename when no file is selected. If the form framework parses this as `filename: Some("")` rather than `filename: None`, the code would process an empty file (producing hashes of empty input). The PHP version uses `file_exists($_FILES['filetohash']['tmp_name'])` which would fail for no-file submissions.

**Recommendation:**
Also check that the filename is non-empty and/or that the file data is non-empty before processing.

TODO: but we do want to support hashing empty files

# Bug Review: Interactive Services

Scope: checksums, html_sanitize, online_x86_assembler, quantum_computer_time_capsule, big_number_calculator, web_server_scan, and all supporting libraries (big_number_calculator/*, x86_assembler/*, html_escape, timecapsule, breach).

---

## BUG-05-01: Big number calculator output is rendered as raw HTML without sanitization [Medium-High Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/big_number_calculator/mod.rs` (lines 130-134)
- `/home/taylor/defuse-rewrite/defuse-rust/templates/pages/services/big_number_calculator.html` (line 34)

**Description:**
The template renders `{{ res.output|safe }}`, meaning the output string is injected as raw HTML. The `output` field is built from the Ruby process's stdout (via `value` in `EvalSuccess`). While the two-layer input validation (character whitelist + AST parser) prevents injecting arbitrary Ruby code, the _output_ from Ruby is never HTML-escaped before being embedded in the page.

The defense-in-depth concern is: if an attacker found a way to make Ruby emit HTML-special characters (e.g., through some edge case in float formatting, error messages leaking through, or a future regression in the filter), those characters would be rendered as HTML.

The PHP version has the same pattern -- it echoes `$res` directly into a `<div>` -- but the Rust version is slightly riskier because it adds `&nbsp;` and `<br />` and `<div>` via `group_digits` and `newlines_to_br`, making the `|safe` filter necessary. The Ruby output itself (`value`) should be HTML-escaped _before_ being passed through `group_digits` and `newlines_to_br`, rather than trusting the input validation to guarantee safe output.

**Recommendation:**
HTML-escape the `value` string from Ruby before running it through `group_digits`/`newlines_to_br`. The `&nbsp;`, `<br />`, and `<div>` tags added by the formatter are trusted and can remain unescaped.

---

## BUG-05-02: Big number calculator `$SAFE = 1` removed without equivalent [Low Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/big_number_calculator/evaluator.rs` (lines 78-88)
- `/home/taylor/defuse-rewrite/defuse.ca/src/pages/services/big-number-calculator.php` (lines 94-99)

**Description:**
The PHP version sets `$SAFE = 1` in the Ruby code, which was a Ruby security feature that restricted potentially dangerous operations (file I/O, system calls, etc.) on tainted data. The Rust version omits this entirely.

While `$SAFE` was deprecated in Ruby 2.7 and turned into a no-op in Ruby 3.0+, if the server is running an older Ruby version, this provides an additional defense layer. The Rust version relies entirely on the character whitelist + AST parser + ulimit, which should be sufficient, but removing a defense layer without noting it is a regression.

**Recommendation:**
Add a comment documenting that `$SAFE` was intentionally omitted because it is a no-op in Ruby 3.x. If Ruby 2.x support is needed, consider adding `$SAFE = 1;` back to the Ruby code for defense in depth.

---

## BUG-05-03: Big number calculator `or` replacement corrupts the word `"xor"` if applied in wrong order -- but matches PHP [Informational]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/big_number_calculator/filter.rs` (lines 54-70)

**Description:**
The `transform_operators` function replaces `"xor"` first, then `"or"`. This is correct and matches PHP. However, there is a subtle issue that both PHP and Rust share: the `"or"` replacement is a naive substring match, so a user typing `0xdeadf or 5` works correctly, but constructs like `short or tall` would have `or` in `short` replaced -- though `s`, `h`, `t` would fail the whitelist, so this is harmless. The Rust and PHP implementations are identical here.

No action needed. Documented for completeness.

---

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

## BUG-05-05: Time capsule date format differs from PHP's `DateTime::ATOM` [Medium Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/quantum_computer_time_capsule.rs` (line 116)

**Description:**
PHP's `DateTime::ATOM` format is `Y-m-d\TH:i:sP`, which produces output like `2024-01-15T12:30:00+00:00`. The Rust code uses chrono's `%Y-%m-%dT%H:%M:%S%:z`, which produces the same format for UTC: `2024-01-15T12:30:00+00:00`. Since `chrono::Utc::now()` always uses UTC, `%:z` will always produce `+00:00`, matching PHP's `P` specifier for UTC.

This appears correct. No action needed.

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

## BUG-05-10: Checksums page -- hash results not HTML-escaped in template [Not a Bug]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/templates/pages/services/checksums.html` (lines 22-23)

**Description:**
The template uses `{{ result.hash }}` which in Askama is auto-escaped by default. Hash outputs are hex strings so they cannot contain HTML-special characters. The algorithm names are `&'static str` constants so they are also safe. NTLM error messages could contain brackets, but Askama's auto-escaping handles this.

No action needed. Askama auto-escaping is correct here.

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

## BUG-05-12: HTML sanitize page -- `get_source_html()` called on every request [Low Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/html_sanitize.rs` (lines 99-104)

**Description:**
`get_source_html()` calls `vim_highlight::highlight_file()` on every GET and POST request. This spawns a vim process to syntax-highlight the PHP source file. The vim_highlight module likely has caching (based on the storage directory mentioned in CLAUDE.md), but if the cache is cold, every page load triggers a vim process.

**Recommendation:**
Verify that vim_highlight has effective caching. If it does, this is fine. If not, consider caching the result in a `OnceLock` or similar static cache.

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

No action needed.

---

## BUG-05-15: BREACH mitigation module is unused [Informational]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/breach.rs`

**Description:**
The `breach.rs` module has `#[allow(dead_code)]` on all public functions and comments saying "These are not actually used by the site." The PHP version uses `breach_encode`/`breach_decode` for CSRF token protection. The Rust version appears to use a different CSRF approach.

The `breach_visual_html` function is potentially useful for the time capsule's encrypted message display, but it is not currently used.

No action needed, but worth noting that BREACH protections from the PHP site have not been ported to forms that might benefit from them.

---

## BUG-05-16: Time capsule textarea contents use `&#x27;` for apostrophes instead of `&#039;` [Low Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/quantum_computer_time_capsule.rs` (lines 180-181)
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/util.rs` (line 58)

**Description:**
The `textarea_contents_escaped()` method uses `util::html_escape` which encodes `'` as `&#x27;`. The PHP version uses `htmlentities($textarea_contents, ENT_QUOTES)` which encodes `'` as `&#039;`. Both are functionally equivalent HTML entities for the apostrophe character, but if byte-exact parity with PHP is desired, these differ.

For the time capsule, this only affects the textarea re-fill on error, not the stored data or archive, so it has no functional impact. The encrypted message display uses the same `util::html_escape` but since the encrypted data should not contain apostrophes (it is hex/base64), this is unlikely to manifest.

**Recommendation:**
If HTML output parity with PHP is a goal (per CLAUDE.md), change `util::html_escape` to use `&#039;` for single quotes. Low priority since it does not affect functionality.

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

## BUG-05-18: Checksums -- NTLM hash error message differs from PHP on invalid UTF-8 [Low Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/checksums.rs` (lines 446-456)

**Description:**
When given non-UTF-8 input, the PHP version uses `@iconv('UTF-8','UTF-16LE',$Input)` which silently produces an empty string on invalid UTF-8, then hashes that empty string. The Rust version returns `"[error: NTLM requires valid text]"` as the hash value.

For text input via the form, this difference is irrelevant because form data is always valid UTF-8. For file uploads, the file is processed as raw bytes, and the NTLM hash receives those bytes. If the file contains non-UTF-8, PHP would hash an empty UTF-16LE string (`iconv` returns empty), while Rust shows an error message.

**Recommendation:**
To match PHP behavior exactly, when UTF-8 decoding fails, hash an empty byte array (or produce `md4("")` as the hash). This matters only for file uploads with non-UTF-8 content. Low priority.

---

## BUG-05-19: x86 assembler error messages not HTML-escaped in error display path [Not a Bug]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/online_x86_assembler.rs` (lines 96-98)
- `/home/taylor/defuse-rewrite/defuse-rust/templates/pages/services/online_x86_assembler.html` (line 58)

**Description:**
Error messages are HTML-escaped via `html_escape::escape_text()` in `format_error()`, then rendered with `{{ err|safe }}` in the template. The `escape_text()` function converts `<`, `>`, `&`, `"`, `'` to HTML entities. This is correct.

Assembly results (`hex_zero_bold`) contain raw HTML (`<b>00</b>`) and are rendered with `|safe`. The `hex_zero_bold` field is constructed from objdump output that has been processed to contain only hex characters and the literal strings `<b>` and `</b>`. The objdump output is controlled by the server (not user input), so this is safe.

The `string_literal` and `array_literal` fields are rendered WITHOUT `|safe` (Askama auto-escapes), which is correct since they contain user-influenced hex data.

No action needed. Escaping is handled correctly.

---

## BUG-05-20: Checksums page -- file upload with empty file name still processed [Informational]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/checksums.rs` (lines 151-167)

**Description:**
The file upload handler checks `field.filename.is_some()` to determine if a file was uploaded. Some browsers submit multipart forms with a file field that has an empty filename when no file is selected. If the form framework parses this as `filename: Some("")` rather than `filename: None`, the code would process an empty file (producing hashes of empty input). The PHP version uses `file_exists($_FILES['filetohash']['tmp_name'])` which would fail for no-file submissions.

**Recommendation:**
Also check that the filename is non-empty and/or that the file data is non-empty before processing.

---

## Summary

| ID | Severity | Component | Issue |
|----|----------|-----------|-------|
| 05-01 | Medium-High | Big Number Calculator | Ruby output rendered as raw HTML without escaping |
| 05-02 | Low | Big Number Calculator | Missing `$SAFE = 1` (no-op in Ruby 3.x) |
| 05-04 | Medium | Time Capsule | `from_utf8_lossy` may corrupt archive bytes |
| 05-06 | Low | Time Capsule | Form fields not validated |
| 05-09 | Low | x86 Assembler | No size limit on hex input string before parsing |
| 05-11 | Low | HTML Sanitize | Error erases user input |
| 05-16 | Low | Time Capsule | `&#x27;` vs `&#039;` apostrophe encoding |
| 05-18 | Low | Checksums | NTLM error behavior differs from PHP for invalid UTF-8 |
| 05-20 | Informational | Checksums | Empty filename file upload edge case |

**Most critical finding: BUG-05-04** (time capsule archive `from_utf8_lossy`) because it could break blockchain hash verification of the archive, which is the core integrity guarantee of the time capsule feature. The archive comments explicitly say the output must be byte-for-byte identical.

**Second most critical: BUG-05-01** (big number calculator XSS) because it is a defense-in-depth gap where Ruby output is trusted to be HTML-safe without explicit escaping.

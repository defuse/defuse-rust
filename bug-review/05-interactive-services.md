# Bug Review: Interactive Services

Scope: checksums, html_sanitize, online_x86_assembler, quantum_computer_time_capsule, big_number_calculator, web_server_scan, and all supporting libraries (big_number_calculator/*, x86_assembler/*, html_escape, timecapsule, breach).

## BUG-05-07: x86 assembler -- `check_code_safety` case-sensitive directive matching [Low Severity]

**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/libs/x86_assembler/filter.rs` (lines 78-81)

**Description:**
The safe directive removal uses exact string replacement: `filtered = filtered.replace(directive, "");`. GAS directives are case-insensitive, so `.BYTE` or `.Ascii` would not be matched by the whitelist and would be rejected (because the `.` remains after filtering). This is actually the SAFE direction -- it means `.INCLUDE` and `.Fill` are also rejected. The PHP version behaves identically (`str_replace` is case-sensitive). This is correct behavior: being overly strict is safe.

No action needed. Documented for completeness.

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

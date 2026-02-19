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
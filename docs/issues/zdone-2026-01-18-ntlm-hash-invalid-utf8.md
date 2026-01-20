# NTLM hash returns empty-string hash for invalid UTF-8 input

**Date:** 2026-01-18

**Severity:** Bug (data correctness)

**Affects:** defuse.ca checksums page

## Problem

When computing NTLM hashes for binary data containing invalid UTF-8 sequences (e.g., bytes 0x80-0xFF), the PHP implementation returns the MD4 hash of an empty string instead of the correct NTLM hash.

## Root Cause

In `/src/pages/services/checksums.php`, the `NTLMHash` function:

```php
function NTLMHash($Input)
{
    // Convert the password from UTF8 to UTF16 (little endian)
    $Input=@iconv('UTF-8','UTF-16LE',$Input);
    $MD4Hash=hash('md4',$Input, true);
    return $MD4Hash;
}
```

The `@iconv('UTF-8','UTF-16LE',$Input)` call:
1. Assumes input is valid UTF-8
2. Silently fails (due to `@` error suppression) when input contains invalid UTF-8 bytes
3. Returns `false` on failure
4. `hash('md4', false, true)` coerces `false` to empty string
5. Result is always `31d6cfe0d16ae931b73c59d7e0c089c0` (MD4 of empty string) for any invalid UTF-8 input

## Example

Input bytes: `[0x80, 0x81, 0x82, 0x83]` (invalid UTF-8)

- **Expected NTLM:** Hash of UTF-16LE representation treating bytes as Latin-1 code points
- **Actual NTLM:** `31d6cfe0d16ae931b73c59d7e0c089c0` (hash of empty string)

## Correct Behavior

For binary data, NTLM should either:
1. Treat each byte as a Latin-1 (ISO-8859-1) code point and convert to UTF-16LE (byte 0xXX becomes `[0xXX, 0x00]`)
2. Or explicitly reject/warn about invalid UTF-8 input instead of silently returning wrong hash

## Rust Implementation

The Rust rewrite currently matches PHP's buggy behavior for compatibility. The fix would be:

```rust
// Correct Latin-1 behavior (not currently used):
let utf16: Vec<u8> = password.iter().flat_map(|&b| [b, 0u8]).collect();
```

## Recommendation

Fix the PHP implementation to handle binary data correctly, or at minimum remove the `@` suppression and warn users when input is not valid UTF-8.

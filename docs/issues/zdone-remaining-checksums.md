# Issue: Implement remaining checksum algorithms

## Summary

Implement the remaining 23 hash algorithms for the checksums page to achieve feature parity with the PHP version.

## Current Status

34 of 57 hash algorithms are implemented. The remaining 23 are:

### HAVAL (15 variants)
- haval128,3 / haval128,4 / haval128,5
- haval160,3 / haval160,4 / haval160,5
- haval192,3 / haval192,4 / haval192,5
- haval224,3 / haval224,4 / haval224,5
- haval256,3 / haval256,4 / haval256,5

### Tiger (6 variants)
- tiger128,3 / tiger128,4
- tiger160,3 / tiger160,4
- tiger192,3 / tiger192,4

### Snefru (2 variants)
- snefru
- snefru256

## Implementation Challenges

### HAVAL
- No well-maintained Rust crate exists
- Options:
  1. Check crates.io for `haval` or `haval-rs`
  2. Port from PHP's mhash extension source
  3. Wrap a C library (e.g., via bindgen)
  4. Custom implementation from the HAVAL paper

### Tiger
- The `tiger` crate exists but may only support 3-pass variants
- PHP's mhash has both 3-pass and 4-pass Tiger
- May need to fork/extend the crate or implement 4-pass manually

### Snefru
- Obscure algorithm from 1990
- No known Rust crate
- Will likely need custom implementation or C wrapper

## Test Vectors

Test vectors for input "test" are available in `defuse-tester/tests/checksums.rs`.

## Priority

Low - these are obscure/legacy algorithms. The commonly-used hashes (SHA-2, SHA-3, MD5, etc.) are already implemented.

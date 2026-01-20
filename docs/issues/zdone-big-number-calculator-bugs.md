# Big Number Calculator Bugs

Two bugs in the PHP implementation that should be fixed in the Rust port.

## 1. Rational results cause silent failure

**Current behavior:** Expressions like `2^-3` fail with "wasn't recognized as a valid mathematical expression"

**Root cause:** Ruby returns a `Rational` type (`1/8`) for negative integer exponents. The PHP code only handles `Float`, `Fixnum`, and `Bignum`:

```ruby
"puts x if x.is_a?(Float); " .
"puts x.to_s($base) if x.is_a?(Fixnum) or x.is_a?(Bignum)"
```

Since `Rational` isn't checked, nothing is printed, result is blank, and the error message is shown.

**Expected behavior:** `2^-3` should output `0.125`

## 2. Float results ignore output base setting

**Current behavior:** `0.125` with hex base selected outputs `0.125` (decimal)

**Root cause:** Float results use `puts x` which doesn't apply base conversion:

```ruby
"puts x if x.is_a?(Float); " .                              // no base conversion
"puts x.to_s($base) if x.is_a?(Fixnum) or x.is_a?(Bignum)"  // has base conversion
```

**Expected behavior:** Float results should be converted to the selected base (hex, octal) for display.

## Notes

- Tests in `defuse-tester` document the current (buggy) defuse.ca behavior
- When implementing in Rust, handle both cases properly

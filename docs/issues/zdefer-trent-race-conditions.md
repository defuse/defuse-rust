# TRENT Race Conditions

## Summary

The TRENT (Trusted Random Entropy) service has race conditions that cause intermittent test failures when multiple requests hit the server concurrently.

## Symptoms

When running the trent.rs integration tests in parallel, some tests intermittently fail:

- `extract_drawing_number()` returns `None` - server returns error page instead of success
- `confirm_step2()` doesn't return "Drawing Complete!" - server returns error instead

Affected tests include:
- `xss_prevention_name`
- `view_completed_drawing`
- `view_completed_has_results`
- `view_completed_has_verification_info`
- `review_period_enforced`
- `max_numbers_1000`
- `random_single_number`
- `file_sha256_recorded`

Tests pass reliably when run serially (`cargo test -- --test-threads=1`).

## Likely Causes

1. **Drawing number reservation** - The "get next number" + "insert" sequence may not be atomic, causing conflicts when multiple requests reserve simultaneously.

2. **Database transaction conflicts** - Concurrent writes to the same tables without proper locking.

3. **Review period edge cases** - Tests use very short review periods (some 0 seconds), which may interact poorly with concurrent requests.

## Workaround

Run tests serially: `cargo test -- --test-threads=1`

## Fix

Investigate the PHP code in `trustedthirdparty.php` for:
- Non-atomic read-modify-write sequences
- Missing database transactions/locks
- Shared state that assumes single-threaded access

# TRENT Unicode Bug

**Status:** OPEN
**Discovered:** 2026-01-16
**Test:** `tests/trent.rs::name_with_unicode`

## Summary

When creating a TRENT drawing with Unicode characters in the name or description field, the server responds with "Drawing Complete!" but the drawing is NOT actually saved as complete in the database.

## Steps to Reproduce

1. Go to https://defuse.ca/trustedthirdparty.htm
2. Reserve a drawing with instant review time (0)
3. Fill in the drawing with:
   - Name: `Über Tëst 日本語 🎉` (or any Unicode)
   - Description: anything
   - Random numbers: 1 number between 1 and 100
4. Click "Pick the Random Numbers!"
5. On confirmation page, click "These values are correct, draw my random numbers!"
6. Server responds with "Drawing Complete!" and provides a URL
7. Click the URL to view results
8. **BUG:** Page shows "The random numbers for this drawing have not yet been chosen" instead of results

## Expected Behavior

The drawing should be saved and viewable with results after server confirms completion.

## Actual Behavior

- Server returns HTTP 200 with "Drawing Complete!" message
- Database write silently fails (likely due to character encoding issue)
- Viewing the drawing shows it as incomplete

## Workaround

Use ASCII-only characters in name and description fields.

## Technical Notes

- The bug appears to be in the PHP database write code
- Possible causes:
  - Database column not using UTF-8 encoding
  - Missing `SET NAMES utf8` or equivalent
  - PHP `htmlentities()` or similar corrupting the data before insert
  - MySQL strict mode rejecting invalid characters silently

## Test Coverage

The test `name_with_unicode` in `tests/trent.rs` catches this bug and is marked as `#[ignore]` until fixed.

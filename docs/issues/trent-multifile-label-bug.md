# TRENT Multi-File Random Line Label Bug

**Status:** OPEN
**Discovered:** 2026-01-16
**Test:** `tests/trent.rs::file_multiple_files_correct_labels`

## Summary

When multiple files are uploaded and random lines are selected from each file (FILE1, FILE2, FILE3), the results incorrectly label ALL random lines as coming from "FILE1" regardless of which file they actually came from.

## Steps to Reproduce

1. Reserve a drawing with instant review time
2. Upload 3 different files with distinct content
3. Request 1 random line from each file (randlines1=1, randlines2=1, randlines3=1)
4. Complete the drawing
5. View the results

## Expected Behavior

Results should show:
```
FILE1 RANDOM LINE 1:
RANDOM LINE NUMBER (FILE1): X
LINE PREVIEW: [content from file 1]

FILE2 RANDOM LINE 1:
RANDOM LINE NUMBER (FILE2): Y
LINE PREVIEW: [content from file 2]

FILE3 RANDOM LINE 1:
RANDOM LINE NUMBER (FILE3): Z
LINE PREVIEW: [content from file 3]
```

## Actual Behavior

All random lines are labeled as FILE1:
```
FILE1 RANDOM LINE 1:
RANDOM LINE NUMBER (FILE1): X
LINE PREVIEW: [content from file 1]

FILE1 RANDOM LINE 1:
RANDOM LINE NUMBER (FILE1): Y
LINE PREVIEW: [content from file 2]  <-- Actually from FILE2!

FILE1 RANDOM LINE 1:
RANDOM LINE NUMBER (FILE1): Z
LINE PREVIEW: [content from file 3]  <-- Actually from FILE3!
```

## Impact

- Users cannot easily verify which file a random line came from
- The LINE PREVIEW shows correct content, but the label is wrong
- SHA256 hashes for all 3 files ARE correctly labeled (FILE1 SHA256, FILE2 SHA256, FILE3 SHA256)

## Technical Notes

The bug is likely in the PHP code that generates the results output. The loop that outputs random lines probably hardcodes "FILE1" instead of using a variable for the file number.

## Test Coverage

- `file_multiple_files` - Tests that all 3 files are processed (passes by checking content)
- `file_multiple_files_correct_labels` - Tests that labels are correct (ignored until fixed)

## Rust Implementation Note

When implementing TRENT in Rust, ensure the random line output loop uses the correct file number variable (1, 2, or 3) instead of hardcoding "FILE1". The test `file_multiple_files_correct_labels` should pass once correctly implemented.

# Vim Syntax Highlighting Differences

## Summary

The vim syntax highlighting output differs between the production server and the local Rust rewrite due to different vim versions producing different HTML class names for syntax elements.

## Affected Pages

- `blind-birthday-attack` (Ruby syntax highlighting)
- Any other pages using vim-based syntax highlighting via `hl_file()`

## Current Workaround

The snapshot comparison tool elides lines containing these span classes to avoid noisy diffs:
- `<span class="Identifier`
- `<span class="Constant`
- `<span class="Special`
- `<span class="Statement`

See `src/bin/snapshot.rs` in `normalize_html()`.

## Root Cause

Different vim versions (and potentially different `:TOhtml` configurations) produce different CSS class names for the same syntax elements. For example, what one vim version marks as `Constant` another might mark as `Special`.

## Tab Expansion Behavior

The `show_lines` (line numbers) setting affects how tabs are rendered in the HTML output:

- **`show_lines = false`** (no line numbers): Tabs are preserved as literal `\t` characters in prod
- **`show_lines = true`** (with line numbers): Tabs are expanded to spaces in prod

The current `vim_highlight.rs` has `let g:html_expand_tabs = 0` which preserves tabs. This matches the `show_lines = false` case but not `show_lines = true`.

**Workaround**: For pages with `show_lines = true` that have tab characters, manually replace tabs with the correct number of spaces in the template to match prod output.

## Potential Solutions

1. **Pin vim version** - Ensure the same vim version is used in production and development
2. **Normalize at generation time** - Post-process the vim HTML output to use consistent class names
3. **Use a different highlighter** - Replace vim-based highlighting with a Rust-native solution (e.g., syntect) that produces consistent output
4. **Accept the difference** - If the visual result is acceptable, just keep the elision in the test tool

## Priority

Low - The highlighting still works correctly, just with slightly different class names. This is a cosmetic difference that doesn't affect functionality.

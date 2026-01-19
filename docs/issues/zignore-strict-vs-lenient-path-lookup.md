# Strict vs lenient path lookup

## Problem

`lookup_page_from_path` is lenient - it handles case differences and strips extensions (.htm, .html). This is needed for URL canonicalization to find pages and redirect.

However, some use cases should probably be strict:

- **upvote_post.rs** - should only process POSTs to canonical URLs. If someone POSTs to `/About.htm` instead of `/about.htm`, something is wrong (form actions use canonical URLs).

## Current behavior

All callers use the same lenient `lookup_page_from_path`:
- `url_canonicalization.rs` - needs lenient (to find page and redirect)
- `dispatcher.rs` - needs lenient (handles redirects too)
- `security_headers.rs` - probably fine either way
- `upvote_post.rs` - should probably be strict

## Potential fix

Add a helper to check if a path is already canonical:

```rust
pub fn is_canonical_path(path: &str) -> bool {
    // Returns true only if path exactly matches canonical form
}
```

Then in `upvote_post.rs`:
```rust
if !is_canonical_path(path) {
    return next.run(request).await;
}
```

## Status

Low priority - POSTs to non-canonical URLs would be unusual (form actions are canonical).

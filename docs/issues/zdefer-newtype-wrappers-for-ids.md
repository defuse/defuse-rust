# Missing Newtype Wrappers for IDs

## Problem

Page IDs and upvote IDs are passed as raw `&str` throughout the codebase. This makes it easy to accidentally pass the wrong type of string:

- URL slug when page ID expected
- Upvote ID when hit counter ID expected
- etc.

## Examples of Potential Confusion

```rust
// These are all &str - easy to mix up
phpcount.add_hit(page_id, client_ip, user_agent)
upvotes.process_vote(permanent_id, client_ip, direction)
upvotes.ensure_page(id, category, title, description, url)
```

## Proposed Solution

Add newtype wrappers:

```rust
pub struct HitCounterId(pub &'static str);
pub struct UpvoteId(pub &'static str);
```

Then the compiler catches mistakes:
```rust
// Won't compile - type mismatch
phpcount.add_hit(upvote_id, ...)  // Error: expected HitCounterId, got UpvoteId
```

## Priority

Low - this is a "nice to have" for type safety, not a bug fix.

## Notes

- Currently `legacy_hit_count_id` and upvote `id` are different values for the same page
- The registry already has both fields, so the mapping exists
- Main benefit is compile-time prevention of ID mixups

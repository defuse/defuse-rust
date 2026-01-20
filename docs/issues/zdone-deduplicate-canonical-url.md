# Deduplicate canonical_url implementations

## Problem

There are multiple places dealing with canonical URLs:

1. **`ctx.canonical_url()`** in `context.rs` - Returns full absolute URLs like `https://defuse.ca:443/about.htm`. Used for upvote form actions in `base.html`.

2. **`canonical_url(slug)`** in `registry/mod.rs` - Returns relative paths like `/about.htm`. Used for redirects and URL canonicalization.

3. **`UpvotePageInfo.canonical_url`** field - Stored in the upvotes database, used on the home page upvote list.

This duplication is confusing and could lead to inconsistencies.

## Constraints

- The upvote form actions MUST include `:443` in the URL to match PHP output (e.g., `https://defuse.ca:443/about.htm`)
- The redirect/canonicalization logic needs relative paths
- The database stores full URLs for the upvote list links

## Suggested approach

Consider consolidating into a single source of truth for URL generation, with different output formats:
- `canonical_path(slug)` - returns `/about.htm`
- `canonical_url(slug)` - returns `https://defuse.ca/about.htm`
- `canonical_url_with_port(slug)` - returns `https://defuse.ca:443/about.htm` (for upvote forms)

Or use a builder pattern / config to specify what format is needed.

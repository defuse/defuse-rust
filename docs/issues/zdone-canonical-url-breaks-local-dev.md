# Canonical URLs break local development links

## Problem

`ctx.canonical_url()` in `context.rs` always returns full URLs with `https://defuse.ca/...` hardcoded. This is used for:

- Upvote form `action` attributes in templates
- Links in the top pages list on home page
- `canonical_url` field stored in the upvotes database

During local development, clicking these links or submitting forms would navigate to production instead of localhost.

## Current behavior (matches PHP)

PHP does the same thing - each page hardcodes its full canonical URL:

```php
Upvote::render_arrows(
    "myvimrc",
    "defuse_pages",
    "My Vim Configuration",
    "...",
    "https://defuse.ca/vimrc.htm"  // Full URL
);
```

## Potential fix

For internal navigation (forms, page lists), use relative paths like `/about.htm` instead of full URLs. Reserve full canonical URLs for:

- SEO `<link rel="canonical">` meta tags
- Open Graph / social sharing meta tags
- External links

## Status

Low priority - matches existing PHP behavior.

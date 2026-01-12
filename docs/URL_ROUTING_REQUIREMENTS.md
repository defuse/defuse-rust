# URL Routing System Requirements

## Overview

This document specifies the requirements for the URL routing system that replaces `URLParse.php`. The goal is to maintain identical behavior while providing a clear, maintainable architecture.

## Single Source of Truth: The Page Registry

In PHP, the `$PAGE_INFO` array is the single source of truth. In Rust, we will have a similar central registry.

### Adding a New Page (Developer Workflow)

To add a new page, a developer should:

1. **Add an entry to the page registry** (`src/pages/registry.rs`)
   - Define the canonical name (e.g., `"checksums"`)
   - Specify metadata: title, description, keywords
   - For static pages: specify the template path
   - For dynamic pages: specify the handler module
   - For aliases: specify the redirect target

2. **Create the page content**
   - For static pages: create `templates/pages/{name}.html`
   - For dynamic pages: create `src/pages/{name}.rs` with handler functions

3. **Done** - routing is automatic from the registry

### Page Registry Structure

```rust
// src/pages/registry.rs

pub struct PageInfo {
    /// Template file path (for static pages) or None for dynamic
    pub template: Option<&'static str>,

    /// Handler module name (for dynamic pages) or None for static
    pub handler: Option<&'static str>,

    /// Page title (None = use default)
    pub title: Option<&'static str>,

    /// Meta description (None = use default)
    pub meta_description: Option<&'static str>,

    /// Meta keywords (None = use default)
    pub meta_keywords: Option<&'static str>,

    /// Redirect target (if set, this is an alias - takes precedence)
    pub redirect: Option<&'static str>,

    /// Is this a directory-style URL (ends with /, no .htm)
    pub is_directory: bool,
}

pub static PAGE_REGISTRY: &[(&str, PageInfo)] = &[
    ("", PageInfo { /* home page */ }),
    ("about", PageInfo { /* about page */ }),
    ("checksums", PageInfo { /* checksums page */ }),
    // ... all pages
];
```

---

## URL Processing Requirements

### Requirement 1: Host Canonicalization

**R1.1** Requests to non-canonical hosts MUST redirect to the master host.
- Master host: `defuse.ca`
- Accepted hosts (no redirect): `localhost`, `127.0.0.1`, configured dev hosts
- Redirect type: 301 Moved Permanently

**R1.2** Accepted hosts MUST be configurable via environment variable.
- `ACCEPTED_HOSTS=localhost,127.0.0.1,192.168.1.102`

**R1.3** Master host MUST be configurable via environment variable.
- `MASTER_HOST=defuse.ca`

**Test cases:**
- [ ] Request to `defuse.ca/page.htm` → no redirect
- [ ] Request to `www.defuse.ca/page.htm` → 301 to `defuse.ca/page.htm`
- [ ] Request to `localhost/page.htm` → no redirect (accepted host)
- [ ] Request to `evil.com/page.htm` → 301 to `defuse.ca/page.htm`

---

### Requirement 2: HTTPS Enforcement

**R2.1** HTTP requests MUST redirect to HTTPS when `FORCE_HTTPS=true`.
- Redirect type: 301 Moved Permanently

**R2.2** HTTPS enforcement MUST be bypassed for accepted hosts.
- This allows local development without TLS

**R2.3** HTTPS enforcement MUST be configurable via environment variable.
- `FORCE_HTTPS=true` (default in production)
- `FORCE_HTTPS=false` (for development)

**Test cases:**
- [ ] HTTP request with `FORCE_HTTPS=true` → 301 to HTTPS
- [ ] HTTP request to `localhost` with `FORCE_HTTPS=true` → no redirect
- [ ] HTTPS request → no redirect
- [ ] HTTP request with `FORCE_HTTPS=false` → no redirect

---

### Requirement 3: URL Canonicalization

**R3.1** Page URLs without `.htm` extension MUST redirect to the `.htm` version.
- `/about` → 301 to `/about.htm`
- `/checksums` → 301 to `/checksums.htm`

**R3.2** Page URLs with `.htm` extension MUST NOT redirect.
- `/about.htm` → serve page (no redirect)

**R3.3** Directory-style URLs MUST end with `/` and NOT have `.htm`.
- `/audits` → 301 to `/audits/`
- `/audits/` → serve page (no redirect)
- `/audits/.htm` → 404 (invalid)

**R3.4** The home page MUST be served at `/` without `.htm`.
- `/` → serve home page
- `/.htm` → 404 (invalid)
- `/index` → 301 to `/`
- `/index.htm` → 301 to `/`
- `/index.html` → 301 to `/`
- `/index.php` → 301 to `/`

**R3.5** URL matching MUST be case-insensitive.
- `/About.htm` → 301 to `/about.htm`
- `/CHECKSUMS.HTM` → 301 to `/checksums.htm`

**R3.6** URL parameters MUST be preserved across redirects.
- `/about?foo=bar` → 301 to `/about.htm?foo=bar`

**Test cases:**
- [ ] `/about` → 301 to `/about.htm`
- [ ] `/about.htm` → 200 OK (serve page)
- [ ] `/About` → 301 to `/about.htm`
- [ ] `/audits` → 301 to `/audits/`
- [ ] `/audits/` → 200 OK (serve page)
- [ ] `/` → 200 OK (serve home)
- [ ] `/index` → 301 to `/`
- [ ] `/index.htm` → 301 to `/`
- [ ] `/page?x=1` → 301 to `/page.htm?x=1`

---

### Requirement 4: Aliases and Redirects

**R4.1** Alias URLs MUST redirect to their target URL.
- Redirect type: 301 Moved Permanently

**R4.2** Alias redirects MUST go to the canonical form of the target.
- `/trent` → 301 to `/trustedthirdparty.htm` (not `/trustedthirdparty`)

**R4.3** Known aliases from PHP site:
```
/index, /index.html, /index.php → /
/key → /contact.htm
/audits/ → /software-security-auditing.htm
/trent → /trustedthirdparty.htm
/passwords, /password, /pass → /passgen.htm
/pphos → /password-policy-hall-of-shame.htm
/bh2016, /BH2016 → /side-channel-attacks-on-everyday-applications.htm
/keyboarddefect → /asuskeyboarddefect.htm
```

**Test cases:**
- [ ] `/trent` → 301 to `/trustedthirdparty.htm`
- [ ] `/trent.htm` → 301 to `/trustedthirdparty.htm`
- [ ] `/passwords` → 301 to `/passgen.htm`
- [ ] `/key` → 301 to `/contact.htm`

---

### Requirement 5: 404 Handling

**R5.1** Unknown URLs MUST return HTTP 404 status.

**R5.2** 404 responses MUST render a custom 404 page.

**R5.3** The 404 page MUST use the standard site template (header, nav, footer).

**Test cases:**
- [ ] `/nonexistent` → 404 with custom page
- [ ] `/nonexistent.htm` → 404 with custom page
- [ ] Response includes proper 404 HTTP status code

---

### Requirement 6: Security Headers

**R6.1** All responses MUST include `X-Frame-Options: SAMEORIGIN`.

**R6.2** HTTPS responses MUST include HSTS header.
- `Strict-Transport-Security: max-age=31536000; includeSubDomains; preload`
- HSTS MUST NOT be sent over HTTP
- HSTS MUST NOT be sent to localhost/accepted hosts

**R6.3** All responses MUST include `Content-Type: text/html; charset=utf-8`.

**Test cases:**
- [ ] Response includes `X-Frame-Options: SAMEORIGIN`
- [ ] HTTPS response includes HSTS header
- [ ] HTTP response does NOT include HSTS header
- [ ] Localhost response does NOT include HSTS header

---

### Requirement 7: Metadata Defaults

**R7.1** Pages without explicit title MUST use default.
- Default: `"Defuse Security Research and Development"`

**R7.2** Pages without explicit meta description MUST use default.
- Default: `"Defuse Security. Home of PIE Bin, TRENT, and more..."`

**R7.3** Pages without explicit meta keywords MUST use default.
- Default: `"defuse security, encryption, privacy, programming, code, research"`

---

## Processing Order

The URL processing MUST happen in this exact order:

1. **Host check** → redirect if not master/accepted host
2. **HTTPS check** → redirect if HTTP and FORCE_HTTPS (unless accepted host)
3. **Lowercase URL** → for case-insensitive matching
4. **Lookup page** → find in registry
5. **Alias check** → if P_RDIR set, redirect to target (in canonical form)
6. **Extension check** → redirect to `.htm` or `/` as appropriate
7. **Serve page** → render template/call handler

If any step issues a redirect, subsequent steps are skipped.

---

## Environment Variables Summary

| Variable | Default | Description |
|----------|---------|-------------|
| `MASTER_HOST` | `defuse.ca` | Canonical hostname |
| `ACCEPTED_HOSTS` | `localhost,127.0.0.1` | Hosts that skip redirects |
| `FORCE_HTTPS` | `true` | Require HTTPS (bypassed for accepted hosts) |
| `LISTEN_ADDR` | `127.0.0.1:3000` | Server bind address |

---

## Implementation Checklist

- [ ] Create `src/pages/registry.rs` with PageInfo struct and PAGE_REGISTRY
- [ ] Create `src/middleware/url_canonicalization.rs`
- [ ] Implement host canonicalization (R1)
- [ ] Implement HTTPS enforcement (R2)
- [ ] Implement URL canonicalization (R3)
- [ ] Implement aliases/redirects (R4)
- [ ] Implement 404 handling (R5)
- [ ] Implement security headers (R6)
- [ ] Implement metadata defaults (R7)
- [ ] Add all test cases
- [ ] Migrate all pages from PHP $PAGE_INFO to Rust registry

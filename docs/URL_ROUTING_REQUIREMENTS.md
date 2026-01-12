# URL Routing System Requirements

## Overview

This document specifies the requirements for the URL routing system that replaces `URLParse.php`. The goal is to maintain **identical behavior**.

**Reference:** `defuse.ca/src/libs/URLParse.php` (1,077 lines)

## Single Source of Truth: The Page Registry

In PHP, the `$PAGE_INFO` array is the single source of truth. In Rust, we will have a similar central registry in `src/pages/registry.rs`.

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

---

## Configuration (HARDCODED - matching PHP)

These values are **hardcoded** in the PHP source (lines 77-85) and MUST be hardcoded in Rust:

```rust
const MASTER_HOST: &str = "defuse.ca";
const ACCEPTED_HOSTS: &[&str] = &[
    "localhost",
    "127.0.0.1",          // Added for convenience
    "192.168.1.102",
    "defuse.h.defuse.ca",
    "defuse",
    "defuse:10443",
];
const FORCE_HTTPS: bool = true;
```

**Note:** For local development, requests to `localhost` or other accepted hosts bypass HTTPS enforcement and host canonicalization.

---

## CRITICAL: Single Redirect Optimization

**The PHP code is carefully designed to issue AT MOST ONE redirect per request.**

Each step anticipates the needs of later steps to avoid double redirects:
- `checkHost()` anticipates HTTPS requirement (line 896)
- `checkRedirectRequest()` anticipates `.htm` extension (line 966)

The Rust implementation **MUST** maintain this property. When redirecting:
- Host redirect should use `https://` if FORCE_HTTPS is true
- Alias redirect should append `.htm` to target (if not a directory)
- This way, a request like `http://www.defuse.ca/trent` becomes `https://defuse.ca/trustedthirdparty.htm` in ONE redirect, not three.

---

## URL Processing Requirements

### Requirement 1: Host Canonicalization

**R1.1** Requests to non-canonical hosts MUST redirect to master host.
- Master host: `defuse.ca` (hardcoded)
- Accepted hosts: see list above (hardcoded)
- Redirect type: 301 Moved Permanently

**R1.2** When redirecting, ANTICIPATE the HTTPS requirement:
- If `FORCE_HTTPS == true` OR current request is already HTTPS → use `https://`
- Otherwise → use `http://`

**R1.3** Preserve the full path and query string in redirect.

**Test cases:**
- [ ] `http://www.defuse.ca/about` → `301 https://defuse.ca/about` (anticipates HTTPS, but NOT .htm - that comes later)
- [ ] `http://localhost/about` → no redirect (accepted host)
- [ ] `https://evil.com/page?x=1` → `301 https://defuse.ca/page?x=1`
- [ ] `http://defuse.ca/page` when already on master → no host redirect (HTTPS redirect may still happen)

---

### Requirement 2: HTTPS Enforcement

**R2.1** HTTP requests MUST redirect to HTTPS when `FORCE_HTTPS == true`.
- Only if host is NOT in accepted hosts
- Redirect type: 301 Moved Permanently

**R2.2** HTTPS enforcement is SKIPPED for accepted hosts.
- `http://localhost/...` works without redirect
- `http://192.168.1.102/...` works without redirect

**R2.3** This check only triggers if host redirect didn't already happen.
- Host redirect anticipates HTTPS, so if we redirected for host, we won't redirect again for HTTPS

**Test cases:**
- [ ] `http://defuse.ca/about.htm` → `301 https://defuse.ca/about.htm`
- [ ] `http://localhost/about.htm` → no redirect (accepted)
- [ ] `https://defuse.ca/about.htm` → no redirect (already HTTPS)

---

### Requirement 3: Page Lookup

**R3.1** Page names are matched **case-insensitively** using `strtolower()`.
- `/About.htm` and `/about.htm` both find page `"about"`

**R3.2** The `.htm` extension is stripped for lookup.
- `/about.htm` → lookup `"about"`
- `/about` → lookup `"about"`

**R3.3** Directory pages (names ending in `/`) are special:
- `/audits/` → lookup `"audits/"`
- `/audits` when `"audits"` doesn't exist but `"audits/"` does → lookup `"audits/"`

**R3.4** Invalid URLs that should 404:
- `/.htm` → 404 (empty name + .htm is invalid)
- `/foo/.htm` → 404 (directory name + .htm is invalid)
- `/nonexistent` → 404 (not in registry)

**Test cases:**
- [ ] `/about.htm` → finds `"about"`
- [ ] `/About.htm` → finds `"about"`
- [ ] `/audits/` → finds `"audits/"`
- [ ] `/audits` (if only `"audits/"` exists) → finds `"audits/"`
- [ ] `/.htm` → 404
- [ ] `/foo/.htm` → 404

---

### Requirement 4: Alias Resolution (P_RDIR)

**R4.1** If a page has `redirect` set, redirect to that target.
- Redirect type: 301 Moved Permanently

**R4.2** When redirecting, ANTICIPATE the `.htm` extension:
- If target is empty (`""`) → redirect to `/`
- If target ends in `/` → redirect to `/{target}`
- Otherwise → redirect to `/{target}.htm`

**R4.3** Preserve query parameters.

**R4.4** Known aliases (from PHP $PAGE_INFO):
```
"index"           → ""                                    (home)
"index.html"      → ""                                    (home)
"index.php"       → ""                                    (home)
"key"             → "contact"
"audits/"         → "software-security-auditing"
"trent"           → "trustedthirdparty"
"passwords"       → "passgen"
"password"        → "passgen"
"pass"            → "passgen"
"pphos"           → "password-policy-hall-of-shame"
"bh2016"          → "side-channel-attacks-on-everyday-applications"
"BH2016"          → "side-channel-attacks-on-everyday-applications"
"keyboarddefect"  → "asuskeyboarddefect"
```

**Test cases:**
- [ ] `/trent` → `301 /trustedthirdparty.htm` (anticipates .htm)
- [ ] `/trent.htm` → `301 /trustedthirdparty.htm` (same result)
- [ ] `/index` → `301 /`
- [ ] `/index.html` → `301 /` (this is an alias, NOT a generic .html→.htm rule)
- [ ] `/index.htm` → `301 /`
- [ ] `/key?subject=hello` → `301 /contact.htm?subject=hello`
- [ ] `/audits/` → `301 /software-security-auditing.htm`

---

### Requirement 5: Extension Canonicalization

**R5.1** Non-directory pages without `.htm` MUST redirect.
- `/about` → `301 /about.htm`

**R5.2** Directory pages without trailing `/` MUST redirect.
- `/audits` → `301 /audits/` (when `"audits/"` exists)

**R5.3** Home page is served at `/` with NO extension.
- `/` → serve home page (no redirect)
- `/?foo=bar` → serve home page with query params

**R5.4** Query parameters MUST be preserved.
- `/about?x=1` → `301 /about.htm?x=1`

**R5.5** Case differences redirect to **canonical case** from registry.
- The registry defines the canonical form (e.g., `"checksums"` or `"BH2016"`)
- `/About.htm` → `301 /about.htm` (registry has `"about"`)
- `/CHECKSUMS.HTM` → `301 /checksums.htm` (registry has `"checksums"`)
- `/bh2016` → `301 /BH2016.htm` (if registry defines `"BH2016"`)

**R5.6** `.html` extension MUST redirect to `.htm` (improvement over PHP).
- `/about.html` → `301 /about.htm`
- `/checksums.html` → `301 /checksums.htm`
- Note: This is a Rust improvement - PHP would 404 on these URLs

**R5.7** Already-canonical URLs do NOT redirect.
- `/about.htm` → serve page (200)
- `/audits/` → serve page (200)

**Test cases:**
- [ ] `/about` → `301 /about.htm`
- [ ] `/about.htm` → 200 (no redirect)
- [ ] `/About.htm` → `301 /about.htm`
- [ ] `/about.html` → `301 /about.htm` (new: .html → .htm)
- [ ] `/audits` → `301 /audits/`
- [ ] `/audits/` → 200 (no redirect)
- [ ] `/` → 200 (no redirect)
- [ ] `/about?x=1&y=2` → `301 /about.htm?x=1&y=2`

---

### Requirement 6: 404 Handling

**R6.1** Unknown URLs return HTTP 404 status.

**R6.2** 404 responses render a custom 404 page using standard template.

**Test cases:**
- [ ] `/nonexistent` → 404
- [ ] `/nonexistent.htm` → 404
- [ ] `/.htm` → 404
- [ ] `/foo/.htm` → 404

---

### Requirement 7: Security Headers

**R7.1** All responses include `X-Frame-Options: SAMEORIGIN`.

**R7.2** HTTPS responses to non-accepted hosts include HSTS:
- `Strict-Transport-Security: max-age=31536000; includeSubDomains; preload`
- NOT sent over HTTP (would be ignored anyway)
- NOT sent to localhost/accepted hosts

**Test cases:**
- [ ] Any response has `X-Frame-Options: SAMEORIGIN`
- [ ] HTTPS to defuse.ca has HSTS header
- [ ] HTTP response has NO HSTS header
- [ ] HTTPS to localhost has NO HSTS header

---

### Requirement 8: Metadata Defaults

**R8.1** Default title: `"Defuse Security Research and Development"`
**R8.2** Default meta description: `"Defuse Security. Home of PIE Bin, TRENT, and more..."`
**R8.3** Default meta keywords: `"defuse security, encryption, privacy, programming, code, research"`

---

## Processing Order (based on PHP, with improvements)

```
1. checkHost()
   - If host != master_host AND host not in accepted_hosts:
     → 301 redirect to master_host
     → Use https:// if FORCE_HTTPS or already HTTPS (ANTICIPATION)
     → Include full path and query string

2. checkHTTPS()
   - If FORCE_HTTPS AND not HTTPS AND host not in accepted_hosts:
     → 301 redirect to https://

3. getPageArrayKey()
   - Strip .htm or .html suffix if present (store which was found)
   - Convert path to lowercase for lookup
   - If ".htm" or ".html" was on a name ending in "/" → return 404
   - Look up in PAGE_REGISTRY (case-insensitive)
   - If not found, try appending "/" and look up again
   - Return (canonical_page_name, lookup_key) or None

4. If page not found → 404

5. checkRedirectRequest()
   - If page has redirect target:
     → 301 redirect to target
     → Append .htm if target doesn't end in "/" (ANTICIPATION)
     → Preserve query params

6. ensureCanonicalURL()
   - If directory page and URL doesn't end in "/":
     → 301 redirect with trailing /
   - If non-directory page and URL doesn't end in ".htm":
     → 301 redirect with .htm
   - If URL had .html extension:
     → 301 redirect to .htm
   - If URL case differs from CANONICAL case in registry:
     → 301 redirect to canonical case (e.g., /bh2016 → /BH2016.htm)

7. Serve the page
```

---

## Important Notes

1. **`.html` → `.htm` redirect IS supported** (improvement over PHP) - Unlike PHP which would 404 on `/about.html`, the Rust implementation redirects to `/about.htm`. This is a user-friendly addition.

2. **Configuration is HARDCODED** - Do not use environment variables for MASTER_HOST, ACCEPTED_HOSTS, or FORCE_HTTPS. Match the PHP behavior exactly.

3. **Single redirect is essential** - Test that complex cases like `http://www.defuse.ca/trent` result in exactly ONE 301 redirect.

4. **Case canonicalization uses registry** - URLs redirect to match the canonical case defined in the registry, not just lowercase. If the registry defines `"BH2016"`, then `/bh2016` redirects to `/BH2016.htm`.

---

## Implementation Checklist

- [ ] Hardcode MASTER_HOST, ACCEPTED_HOSTS, FORCE_HTTPS constants
- [ ] Implement host canonicalization with HTTPS anticipation
- [ ] Implement HTTPS enforcement (skip for accepted hosts)
- [ ] Implement page lookup (case-insensitive, strip .htm and .html)
- [ ] Implement alias resolution with .htm anticipation
- [ ] Implement extension canonicalization (.htm / trailing /)
- [ ] Implement .html → .htm redirect (improvement over PHP)
- [ ] Implement case normalization (redirect to canonical case from registry)
- [ ] Implement 404 handling with custom page
- [ ] Implement security headers (X-Frame-Options, HSTS)
- [ ] **Verify single-redirect property** (integration tests)
- [ ] Port all pages from PHP $PAGE_INFO
- [ ] Add integration tests for ALL test cases above

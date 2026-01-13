# Defuse.ca Rust Rewrite - Project Tracker

## Global Infrastructure (applies to all/most pages)

### Completed
- [x] URL routing/canonicalization middleware
  - [x] Redirect `/page` → `/page.htm`
  - [x] Redirect `.html` → `.htm`
  - [x] Case normalization (e.g., `/About.htm` → `/about.htm`)
  - [x] Alias resolution (e.g., `/trent` → `/trustedthirdparty.htm`)
  - [x] Directory pages with trailing slash
  - [x] HTTPS enforcement (with localhost bypass)
  - [x] Host canonicalization (redirect to defuse.ca)
  - [x] ACCEPTED_HOSTS with port matching (e.g., `defuse:10443`)
- [x] Security headers middleware
  - [x] X-Frame-Options: SAMEORIGIN
  - [x] Strict-Transport-Security (HSTS) - only over HTTPS, not for localhost
  - [x] Content-Type: text/html; charset=utf-8 (explicit, not relying on defaults)
  - [x] Cache-Control: no-cache for sensitive pages (via `no_cache` in registry)
- [x] Panic handling (CatchPanicLayer) - panics return 500, don't crash server
- [x] 404 page with custom template
- [x] Base template matching original site layout
  - [x] Full navigation menu with all dropdowns
  - [x] Footer with IP/DNT display, hit counts, CC license
  - [x] Home page uses `contenthome` div (no footer)
  - [x] Regular pages use `content` div (with footer)
- [x] Static files
  - [x] CSS (main.css, mainmenu.css, vimhl.css, print.css)
  - [x] JS (jquery.js, upvote.js)
  - [x] Images at original URLs
- [x] Google site verification meta tag
- [x] Client IP display in footer (X-Forwarded-For aware)
- [x] DNT header detection in footer
- [x] Page registry as single source of truth for metadata

### Requires Database
- [ ] PHPCount hit tracking
  - [ ] Record page hits on every request
  - [ ] Unique hit tracking via IP+page hash (privacy-preserving)
  - [ ] Display hit counts in footer (currently shows 0)
  - [ ] Ignore search bots
  - [ ] Tables: `hits`, `nodupes`
- [ ] Upvote system
  - [ ] `/upvote.php` AJAX endpoint (POST, returns XML)
  - [ ] `Upvote::process_post()` on every request for form fallback
  - [ ] Vote tracking via IP+page hash
  - [ ] Tables: `counts`, `history`
- [x] Upvote images: `upvote.gif`, `upvote-selected.gif`, `downvote.gif`, `downvote-selected.gif`

### Static File Directories (from /storage)
- [ ] `/extras/files` - downloadable files
- [ ] `/extras/files2` - more downloadable files
- [ ] `/extras/mirrors` - mirrored content
- [ ] `/extras/upload/tmp_w` - upload temp directory
- [ ] Figure out URL routing for these (check Apache config)

### Decided to Skip
- [x] ~~Piwik/analytics~~ - Removing entirely
- [x] ~~Entropy feeding to /dev/urandom~~ - Unnecessary in Rust
- [x] ~~Last modified date in footer~~ - Dead code in PHP (computed but never displayed)

---

## TLS/Deployment
- [ ] Document Caddy reverse proxy setup (recommended - auto Let's Encrypt)
- [ ] Alternative: native Rust TLS with `axum-server` + `rustls`

---

## Database Integration (sqlx + MySQL)

### PHPCount Database (`phpcount`)
- [ ] Connect to existing database
- [ ] Implement `AddHit()` - record hit, check uniqueness
- [ ] Implement `GetHits()` - return page hit count
- [ ] Implement `GetTotalHits()` - return site-wide total
- [ ] Search bot detection (skip counting)

### Upvote Database (`upvotes`)
- [ ] Connect to existing database
- [ ] Implement vote processing (up/down/undo)
- [ ] Implement AJAX endpoint returning XML response
- [ ] Render upvote arrows in templates

### Pastebin Database (`cracky_bin`)
- [ ] Connect to existing database
- [ ] Crypto module (CRITICAL - must match PHP exactly)
  - [ ] AES-256-CBC with null-byte padding (NOT PKCS7)
  - [ ] HMAC-SHA256 key derivation
  - [ ] Test vectors from PHP to verify compatibility

### Other Databases
- [ ] TRENT (`cracky_trent`) - trusted RNG drawings
- [ ] Password Policy Hall of Shame (`pphos`)
- [ ] IP geolocation (`ip2location`) - if needed

---

## Feature Implementation Queue

### Dynamic Pages (require logic)
- [ ] Pastebin - encrypted paste storage/retrieval
- [ ] Password generator - secure random generation
- [ ] TRENT - trusted RNG drawings
- [ ] Big number calculator - num-bigint + expression parser
- [ ] HTML sanitizer
- [ ] Online x86 assembler - keep gcc/objdump, fix temp file race
- [x] Checksums page (form handling, multiple hash algorithms)
- [ ] Quantum computer time capsule
- [ ] Password Policy Hall of Shame

### Static Pages (content only)
- [x] Home page
- [x] About page
- [ ] Contact page
- [ ] Services, Projects, Research index pages
- [ ] All audit pages (audits/encfs, audits/ecryptfs, etc.)
- [ ] All research/article pages
- [ ] All misc pages
- [ ] See URLParse.php $PAGE_INFO for full list (~100+ pages)

---

## Testing
- [ ] Create PHP test vectors for crypto compatibility
- [ ] URL routing tests - all old URLs must work
- [ ] Database query verification against production data

---

## Key Files Reference
- `defuse.ca/src/index.php` - Master template
- `defuse.ca/src/libs/URLParse.php` - URL routing (1,077 lines)
- `defuse.ca/src/bin/pastebin.php` - Crypto to match
- `defuse.ca/src/libs/PasswordGenerator.php` - Constant-time RNG
- `defuse.ca/src/libs/phpcount.php` - Hit tracking
- `defuse.ca/src/libs/Upvote.php` - Vote system

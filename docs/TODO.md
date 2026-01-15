# Defuse.ca Rust Rewrite - Project Tracker

## Implementation Roadmap

### Phase 1: Global Infrastructure - COMPLETE
- [x] URL routing/canonicalization
- [x] Security headers
- [x] Hit counting (PHPCount)
- [x] Upvote system
- [x] Base template with navigation
- [x] Static files (CSS, JS, images)
- [x] robots.txt

### Phase 1.5: Codebase Audit Follow-up (NEXT)
- [x] Review `docs/CODEBASE_AUDIT.md` and decide what to address

### Phase 2: Per-Page Features (IN PROGRESS)
Enable features needed by many static pages before bulk migration.

- [x] **Syntax Highlighting** (vim-based, matching PHP exactly)
  - `ctx.hl_string(text, filetype, show_lines)` in templates
  - `ctx.hl_file(path, show_lines)` for source files
  - Caching support via /storage/vimhl
  - Tested with blind-birthday-attack page

- [ ] **Bibliography System**
  - Rust struct to hold references
  - Askama macros/filters for `cite()` and bibliography rendering
  - Test with a research page (e.g., FLUSH+RELOAD)

### Phase 3: Static Content Migration
Bulk migration of ~100 static pages. Build tooling to ensure accuracy.

- [ ] **Verification tooling**
  - Script to compare Rust output vs live site HTML
  - Diff tool to catch transcription errors
  - Automated check for broken links, missing images

- [ ] **Migration strategy**
  - Extract page list from URLParse.php $PAGE_INFO
  - For each page: add to registry, create template, verify output
  - Group by section (audits/, research/, misc/)

- [ ] **Static page groups**
  - [ ] Index pages (services, projects, research)
  - [ ] Audit reports (6 pages)
  - [ ] Research articles (~30 pages)
  - [ ] Misc pages (~40 pages)
  - [ ] Mirrors (pocorgtfo, truecrypt hashes)

### Phase 4: Dynamic Features
Complex pages requiring significant logic.

- [ ] **Pastebin** (CRITICAL - crypto compatibility)
  - AES-256-CBC with null-byte padding
  - HMAC-SHA256 key derivation matching PHP exactly
  - Test vectors from production data

- [ ] **Password Generator**
  - Constant-time secure random generation
  - Multiple output formats
  - No-cache headers (already in registry)

- [ ] **TRENT** - Trusted random number drawings
  - Database integration
  - Drawing verification

- [ ] **Big Number Calculator**
  - `num-bigint` + expression parser
  - Replace Ruby shell-out

- [ ] **Online x86 Assembler**
  - Keep gcc/objdump but fix temp file race
  - Use `tempfile` crate

- [ ] **HTML Sanitizer**
  - Pure Rust implementation

- [ ] **Quantum Computer Time Capsule**
  - reCAPTCHA integration

### Phase 5: Deployment & Testing
- [ ] Caddy reverse proxy setup (or native TLS)
- [ ] Full URL routing test suite
- [ ] Crypto compatibility verification
- [ ] Load testing
- [ ] Production deployment

---

## Detailed Status

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
- [x] PHPCount hit tracking
  - [x] Record page hits on every request
  - [x] Unique hit tracking via IP+page hash (privacy-preserving)
  - [x] Display hit counts in footer
  - [x] Ignore search bots
  - [x] Tables: `hits`, `nodupes`
  - [x] Local dev: docker-compose.yml with MariaDB
- [x] Upvote system
  - [x] `/upvote` AJAX endpoint (POST, returns XML)
  - [x] Upvote POST fallback middleware (for non-JS users)
  - [x] Vote tracking via IP+page hash (privacy-preserving)
  - [x] 24-hour rate limiting (allows re-vote after cooldown)
  - [x] Tables: `counts`, `history`
  - [x] Per-page upvote config via registry (UpvoteConfig)
  - [x] Vote counts fetched in middleware, available in templates
  - [x] Home page top 8 pages list
- [x] Upvote images: `upvote.gif`, `upvote-selected.gif`, `downvote.gif`, `downvote-selected.gif`
- [x] robots.txt

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

### PHPCount Database (`phpcount`) - COMPLETE
- [x] Connect to existing database
- [x] Implement `AddHit()` - record hit, check uniqueness
- [x] Implement `GetHits()` - return page hit count
- [x] Implement `GetTotalHits()` - return site-wide total
- [x] Search bot detection (skip counting)
- [x] `php_page_id` field in PageInfo for backward compatibility

### Upvote Database (`upvotes`) - COMPLETE
- [x] Connect to existing database
- [x] Implement vote processing (up/down/undo)
- [x] Implement AJAX endpoint returning XML response
- [x] Render upvote arrows in templates
- [x] Non-JS fallback via POST middleware

### Pastebin Database (`cracky_bin`)
- [ ] Connect to existing database
- [ ] Crypto module (CRITICAL - must match PHP exactly)
  - [ ] AES-256-CBC with null-byte padding (NOT PKCS7)
  - [ ] HMAC-SHA256 key derivation
  - [ ] Test vectors from PHP to verify compatibility

### Other Databases
- [ ] TRENT (`cracky_trent`) - trusted RNG drawings
- [ ] IP geolocation (`ip2location`) - if needed
- [x] ~~Password Policy Hall of Shame (`pphos`)~~ - Page deprecated, just static content now

---

## Per-Page Features (used on multiple pages)

### Syntax Highlighting (VimHighlight.php)
- [ ] PHP: Uses Vim shell-out to syntax-highlight code
- [ ] Rust: Use `syntect` crate instead (per plan, no shell-out)
- [ ] Functions to implement:
  - `printHlString($text, $ft, $numbers)` - highlight text inline
  - `printSourceFile($path, $numbers)` - highlight a file
- [ ] Used on many code/research pages (e.g., FLUSH+RELOAD, crypto pages)
- [ ] Color scheme: `dw_cyan` (may need custom theme)

### Bibliography System (Bibliography.php)
- [ ] For research articles with academic citations
- [ ] Functions to implement:
  - `addReference($key, $title, $authors, $date, $url)` - add citation
  - `cite($key)` - inline citation link [1]
  - `printBibliography()` - render references section
- [ ] Used on research pages (FLUSH+RELOAD, side channels, etc.)

### Page-Specific Script Includes
- [ ] reCAPTCHA for quantum-computer-time-capsule page
- [ ] Other page-specific JS/CSS if needed

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
- [x] Contact page
- [ ] Services, Projects, Research index pages
- [ ] All audit pages (audits/encfs, audits/ecryptfs, etc.)
- [ ] All research/article pages
- [ ] All misc pages
- [ ] See URLParse.php $PAGE_INFO for full list (~100+ pages)

---

## Known Issues / Bugs

- [ ] **File paths are relative to CWD, not binary location**
  - `hl_file()` and static file serving use relative paths
  - Server must be started from project directory or paths break
  - Should resolve paths relative to the binary location instead
  - Affects: vim highlighting, static files, source files

- [x] **Vim syntax highlighting: dw_cyan colorscheme was never on prod server**
  - **Problem**: PHP code specified `dw_cyan` colorscheme but it was never installed on
    the production server. Vim silently fell back to `default` colorscheme.
  - **Symptom**: Local dev with dw_cyan installed produced different HTML than production.
    Specifically, dw_cyan defines `String` with only `guifg` (no `ctermfg`), so terminal
    vim's TOhtml skipped outputting `<span class="String">` entirely for string content.
  - **Discovery**: Strings showed as `<span class="Constant">` in production but had no
    span wrapper locally. The default colorscheme links `String` → `Constant` and has
    proper `ctermfg` definitions, so TOhtml outputs spans correctly.
  - **Solution**: Changed Rust implementation to use `default` colorscheme instead of
    `dw_cyan` to match actual production behavior.
  - **Remaining differences**: Minor vim 9.1 vs 8.2 syntax file differences (e.g., block
    params highlighted as `Identifier` in old vim, Ruby symbols split differently)

---

## Testing
- [ ] Create PHP test vectors for crypto compatibility
- [ ] URL routing tests - all old URLs must work
- [ ] Database query verification against production data
- [ ] Integration tests for upvote system (AJAX endpoint + fallback middleware)

---

## Key Files Reference
- `defuse.ca/src/index.php` - Master template
- `defuse.ca/src/libs/URLParse.php` - URL routing (1,077 lines)
- `defuse.ca/src/bin/pastebin.php` - Crypto to match
- `defuse.ca/src/libs/PasswordGenerator.php` - Constant-time RNG
- `defuse.ca/src/libs/phpcount.php` - Hit tracking
- `defuse.ca/src/libs/Upvote.php` - Vote system

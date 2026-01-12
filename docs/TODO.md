# Defuse.ca Rust Rewrite - Project Tracker

## Completed
- [x] Design decisions documented (see DESIGN_DECISIONS.md)
- [x] Basic Axum project structure
- [x] Askama templating setup
- [x] Base template matching original site layout EXACTLY
  - Full navigation menu with all dropdowns
  - Footer with IP/DNT display, hit counts, CC license
  - Home page uses contenthome div (no footer)
  - Regular pages use content div (with footer)
- [x] Copied exact CSS (main.css, mainmenu.css, vimhl.css, print.css)
- [x] Copied all images and JS at original URLs
- [x] Checksums page (form handling, multiple hash algorithms)
- [x] Home page (static, with original content)
- [x] About page (static, with original content)
- [x] Context system for passing request info to templates

## In Progress
- [ ] URL canonicalization middleware
  - Redirect `/page` → `/page.htm` (canonical form)
  - Redirect `/page.htm` → `/page.htm` (no change)
  - Handle directory pages (e.g., `/audits/` stays as-is)
  - HTTPS enforcement (with localhost bypass via FORCE_HTTPS env var)
  - Host canonicalization (redirect to defuse.ca)
  - See `defuse.ca/src/libs/URLParse.php` for full logic

## Next Steps (High Priority)

### TLS/HTTPS Support
- [ ] Add TLS support for production deployment
- [ ] Integrate with Let's Encrypt (certbot or acme-client)
- [ ] HSTS header support (already in original PHP)
- [ ] Document nginx reverse proxy setup as alternative

### Database Integration (sqlx + MySQL)
- [ ] PHPCount hit tracking
  - Page hits counter in footer (currently shows 0)
  - Unique hits counter in footer (currently shows 0)
  - Anonymous tracking via IP+salt hashing
- [ ] Upvote system
  - "Top 8 Pages" list on home page
  - Vote counters on pages
- [ ] Pastebin storage

### Request Context
- [ ] Extract real client IP from connection (not just headers)
  - Currently using X-Forwarded-For/X-Real-IP headers
  - Need to extract from socket addr when not behind proxy
- [ ] Pass connection info via Axum ConnectInfo extractor

## Feature Implementation Queue
- [ ] Crypto module for pastebin (CRITICAL - must match PHP exactly)
  - AES-256-CBC with null-byte padding
  - HMAC-SHA256 key derivation
  - See DESIGN_DECISIONS.md for exact algorithm
- [ ] Pastebin page
- [ ] Password generator page
- [ ] TRENT (trusted RNG) page
- [ ] Big number calculator (num-bigint + expression parser)
- [ ] HTML sanitizer page
- [ ] Online x86 assembler (keep gcc/objdump, fix temp file race)

## Port Remaining Static Pages
Pages to port from PHP site (see URLParse.php $PAGE_INFO array):
- [ ] contact.htm
- [ ] services.htm, projects.htm, research.htm, software.htm
- [ ] All audit pages (audits/encfs, audits/ecryptfs, etc.)
- [ ] All research pages
- [ ] All misc pages
- [ ] 404 page

## Per-Feature Decisions (decide when implementing)
- [ ] Assembler - keep gcc/objdump (fix temp file race with `tempfile` crate)
- [ ] Syntax highlighting - syntect integration
- [ ] Big number calculator - use num-bigint (replacing Ruby)

## Testing
- [ ] Create PHP test vectors for crypto compatibility
- [ ] URL routing tests (ensure all old URLs work)
- [ ] Database query verification

## Security Headers (from original PHP)
- [ ] X-Frame-Options: SAMEORIGIN
- [ ] Strict-Transport-Security (HSTS)
- [ ] Content-Type: text/html; charset=utf-8

## Key Files Reference
- `defuse.ca/src/index.php` - Master template
- `defuse.ca/src/libs/URLParse.php` - URL routing (1,077 lines)
- `defuse.ca/src/bin/pastebin.php` - Crypto to match
- `defuse.ca/src/libs/PasswordGenerator.php` - Constant-time RNG
- `defuse.ca/src/libs/phpcount.php` - Hit tracking
- `defuse.ca/src/libs/Upvote.php` - Vote system

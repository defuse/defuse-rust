# Defuse.ca Rust Rewrite - Design Decisions

## Goals
- Drop-in replacement for PHP site (same DB tables, same URLs, same functionality)
- Code clarity over framework magic
- Minimal maintenance burden
- Easy local development (clone DB, run locally)
- Security paramount

## Confirmed Decisions

### Web Framework: Axum
- Backed by Tokio team (AWS), built on stable tower/hyper ecosystem
- Minimal magic, explicit routing
- Strong long-term maintenance likelihood

### Templating: Askama
- Compile-time template checking (errors at build, not runtime)
- Real `.html` files with syntax highlighting
- Jinja2-style syntax: `{% for %}`, `{% if %}`, `{{ variable }}`
- Each page has logic in `src/pages/foo.rs` and template in `templates/pages/foo.html`

### Database: sqlx
- Raw SQL with compile-time checking
- Parameterized queries (secure against SQL injection)
- Works with existing tables, no schema management

### Syntax Highlighting: syntect
- Pure Rust, no shell-out
- Different output than Vim but good quality
- Can revisit Vim-identical output later if wanted

### Big Number Calculator: Keep Ruby (for now)
- Existing code is well-secured
- Can revisit later

### PDF Cleaner: Remove
- Not used, has command injection vulnerability

## Per-Feature Decisions (TBD during implementation)
- Assembler (gcc/objdump): decide when implementing
- Other shell-outs: decide when implementing

## Critical: URL Routing

The PHP site has important URL canonicalization in `URLParse.php`:

1. **Canonical form**: `/pagename` (no extension)
2. **Redirects**:
   - `/pagename.htm` → redirect to `/pagename`
   - `/pagename/` → may be directory index or redirect
   - `http://` → redirect to `https://`
   - Non-canonical hosts → redirect to `defuse.ca`

3. **Aliases**: Some pages have multiple names (e.g., `/trent` → `/trustedthirdparty`)

4. **Local development**: Must work on `localhost` without HTTPS redirects

### Implementation approach:
- Environment variable or config to control HTTPS enforcement
- Middleware layer for URL canonicalization
- Route aliases defined alongside pages

## Critical: Crypto Compatibility

Pastebin encryption MUST match PHP exactly to decrypt existing data:

```
Key derivation (from URL key):
  database_id    = HMAC-SHA256(key="database_identity", data=urlKey) -> hex
  encryption_key = HMAC-SHA256(key="encryption_key", data=urlKey)    -> raw bytes

Encryption: AES-256-CBC
  - IV: 16 random bytes
  - Padding: null bytes (NOT PKCS7) - this is mcrypt's behavior
  - Output: base64(IV || ciphertext)
```

## Architecture

```
defuse-rust/
  src/
    main.rs              # Entry point, router setup
    pages/
      mod.rs             # Page module exports
      home.rs            # Logic for each page
      checksums.rs
      ...
    libs/                # Shared libraries (crypto, db, etc.)
  templates/
    base.html            # Main layout (header, nav, footer)
    pages/
      home.html          # Template for each page
      checksums.html
      ...
  static/                # CSS, JS, images (served directly)
```

## Local Development

1. Clone production database to local MySQL
2. Create `.env` file:
   ```
   DATABASE_URL=mysql://user:pass@localhost/dbname
   LISTEN_ADDR=127.0.0.1:3000
   FORCE_HTTPS=false
   ```
3. Run `cargo run`

## Key PHP Files Reference

- `defuse.ca/src/index.php` - Master template (453 lines)
- `defuse.ca/src/libs/URLParse.php` - URL routing (1,077 lines)
- `defuse.ca/src/bin/pastebin.php` - Crypto to match exactly
- `defuse.ca/src/libs/PasswordGenerator.php` - Constant-time RNG
- `defuse.ca/src/libs/phpcount.php` - Hit tracking
- `defuse.ca/src/libs/Upvote.php` - Vote system
- `defuse.ca/src/pages/services/checksums.php` - Hash algorithms
- `defuse.ca/src/main.css` - Main stylesheet
- `defuse.ca/src/mainmenu.css` - Menu stylesheet (GRC script-free menu)

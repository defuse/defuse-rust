# Deferred Bug Review Issues

## Timeouts & DoS

- [ ] **Vim syntax highlighting has no timeout** — `vim_highlight.rs:315`. Uses blocking `std::process::Command` with no timeout. A hanging vim process blocks a thread forever. Unlike gcc/objdump/ruby, vim has no `timeout()` or `kill_on_drop(true)`. (Showstopper)
- [ ] **No timeout on Google reCAPTCHA API requests** — `recaptcha.rs:50-61`. `reqwest::Client::new()` has no connect/request timeout. A Google outage hangs blocking threads indefinitely. Also creates a new client per call instead of sharing one. (Showstopper)
- [ ] **No timeouts on database queries** — All `MySqlPool::connect()` calls use default settings with no query timeout or acquire timeout. A slow/unresponsive MySQL hangs every page (all pages do hit counting). (Showstopper)
- [ ] **TRENT `select_random_lines` O(N^2) CPU DoS** — `trent.rs:537-551`. Rejection sampling without replacement degrades to ~500K iterations for 1000 lines. Should use Fisher-Yates shuffle. (Critical)
- [ ] **POST body buffered up to 100 MB before checking method support** — `registered_page_handler.rs:110-146`. Pages that don't accept POST still read the full body. No global body limit exists. (Medium)

## Panics Reachable from User Requests

- [ ] **`assert!()` in big number calculator on crafted Ruby output** — `big_number_calculator/mod.rs:112-116`. Should return an error response instead of panicking. (Showstopper)
- [ ] **`panic!()` in vim cache directory check** — `vim_highlight.rs:133`. If cache dir is deleted at runtime, every syntax-highlighted page panics. Should fall back gracefully. (Critical)
- [ ] **`.expect()` on DB queries for homepage and all-pages** — `home.rs:20-25`, `all_pages.rs:19-24`. Transient DB issues panic the two most-visited pages. Should use `.unwrap_or_else()` with fallback. (High)
- [ ] **`.expect()` on vote processing in middleware** — `upvote_post.rs:119`. DB error during upvote panics in middleware that runs on every POST. Should return error response. (High)
- [ ] **`assert!()` on DB result in TRENT GET handler** — `trent.rs:226`. `assert!(drawing_num == drawing.drawingnum)` panics on every view if invariant breaks. Should log and return error. (Medium)
- [ ] **`.expect()` on missing `RECAPTCHA_SECRET_KEY`** — `recaptcha.rs:41`. Not validated at startup unlike other env vars. First time capsule submission panics if unset. (Medium)

## Security

- [ ] **No CSRF protection on paste creation** — `pastebin_add.rs`. `POST /bin/add.php` has no `csrf::check_origin()`. Allows cross-site paste creation. Matches PHP behavior but should be fixed. (Critical)
- [ ] **gcc/objdump error messages may leak internal paths** — `x86_assembler/executor.rs:75`. `InternalError` variant passes system errors to user. Should return generic message. (Critical)

## TRENT System

- [ ] **Silent temp file loss between confirmation and completion** — `trent.rs:394-409`. If `/tmp` is cleaned between steps, file hashes silently disappear from the printout for files with `randlines == 0`. Should error instead of silently omitting. (High)
- [ ] **`from_utf8_lossy` corrupts legacy Latin-1 drawing data** — `trent.rs:160-163`. Old PHP drawings with accented characters display as replacement characters. (Medium)
- [ ] **Misleading "ALLOW REPEAT LINES: No" in printout** — `trent.rs:386-388`. Shows even when no files are uploaded. Confusing to verifiers. (Low)
- [ ] **`last_insert_id()` truncated to i32** — `trent.rs:187`. Wraps after 2B drawings. (Low)
- [ ] **No temp file cleanup for abandoned drawings** — `trent.rs:464-472`. Up to 30 MB per abandoned drawing accumulates in `/tmp`. (Low)

## Race Conditions (all match PHP behavior)

- [ ] **Upvote `process_vote` TOCTOU** — `upvotes.rs:146-201`. Concurrent votes can double-increment counts. Should wrap in transaction. (Low)
- [ ] **`ensure_page` and `set_user_action` race on INSERT** — `upvotes.rs:316-330,644-678`. SELECT-then-INSERT can create duplicates. Should use `INSERT ... ON DUPLICATE KEY UPDATE`. (Low)
- [ ] **PHPCount duplicate row creation** — `phpcount.rs:224-258`. Concurrent first-visits create duplicate rows, inflating site-wide totals. (Low)
- [ ] **Pastebin key generation race** — `pastebin.rs:150-188`. Check-then-insert without transaction. Astronomically unlikely with 22-char random keys. (Low)

## Minor

- [ ] **404 page `url_prefix` hardcoded** — `registered_page_handler.rs:285`. Uses `https://defuse.ca` instead of deriving from request Host. Affects dev. (Low)
- [ ] **`resolve_alias` can infinite-loop on circular aliases** — `registry/mod.rs:355-362`. No cycle detection or depth limit. (Low)

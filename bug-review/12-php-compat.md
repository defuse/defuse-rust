# Bug Review #12: PHP Compatibility

Scope: Checking that the Rust rewrite matches the original PHP behavior for
areas where compatibility is required (URL structure, crypto, database keys,
form fields, response formats).

---

## CRITICAL: Hit Counter ID Mismatch for Pastebin Page

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/registry/pages.rs` line 260

The pastebin page's `legacy_hit_count_id` does not match the PHP version.

- **PHP:** The pastebin page at slug `"pastebin"` has `P_FILE => "services/pastebin.html"`,
  and the hit counter key is constructed as `ROOT_FOLDER . P_FILE` = `"pages/services/pastebin.html"`.
- **Rust:** `legacy_hit_count_id: "pages/services/pastebin.php"`

The extension is `.php` in Rust but `.html` in PHP. This means the Rust version
will create a **new** hit counter entry in the database instead of continuing
the existing `pages/services/pastebin.html` counter. All historical hit count
data for the pastebin page will be lost / invisible.

**Fix:** Change line 260 to:
```rust
legacy_hit_count_id: "pages/services/pastebin.html",
```

---

## MODERATE: Expired Paste Deletion Uses `<` Instead of `<=`

**Files:**
- PHP: `/home/taylor/defuse-rewrite/defuse.ca/src/bin/pastebin.php` line 138
- Rust: `/home/taylor/defuse-rewrite/defuse-rust/src/libs/pastebin.rs` line 238

PHP uses:
```sql
DELETE FROM pastes WHERE time <= :time
```

Rust uses:
```sql
DELETE FROM pastes WHERE time < ?
```

The `<=` vs `<` difference means that a paste expiring at exactly the current
second will be deleted by PHP but retained by Rust until the next second. This
is a minor behavioral difference but could cause intermittent issues in test
suites that check paste expiration timing precisely.

**Fix:** Change line 238 of `pastebin.rs` to use `<=`:
```rust
sqlx::query("DELETE FROM pastes WHERE time <= ?")
```

---

## MODERATE: Three PHP Pages Missing From Rust Registry

The following pages exist in the PHP `$PAGE_INFO` array but are absent from the
Rust `PAGE_REGISTRY`:

### 1. `ip` page

**PHP:** `"ip" => P_FILE => "services/ip.php"` with title "Your IP Address"

The Rust version has `/ip.php` as a special endpoint returning plain-text IP,
but the PHP version also had `/ip.htm` as a full page (with header, nav, footer)
that shows both HTTPS and HTTP IP addresses in styled HTML with an iframe. The
`/ip.htm` URL now returns 404 in the Rust version.

**Impact:** Low. This is a minor utility page. However, any external links to
`https://defuse.ca/ip.htm` will break.

### 2. `peerreview` page

**PHP:** `"peerreview" => P_FILE => "services/peerreview.html"` with title
"Peer Review and Security Testing Service - Defuse Security"

This page is completely missing from the Rust version. The URL `/peerreview.htm`
returns 404.

**Impact:** Low. This appears to be an old service offering. External links will
break.

### 3. `passwordblocks` page

**PHP:** `"passwordblocks" => P_FILE => "services/passwordblocks.php"` with
title "Password Building Blocks - The HUMAN Password Generator"

This is a client-side interactive password generator using JavaScript. The page
is missing from the Rust registry even though the JS file
(`static/js/passwordblocks.js`) exists. The URL `/passwordblocks.htm` returns
404.

**Impact:** Low-moderate. This was a functional service page. External links will
break and anyone bookmarking it will get a 404.

**Recommendation:** For all three, either implement the pages or add them as
aliases redirecting to a related page (e.g., `passwordblocks` -> `passgen`,
`peerreview` -> `software-security-auditing`).

---

## LOW: Pastebin Null-Byte Stripping Difference

**Files:**
- PHP: `/home/taylor/defuse-rewrite/defuse.ca/src/bin/pastebin.php` line 107
- Rust: `/home/taylor/defuse-rewrite/defuse-rust/src/libs/pastebin_crypto.rs` line 170

PHP strips ALL null bytes from the decrypted text:
```php
str_replace("\0", "", mcrypt_decrypt(...))
```

Rust strips only TRAILING null bytes:
```rust
let end = decrypted.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
decrypted.truncate(end);
```

If a paste's plaintext somehow contained an embedded null byte (e.g., through
the mcrypt padding landing on a block boundary where interior bytes were already
null), PHP would remove it while Rust would preserve it.

**Impact:** Very low for normal text pastes. Text input should never contain null
bytes. The mcrypt zero-padding only adds null bytes at the end. This would only
matter for binary data, which the pastebin does not officially support.

**No fix needed** unless binary paste support is added in the future.

---

## LOW: Upvote XML Response Content-Type Difference

**Files:**
- PHP: `/home/taylor/defuse-rewrite/defuse.ca/src/libs/Upvote.php` line 466
- Rust: `/home/taylor/defuse-rewrite/defuse-rust/src/upvote.rs` line 82

PHP sends: `Content-Type: text/xml`
Rust sends: `Content-Type: text/xml; charset=utf-8`

The charset suffix is an addition but should not break any XML parser since
UTF-8 is the default for XML anyway. The JavaScript client (`upvote.js`) parses
the XML using `responseXML` on the `XMLHttpRequest` object, which handles both
variants.

**No fix needed.**

---

## INFO: HTML Escaping Apostrophe Difference (Known)

**Files:**
- Rust: `/home/taylor/defuse-rewrite/defuse-rust/src/libs/util.rs` line 58

The Rust `html_escape` function encodes apostrophes as `&#x27;` while PHP's
`htmlentities(..., ENT_QUOTES)` encodes them as `&#039;`. This is already
documented in the project memory.

**Impact:** Both are valid HTML entity encodings for the single quote character
and render identically in all browsers. There is no functional or visual
difference to users.

The Askama templating engine's built-in escaping also uses `&#x27;` for
apostrophes. Test assertions that compare against rendered HTML should use the
`&#x27;` form (or Rust's `&#039;` if the custom `html_escape` function is used -
though the current code consistently uses `&#x27;`).

**No fix needed** unless byte-for-byte HTML output matching is required for some
specific integration.

---

## VERIFIED: No Issues Found

### Pastebin Crypto (HMAC Key Derivation, Encryption, Decryption)

The Rust implementation in `/home/taylor/defuse-rewrite/defuse-rust/src/libs/pastebin_crypto.rs`
correctly matches the PHP implementation:

- **HMAC parameter order:** PHP's `hash_hmac("SHA256", "database_identity", $urlKey, false)`
  uses `$urlKey` as the key and `"database_identity"` as the data. The Rust code
  correctly does `Hmac::<Sha256>::new_from_slice(url_key.as_bytes())` with
  `mac.update(b"database_identity")`. Same for `"encryption_key"`.
- **Database ID:** Returns lowercase hex of HMAC-SHA256 -- matches PHP's `false`
  (hex output) parameter.
- **Encryption key:** Returns raw 32 bytes of HMAC-SHA256 -- matches PHP's `true`
  (binary output) parameter.
- **AES-256-CBC:** Uses MCRYPT_RIJNDAEL_128 (AES) in CBC mode with zero-byte
  padding, matching PHP's mcrypt behavior.
- **IV handling:** Random 16-byte IV prepended to ciphertext, base64 encoded --
  matches PHP format.

### Password Generator (Charset and Algorithm)

The Rust implementation in `/home/taylor/defuse-rewrite/defuse-rust/src/libs/passgen.rs`
correctly matches the PHP implementation:

- **ASCII charset:** `!"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\]^_` + `` ` `` + `abcdefghijklmnopqrstuvwxyz{|}~`
  (94 characters, codes 33-126) -- matches PHP.
- **Alphanumeric charset:** `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789`
  (62 characters) -- matches PHP.
- **Hex charset:** `0123456789ABCDEF` (uppercase) -- matches PHP.
- **Algorithm:** Rejection sampling with minimal bit mask -- matches PHP algorithm.
- **Pastebin URL key generation:** Uses `generate_alphanumeric_password(22)` for
  standard and `(8)` for short URLs -- matches PHP's
  `PasswordGenerator::getAlphaNumericPassword($short ? 8 : 22)`.

### Form Field Names

All interactive pages have matching form field names between PHP and Rust:

- **Pastebin submit** (`/bin/add.php`): `paste`, `jscrypt`, `lifetime`,
  `shorturl`, `submitpaste` -- all match.
- **Pastebin view** (`/b/{key}`): `raw`, `delete` query params -- match.
- **Checksums:** `data`, `normalize`, `filetohash`, `hashfile` -- all match.
- **HTML Sanitize:** `data`, `sanitize`, `tw`, `br` -- all match.
- **TRENT:** `drawingnum`, `makedrawingnumber`, `prereview`, `drawingnumber`,
  `passcode`, `name`, `description`, `file1`-`file3`, `randlines1`-`randlines3`,
  `chosentwice`, `lowval`, `highval`, `numgen`, `create`, `confirmed` -- all match.
- **Time Capsule:** `message`, `algorithm`, `present_public_key`,
  `future_public_key`, `ciphertext`, `g-recaptcha-response` -- all match.
- **Upvote:** `upvotes_id`, `upvotes_direction` -- match.

### URL Structure and Aliases

All page slugs and redirect aliases in the Rust registry match the PHP
`$PAGE_INFO` array (with the exception of the three missing pages noted above).
The `.htm` canonical extension behavior and case-insensitive slug matching are
correctly implemented.

### Upvote XML Response Format

The XML structure matches PHP exactly (same element names, same order, same
values for arrow states).

### Pastebin View Page

The HTML structure, CSS, JavaScript (including `js_string_escape`), and form
elements all match the PHP version.

---

## Summary

| Issue | Severity | Status |
|-------|----------|--------|
| Pastebin hit counter ID `.php` vs `.html` | CRITICAL | Needs fix |
| Paste cleanup `<` vs `<=` | MODERATE | Needs fix |
| Three missing pages (ip, peerreview, passwordblocks) | MODERATE | Needs decision |
| Null-byte stripping (all vs trailing only) | LOW | Acceptable |
| Upvote XML charset suffix | LOW | Acceptable |
| Apostrophe escaping `&#x27;` vs `&#039;` | INFO | Known/Acceptable |

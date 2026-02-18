# Pastebin System Review

## 1. Delete-then-view allows fetching a paste after requesting deletion

**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/pages/services/pastebin_view.rs:54-63
**Description**: When the `?delete=SECRET` parameter is provided, the handler
deletes the paste and then immediately tries to fetch it with `get_paste`. Since
the paste was just deleted, `get_paste` returns `NotFound` and the user sees the
"paste not found" page. This matches the PHP behavior (PHP also deletes then
fetches, getting `false`). However, both the delete result and fetch result are
unlinked: the user gets no confirmation that the delete succeeded vs. the paste
simply not existing. This is consistent with PHP, so no change needed, but worth
noting.

## 2. Expiration delete uses strict less-than while PHP uses less-than-or-equal

**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/libs/pastebin.rs:238
**Description**: The Rust `delete_expired` method uses `DELETE FROM pastes WHERE
time < ?` (strict less-than), while the PHP version uses `DELETE FROM pastes
WHERE time <= :time` (less-than-or-equal). This means a paste expiring at
exactly `time == now` survives one extra second in Rust before being cleaned up
by the next `delete_expired` call. In practice, this is mitigated by line 216 in
`get_paste` which checks `if timeleft <= 0` and returns `NotFound` for pastes at
exactly the expiration time. The net effect is that the paste becomes invisible
to users at the right time, but the database row lingers for up to one extra
second before physical deletion. No user-visible impact, but should use `<=` for
consistency with PHP.

## 3. Time-left display format differs from PHP

**Severity**: Low
**File**: /home/taylor/defuse-rewrite/defuse-rust/src/libs/pastebin.rs:265-296
**Description**: The PHP view page always shows the format:
`"This post will be deleted in X days, Y hours, and Z minutes."` with all three
components present (even when zero). The Rust `format_timeleft` function uses a
tiered format that drops smaller units: when days >= 1, only days and hours are
shown (minutes omitted); when hours >= 1, only hours and minutes are shown; etc.
Example: a paste with 5 days, 3 hours, 42 minutes remaining shows as
`"5 days, 3 hours"` in Rust vs. `"5 days, 3 hours, and 42 minutes."` in PHP.
The PHP format also includes "and" between the last two components and a trailing
period. This is a visible behavioral difference for users but does not affect
functionality.

## No Critical or High Severity Bugs Found

The core crypto implementation is correct:

- **HMAC key derivation**: The `get_database_id` and `get_encryption_key`
  functions correctly use URL key as the HMAC key and fixed strings as the
  message, matching PHP's `hash_hmac("SHA256", $message, $urlKey)` parameter
  order.

- **AES-256-CBC**: The manual CBC implementation (XOR with previous block, then
  encrypt) is correct. The decryption (decrypt block, then XOR with previous
  ciphertext block) is the correct inverse.

- **Zero-byte padding**: Encryption pads with zeros to block boundary (matching
  mcrypt). Decryption strips trailing null bytes. The PHP version strips ALL null
  bytes (`str_replace("\0", "", ...)`), while Rust strips only trailing ones.
  For text-only content this is equivalent, and the Rust behavior is arguably
  more correct (intentionally changed in commit 73d182b).

- **Base64 encoding**: Standard base64 is used for IV||ciphertext, matching
  PHP's `base64_encode`/`base64_decode`.

- **js_string_escape**: Correctly escapes non-alphanumeric characters as `\xHH`
  per byte, matching PHP's byte-level iteration with `strlen`/`ord`.

- **Line ending normalization**: CRLF -> LF then CR -> LF, matching PHP's
  `str_replace` chain.

- **HTML escaping**: Uses `html_escape` for both the display area and textarea
  content, preventing XSS. The escaping covers `& < > " '`.

- **Delete secret**: SHA-256 hash comparison matches PHP. The hash is compared
  against a hardcoded constant.

- **jscrypt handling**: Server-side encryption is correctly applied on top of
  client-side ciphertext (matching PHP). The view page correctly shows a
  password prompt for jscrypt pastes and returns an error for raw access.

- **Form fields**: All form fields from PHP are present: paste, jscrypt
  (hidden), lifetime (select), shorturl (checkbox), and the client-side
  encryption password fields with Encrypt & Post button.

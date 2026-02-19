# Final Review 07: Pages, Routing, and Miscellaneous Areas

**Reviewer scope:** Page registry/routing, contact form, research/project/audit pages,
BREACH mitigation, bibliography, upvote system, web server scan.

## Result: No showstopper issues found.

All reviewed areas look clean for production deployment. Below is a summary of each
area and why it passes.

### 1. Page Registry and Routing

**Files:** `src/registry/mod.rs`, `src/registry/pages.rs`, `src/registered_page_handler.rs`

The page registry is a compile-time-defined static HashMap of page slugs to PageInfo
structs. Handlers are referenced as `&'static dyn PageHandler` -- there is no way for
user input to influence which handler is invoked. The `resolve_path()` function does
case-insensitive lookup with proper alias resolution and returns only Canonical, Redirect,
or NotFound. Double slashes, path traversal (`/../`), and other malformed paths all
correctly return NotFound. Extensive unit tests confirm these invariants.

The `panic-test` and `test-directory` pages exist in the registry but are harmless:
panic-test is caught by CatchPanicLayer (returns 500), and test-directory is a static
test page. Neither leaks sensitive information. These could be removed before deployment
but are not a security concern.

**Verdict:** Clean.

### 2. Contact Form

**File:** `src/pages/contact.rs`

The contact page is a `simple_page!` macro invocation -- it only serves a static HTML
template on GET. There is no POST handler, no email sending, no form processing of any
kind. No email injection or open relay risk.

**Verdict:** Clean.

### 3. Blog/Research/Project/Audit Pages

**Files:** `src/pages/research/`, `src/pages/projects/`, `src/pages/audits/`

Searched all files in these directories for POST handling or form processing. Only one
file has any dynamic behavior: `mitigating_breach_tls_attack_in_php.rs`, which accepts
POST but delegates to the same GET handler (no user input processing). All other pages
are `simple_page!` macros (static GET-only). No injection vectors.

**Verdict:** Clean.

### 4. BREACH Mitigation

**File:** `src/libs/breach.rs`

The `breach_encode`/`breach_decode` functions are marked `#[allow(dead_code)]` and have
a TODO comment confirming they are not actually used by the site. They exist as reference
implementations to show on the BREACH mitigation page.

`breach_visual_html()` is used to generate sample output on the BREACH mitigation page.
It inserts random HTML comments and zero-width spaces between characters. The input is
a hardcoded string literal ("Sample Header", "Sample paragraph text."), not user input.
No security concern.

**Verdict:** Clean. The code is not used for actual BREACH defense -- it is a demonstration.

### 5. Bibliography

**File:** `src/libs/bibliography.rs`

All fields (title, url, authors, date) are passed through `html_escape()` before being
inserted into HTML output. The `cite()` method HTML-escapes the numeric index. The
`render()` method uses pre-escaped reference HTML. All bibliography data comes from
hardcoded `&'static str` values in page handlers, never from user input.

**Verdict:** Clean.

### 6. Upvote System

**Files:** `src/upvote.rs`, `src/libs/upvotes.rs`, `src/middleware/upvote_post.rs`,
`src/libs/csrf.rs`

**CSRF protection:** Both the AJAX endpoint (`/upvote.php`) and the form fallback
middleware check CSRF via `csrf::check_origin()`. This validates the Origin header
(or Referer fallback) against the Host header, and rejects requests where neither
header is present. It also validates that the request Host is an accepted host
(defuse.ca or a dev host), preventing DNS rebinding attacks. This is adequate CSRF
protection for a low-stakes upvote system with no user accounts.

**Vote manipulation / ID validation:** `process_vote()` calls
`registry::is_valid_upvote_id()` which checks against a compile-time set of valid IDs
derived from the page registry. Arbitrary permanent_id values are rejected. SQL queries
use parameterized bindings throughout (no string interpolation).

**Rate limiting:** Votes are tracked per SHA256(page_id + IP) with a 24-hour expiry.
This is IP-based rate limiting. An attacker could vote-stuff by rotating IPs, but
this is a low-stakes feature (no user accounts, no money involved) and matches the
original PHP behavior.

**Race condition in process_vote:** There is a TOCTOU window between `get_user_action()`
and `set_user_action()` / counter updates. Concurrent requests from the same IP could
cause vote count drift. This is a minor data integrity issue, not a security concern,
and matches the original PHP behavior.

**Verdict:** Clean for deployment. No CSRF bypass, no SQL injection, no meaningful abuse vector.

### 7. Web Server Scan

**File:** `src/pages/services/web_server_scan.rs`, template:
`templates/pages/services/web_server_scan.html`

This is a `simple_page!` with no POST handler. The template contains a form that submits
to `web-server-scan.htm` with method="post", but since the handler does not implement
`post()`, the server returns 405 Method Not Allowed. There are no outbound network
requests, no DNS lookups based on user input, and no SSRF risk. The form appears to be
a leftover from the PHP version where the backend scanning functionality has not been
(and likely will not be) ported.

**Verdict:** Clean. No SSRF risk whatsoever.

### Additional Observations (not showstoppers)

- **`/s.php` shout endpoint:** Properly HTML-escapes all user-controlled output.
  Base64-decoded text is escaped via `html_escape()` before rendering. Clean.

- **`/ip.php`, `/ip-insecure.php`, `/getmyip.php`:** All properly escape user-visible
  output. The `getmyip.php` endpoint does a reverse DNS lookup of the client's own IP
  (not arbitrary IPs), so no SSRF concern.

- **`all_pages_html` passed through `|safe` filter:** The HTML is generated by
  `UpvoteService::render_list()` which HTML-escapes all database-sourced values (title,
  description, URL) via `html_escape()`. The permanent_id values come from the hardcoded
  page registry. This is safe.

# Security Review: XSS and Injection Vulnerabilities

**Scope:** All user-input-to-output paths, manual HTML construction, `|safe` template usage, SQL queries, command execution, header injection, and open redirects.

**Date:** 2026-02-18

---

## CONFIRMED VULNERABILITIES

### VULN-01: Stored XSS via Big Number Calculator Output [HIGH]

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/big_number_calculator/mod.rs`, lines 130-134

The `calculate()` function builds HTML output that is rendered with `|safe` in the template:

```rust
// mod.rs line 131
format!("<div style=\"text-align: right;\">{}</div>", newlines_to_br(&output))
```

And the non-rational path (line 133):
```rust
newlines_to_br(&output)
```

The `newlines_to_br` function (line 173-175) does `s.replace('\n', "<br />")` but performs **no HTML escaping** on the value itself. The `output` string comes from Ruby's stdout (evaluator.rs lines 120-121):

```rust
let stdout = String::from_utf8_lossy(&output.stdout);
let result = stdout.trim();
```

While the parser and character filter provide strong defense-in-depth against arbitrary Ruby code execution, the Ruby output itself is inserted into HTML unescaped. If Ruby ever produces output containing `<` or `>` (e.g., from error messages, or if the filter/parser is ever relaxed), it would be rendered as raw HTML.

**Current risk mitigation:** The parser validates that only arithmetic expressions pass through, and the character filter blocks `<`, `>`, `"`, `'`, etc. The Ruby output for valid arithmetic is always digits, `-`, `/`, hex chars, `true`, `false`, or `.` -- none of which are HTML-dangerous. However, this is defense by accident rather than by design.

**Template:** `/home/taylor/defuse-rewrite/defuse-rust/templates/pages/services/big_number_calculator.html`, line 34:
```html
{{ res.output|safe }}</div>
```

**Recommendation:** Apply `html_escape()` to the numeric result before wrapping it in HTML tags. The `<br />` and `<div>` tags can be added afterward. This adds a proper escaping layer so that the security of the output does not depend entirely on the input filter.

---

### VULN-02: Potential Stored XSS via Upvote Page Data from Database [MEDIUM]

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/upvotes.rs`, lines 364-430

The `render_list()` function builds HTML manually and the result is rendered via `|safe` in templates (home.html line 18, all_pages.html line 8). The function does escape `title`, `description`, `canonical_url`, and `permanent_id` using `html_escape()` and `js_escape()`:

```rust
let safe_title = html_escape(&page.title);
let safe_description = html_escape(&page.description);
let safe_url = html_escape(&page.canonical_url);
```

**However**, the `page_url` parameter (the current page's URL used as the form action) is escaped at line 471:
```rust
html_escape(page_url),
```

And the `safe_id` value used in CSS class names (lines 456-457, 498, 501-503, 521-522) is HTML-escaped but then concatenated directly into class name strings:
```rust
let up_form_name = format!("upvoteUpForm{}", safe_id);
```

Since `permanent_id` values come from the page registry (hardcoded in source), not from user input, and `page_url` comes from the registry's `relative_url()`, this is safe in practice. But if the upvote system were ever extended to accept user-provided IDs, the CSS class name injection could be exploited.

**Current status:** Safe because all data sources are trusted (registry-defined constants). No fix needed now, but worth noting for future extensibility.

---

### VULN-03: html_sanitize.rs Error Message Includes Unescaped VimHighlightError [LOW]

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/html_sanitize.rs`, line 102

```rust
fn get_source_html() -> String {
    let source_path = Path::new("static/source/HtmlEscape.php");
    vim_highlight::highlight_file(source_path, true).unwrap_or_else(|e| {
        format!("<pre>Error loading source: {}</pre>", e)
    })
}
```

The error `e` (a `VimHighlightError`) is formatted into HTML without escaping. The `VimHighlightError` variants include `IoError(String)` and `FileNotFound(String)` which contain filesystem paths. This is rendered via `{{ source_html|safe }}` in the template.

**Exploitation:** The file path is hardcoded (`static/source/HtmlEscape.php`), so an attacker cannot control the error message content. This is a code quality issue rather than an exploitable vulnerability.

**Recommendation:** Use `html_escape(&e.to_string())` in the format string for defense-in-depth.

---

### VULN-04: mitigating_breach_tls_attack_in_php.rs Same Pattern [LOW]

**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/pages/research/mitigating_breach_tls_attack_in_php.rs`, line 25

```rust
let highlighted_source = vim_highlight::highlight_file(source_path, false)
    .unwrap_or_else(|e| format!("<p>Error highlighting source: {}</p>", e));
```

Same pattern as VULN-03: error from `VimHighlightError` rendered without escaping via `|safe`. Also hardcoded path, so not exploitable.

---

## AREAS REVIEWED AND FOUND SAFE

### Pastebin View (pastebin_view.rs) -- SAFE

The manually-constructed HTML page properly escapes all user content:

1. **Server-encrypted pastes** (line 180): `html_escape(line)` is called on each line before insertion into `<li>` elements.
2. **Textarea content** (line 195): `html_escape(&paste.text)` is used.
3. **jscrypt pastes** (line 132): `js_string_escape(&paste.text)` escapes every non-alphanumeric character as `\xHH`, which is safe for JavaScript string literal injection.
4. **timeleft_display** (line 128): Comes from `format_timeleft()` which only produces digits and static English words.

### Special Endpoints (special_endpoints.rs) -- SAFE

1. **ip_insecure_php**: `html_escape(&addr.ip().to_string())` -- IP addresses are always safe, but escaping is applied anyway.
2. **getmyip_php**: All three values (IP, hostname, user_agent) are passed through `html_escape()`.
3. **shout_php**: `html_escape(&decoded)` is applied to the base64-decoded user input before rendering.

### Upvote XML Response (upvote.rs) -- SAFE

The `xml_response()` function applies `html_escape()` to `status`, `uparrow`, and `downarrow` values. These are all hardcoded string constants ("pass", "fail", "Y", "N"), so escaping is belt-and-suspenders. The `total` is an `i32`. Safe.

### Online x86 Assembler (online_x86_assembler.rs) -- SAFE

1. **hex_zero_bold** (rendered via `|safe`): Contains only uppercase hex characters and literal `<b>00</b>` tags generated by the parser from objdump output. The hex bytes come from objdump's disassembly of compiled code, not from raw user input. The pipeline: user input -> gcc assembly -> objdump disassembly -> regex extraction of hex pairs -> uppercase + bold. No user-controlled strings survive this pipeline.

2. **format_disassembly** (rendered via `|safe`): Calls `html_escape::escape_text()` which applies full HTML escaping. Safe.

3. **format_error** (rendered via `|safe`): Calls `html_escape::escape_text()` on the error message. The error message includes cleaned gcc output (temp paths stripped). Safe because it is escaped.

4. **string_literal and array_literal**: Rendered with Askama auto-escaping (no `|safe`). Safe.

### Quantum Computer Time Capsule -- SAFE

Both `encrypted_message_escaped()` and `textarea_contents_escaped()` call `html_escape()` before the result is rendered with `|safe`. Safe.

### Checksums Page -- SAFE

All values in the template use Askama auto-escaping. Hash results are hex strings. Form input is reflected via `{{ input }}` which auto-escapes. Safe.

### HTML Sanitize Page -- SAFE

The `data` field (escaped user HTML) goes through Askama auto-escaping in the template (`{{ data }}`). The `source_html` uses `|safe` but comes from vim highlighting of a static file. Safe.

### TRENT Template -- SAFE

All user-supplied values (`err`, `conf.params.name`, `conf.params.description`, `fv.*` fields, `userprintout`, `printout`, `res.password`, `res.url`, `comp.url`) are rendered via Askama's `{{ }}` which auto-escapes. No `|safe` is used on any user-controlled data. The `parse_int_or_empty` error message (line 505 of trent.rs) includes the user's input string (`format!("'{}' is not a valid number.", s)`), but this goes through `self.set_error()` -> `error` field -> `{{ err }}` in the template, which auto-escapes. Safe.

### Vim Highlight (vim_highlight.rs) -- SAFE (with caveat)

The module's own WARNING comment (lines 1-6) notes that vim's HTML generation could theoretically contain XSS if used on untrusted input. However, it is only called on:
- Static source files hardcoded in templates (e.g., `highlight_file(Path::new("static/source/..."))`)
- Hardcoded string literals in templates (e.g., `highlight_string("puts 'hello'", ...)`)

No user input is ever passed to `process_text()` or `process_file()`. Safe.

### Bibliography (bibliography.rs) -- SAFE

All reference fields (title, url, authors, date) are passed through `html_escape()` before being inserted into HTML. The `cite()` method escapes the index. The rendered bibliography is used with `|safe` in templates but all data is developer-controlled (hardcoded reference lists). Safe.

### Breach Module (breach.rs) -- SAFE

`breach_visual_html()` does not HTML-escape its input, but it is only called with hardcoded string literals ("Sample Header", "Sample paragraph text.") in `mitigating_breach_tls_attack_in_php.rs`. The output is rendered with `|safe`. Safe because input is not user-controlled.

---

## SQL INJECTION -- ALL SAFE

Every SQL query in the codebase uses parameterized queries via sqlx's `.bind()` method:

- **upvotes.rs**: All 15+ queries use `?` placeholders with `.bind()`. The `get_top_pages` function (lines 242-249) does use `format!()` to build the query string, but only to select between 4 hardcoded query variants based on `Option<&str>` and `Option<u32>` -- the actual values are always bound via `.bind()`.
- **pastebin.rs**: All queries use `?` placeholders with `.bind()`.
- **trent.rs**: All queries use `?` placeholders with `.bind()`.
- **timecapsule.rs**: All queries use `?` placeholders with `.bind()`.

No SQL injection vulnerabilities found.

---

## COMMAND INJECTION -- ALL SAFE

### x86 Assembler (x86_assembler/)

The assembly code is written to a temporary file and the file path is passed to gcc as a command-line argument (executor.rs line 117: `.arg(&source_path)`). User input never appears in the command string itself -- it is always in a file that gcc reads. The `SafeAsm` type system ensures that only validated code reaches `assemble_unsafe()`.

The `filter.rs` module implements a whitelist-based filter that:
1. Rejects input > 10KB
2. Strips whitelisted directives and decimal floats
3. Rejects if any `.` character remains (blocking `.include`, `.incbin`, etc.)
4. Rejects `#APP` / `#NO_APP` GAS special comments

This effectively prevents file inclusion attacks, macro definition, and other dangerous GAS directives.

For disassembly, user hex input is parsed into binary bytes and written to a file, then passed to objdump. No injection possible since the binary data never appears in command arguments.

### Big Number Calculator (big_number_calculator/)

User expressions are evaluated by spawning a Ruby process. The expression goes through:
1. **Character whitelist filter** (`filter.rs`): Only allows `0-9`, `a-f`, `A-F`, `r`, `(`, `)`, `*`, `^`, `|`, `&`, `%`, `/`, `+`, `-`, `<`, `>`, `.`, space, and `x`. This blocks all letters except hex digits and `r`/`x`, preventing `system()`, `exec()`, backticks, `$`, `@`, `'`, `"`, `;`, newlines, etc.
2. **AST parser validation** (`parser.rs`): A pest grammar validates the expression has valid arithmetic structure. Rejects function calls, method calls, identifiers, range operators, string literals, brackets, commas, and Ruby keywords.
3. **Shell escaping** (`evaluator.rs` line 163-166): The Ruby code is shell-escaped using single-quote wrapping with proper `'\''` escaping.
4. **Resource limits**: `ulimit -t 10 -v 262144` limits CPU time to 10 seconds and virtual memory to 256MB. `kill_on_drop(true)` ensures cleanup.

The defense-in-depth is strong. Even if one layer fails, the others prevent exploitation.

### Vim Execution (vim_highlight.rs)

Vim is only invoked on temporary files created from trusted (static) content. The `set_vim_command()` method (line 97-104) whitelists the command to only "vim", "vi", or "gvim". The `file_type` is set from hardcoded strings, and the `color_scheme` is always "default". No user input reaches any vim command-line argument.

---

## HEADER INJECTION -- ALL SAFE

### Location Headers

All Location header values are constructed from:
1. **Static strings**: `redirect_301("/pastebin.htm")` in pastebin_view.rs
2. **Server-generated keys**: `format!("/b/{}", url_key)` in pastebin_add.rs where `url_key` is a server-generated alphanumeric password
3. **URL-encoded user input**: `format!("/s.php?s={}", urlencoding::encode(&encoded))` in special_endpoints.rs -- the `urlencoding::encode()` prevents CRLF injection
4. **Middleware redirects**: `build_redirect_url()` in url_canonicalization.rs uses the request's `host` and `path` which come from the HTTP request's parsed URL (already validated by hyper/axum's HTTP parser which rejects bare CRLFs)
5. **TRENT URLs**: Built from `ctx.url_prefix` (from Host header, parsed by `.to_str()` which rejects non-visible-ASCII) and integer drawing numbers

Axum's `HeaderValue` type rejects values containing `\r` or `\n`, providing an additional layer of protection against CRLF injection.

### Content-Disposition Headers

The time capsule download (quantum_computer_time_capsule.rs line 240) includes a filename built from `chrono::Utc::now().format(...)` which produces only safe ASCII characters. No user input in the filename.

---

## OPEN REDIRECTS -- NONE FOUND

All redirect targets are either:
- Hardcoded paths (e.g., `/pastebin.htm`)
- Server-generated paths (e.g., `/b/{url_key}`)
- URL-encoded transformations of user input (e.g., `/s.php?s={base64}`)
- Host canonicalization (always redirects to `defuse.ca`)

No redirect accepts a user-supplied URL as the target.

---

## SUMMARY

| ID | Severity | Description | Status |
|----|----------|-------------|--------|
| VULN-01 | HIGH | Big number calculator output not HTML-escaped before `\|safe` rendering | Missing escaping layer -- currently safe due to input filter, but fragile |
| VULN-02 | MEDIUM | Upvote render_list builds HTML manually | Safe because all data sources are trusted; note for extensibility |
| VULN-03 | LOW | html_sanitize error path formats VimHighlightError without escaping | Not exploitable (hardcoded path) |
| VULN-04 | LOW | mitigating_breach error path same pattern | Not exploitable (hardcoded path) |

**Key finding:** The codebase is well-defended overall. Askama auto-escaping covers most templates, manual HTML construction consistently uses `html_escape()`, SQL queries are all parameterized, and command execution has strong input validation. The main recommendation is to add HTML escaping to the big number calculator output as a defense-in-depth measure so that the XSS safety of the output path does not depend solely on the input validation layers.

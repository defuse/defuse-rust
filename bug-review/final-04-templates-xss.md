# Final Security Review: Templates & XSS (final-04-templates-xss.md)

## Summary

Comprehensive review of all Askama templates for XSS vulnerabilities, focusing on
`|safe` filter usage, user-controlled data in HTML/JS/URL contexts, and sensitive
data exposure. Reviewed every `|safe` instance across all templates, traced data
flow from user input through handlers and libraries to template rendering.

## Result: No showstopper XSS issues found.

The codebase demonstrates a strong, layered approach to XSS prevention. Below is
the detailed analysis of each `|safe` usage and other potential injection points.

---

## Analysis of all `|safe` usages

### 1. Big Number Calculator (`big_number_calculator.html` line 34)

```html
{{ res.output|safe }}
```

**Data flow:** User input (expression) -> `filter::transform_operators` ->
`filter::is_safe` (character whitelist) -> `parser::validate` (AST validation) ->
`evaluator::evaluate` (Ruby process with ulimit) -> defense-in-depth assertion
that `html_escape(value) == value` (panics if violated) -> `formatter::group_digits`
(adds `&nbsp;` and spaces, all safe characters) -> `newlines_to_br` (adds `<br />`).

**Verdict: SAFE.** Three defense layers (character whitelist, AST parser, output
assertion) plus the output formatter only adds known-safe HTML entities. The assert
at `mod.rs:112-116` catches any regression.

### 2. x86 Assembler - hex_zero_bold (`online_x86_assembler.html` lines 41, 93)

```html
{{ result.hex_zero_bold|safe }}
```

**Data flow:** User assembly code -> `filter::check_code_safety` -> GCC assembly
-> objdump -> `parser::parse_objdump_output` -> regex extracts only hex bytes ->
`to_uppercase()` -> defense-in-depth assertion that `html_escape(&hex_bytes) == hex_bytes`
(panics if violated) -> replaces "ZERO" marker with `<b>00</b>`.

For disassembly: User hex input -> `parse_hex_input` (only hex digits survive) ->
binary file -> objdump -> same parser path.

**Verdict: SAFE.** The hex bytes are extracted by regex from objdump output, uppercased
(only A-F and 0-9), and the assertion at `parser.rs:108-112` confirms no HTML-special
characters. The only HTML injected is `<b>00</b>`.

### 3. x86 Assembler - format_disassembly (`online_x86_assembler.html` lines 49, 101)

```html
{{ self.format_disassembly(result)|safe }}
```

**Data flow:** `result.disassembly` (objdump text output) -> `html_escape::escape_text()`
with `br_tags=true` and `tab_width=4`.

**Verdict: SAFE.** `escape_text()` performs full HTML entity escaping (at
`html_escape.rs:40-41`: `&`, `<`, `>`, `"`, `'` are all escaped) before adding
`<br />` tags for line breaks.

### 4. x86 Assembler - error (`online_x86_assembler.html` line 58)

```html
{{ err|safe }}
```

**Data flow:** `AssemblerError::to_string()` -> `format_error()` calls
`html_escape::escape_text()` -> result stored as `error: Option<String>`.

The error messages include GCC stderr which may contain fragments of user input.
The `clean_error_message()` function does text processing, then `format_error()`
escapes everything with `html_escape::escape_text()`.

**Verdict: SAFE.** HTML escaping is applied before `|safe` rendering.

### 5. HTML Sanitize - source_html (`html_sanitize.html` line 32)

```html
{{ source_html|safe }}
```

**Data flow:** `get_source_html()` calls `vim_highlight::highlight_file(Path::new("static/source/HtmlEscape.php"))`.

**Verdict: SAFE.** Renders a static file through vim highlighting. No user input involved.

### 6. HTML Sanitize - data field (`html_sanitize.html` line 19)

```html
<textarea ...>{{ data }}</textarea>
```

Note: The `data` field is the result of `html_escape::escape_text()` on user input
(when submitted) or the raw user input (when not submitted). Either way, Askama
auto-escapes `{{ data }}` here since there is no `|safe` filter. The post handler
at `html_sanitize.rs:68` calls `escape_text()` which produces HTML with `<br />`
and `&nbsp;` entities -- but since this goes into a `<textarea>`, Askama will
double-escape those entities, which is the correct behavior (the user sees the
HTML source).

**Verdict: SAFE.** Askama auto-escaping handles this correctly.

### 7. Quantum Time Capsule (`quantum_computer_time_capsule.html` lines 27, 48)

```html
>{{ self.encrypted_message_escaped()|safe }}</div>
>{{ self.textarea_contents_escaped()|safe }}</textarea>
```

**Data flow:** Both `encrypted_message_escaped()` and `textarea_contents_escaped()`
call `html_escape()` from `util.rs` which escapes `&`, `<`, `>`, `"`, `'`.

For the encrypted message: form fields are validated as printable ASCII without
spaces (bytes 0x21-0x7E) at `quantum_computer_time_capsule.rs:122`, so they cannot
contain HTML-special characters at all. The `html_escape()` is defense-in-depth.

For textarea contents: this is the raw user message, escaped by `html_escape()`.

**Verdict: SAFE.** Explicit HTML escaping applied before `|safe`.

### 8. BREACH mitigation page (`mitigating_breach_tls_attack_in_php.html` lines 23, 30-31)

```html
{{ highlighted_source|safe }}
{{ sample_header|safe }}
{{ sample_paragraph|safe }}
```

**Data flow:**
- `highlighted_source`: vim highlight of static file `static/source/breach.php`
- `sample_header`: `breach_visual_html("Sample Header")` - hardcoded string
- `sample_paragraph`: `breach_visual_html("Sample paragraph text.")` - hardcoded string

**Verdict: SAFE.** All inputs are hardcoded literals, no user input.

### 9. Bibliography citations (cbcmodeiv.html, flush_reload_side_channel.html)

```html
{{ bib.cite(1)|safe }}
{{ bib.render()|safe }}
```

**Data flow:** Bibliography entries are hardcoded in the page handler Rust code.
The `Bibliography::new()` method calls `html_escape()` on all fields (title, url,
authors, date) at `bibliography.rs:26-34`. The `cite()` method uses numeric indices.

**Verdict: SAFE.** All data is hardcoded and HTML-escaped.

### 10. Vim highlight calls (many templates)

All `vim_highlight::highlight_string(...)` and `vim_highlight::highlight_file(...)`
calls in templates use hardcoded string literals and hardcoded file paths. None
accept user-controlled input.

**Verdict: SAFE.** No user input reaches vim.

### 11. Home page and All Pages (`home.html`, `all_pages.html`)

```html
{{ top_pages_html|safe }}
{{ all_pages_html|safe }}
```

**Data flow:** `UpvoteService::render_list()` generates HTML from the page registry
(hardcoded page metadata) and vote counts (integers from database). The upvote IDs
come from the hardcoded page registry (`permanent_id`), not user input.

**Verdict: SAFE.** Data originates from hardcoded page metadata and integer vote counts.

---

## Other template security checks

### User data in HTML attribute contexts

- **`<input value="{{ eqn }}"/>`** (big_number_calculator.html line 7): Askama
  auto-escapes. SAFE.
- **`<textarea>{{ instructions }}</textarea>`** (online_x86_assembler.html line 26):
  Askama auto-escapes. SAFE.
- **`<textarea>{{ hexstring }}</textarea>`** (online_x86_assembler.html line 78):
  Askama auto-escapes. SAFE.
- **`<textarea>{{ data }}</textarea>`** (checksums.html line 7):
  Askama auto-escapes. SAFE.
- **`<textarea>{{ input }}</textarea>`** (html_sanitize.html line 18-19):
  Askama auto-escapes. SAFE.
- **TRENT form values** (trustedthirdparty.html, various hidden inputs and form
  fields): All use `{{ value }}` without `|safe`, so Askama auto-escapes. SAFE.

### User data in URL (href) contexts

- **`<a href="{{ res.url }}">{{ res.url }}</a>`** (trustedthirdparty.html line 84):
  The URL is constructed at `trent.rs:267-269` as
  `format!("{}/trustedthirdparty.htm?drawingnum={}", self.ctx.url_prefix, result.drawing_num)`
  where `url_prefix` is server-controlled and `drawing_num` is an i32 from the
  database auto-increment. No user-controlled strings in the URL. SAFE.
- **`<a href="{{ comp.url }}">{{ comp.url }}</a>`** (trustedthirdparty.html line 163):
  Same pattern - constructed from `url_prefix` and integer drawing number. SAFE.

### User data in JavaScript contexts

- **Pastebin view jscrypt** (`pastebin_view.rs` line 137): Ciphertext is passed through
  `js_string_escape()` which escapes ALL non-alphanumeric characters as `\xHH`.
  Inserted into a JavaScript string literal within double quotes. SAFE.
- **Upvote IDs in `onsubmit` handlers** (`base.html` lines 240, 265): The
  `upvote_cfg.id` comes from the hardcoded page registry, not user input. SAFE.

### Sensitive data exposure

- **`{{ ctx.client_ip }}`** in base.html footer: This displays the visitor's own IP
  address (standard behavior for the original PHP site). Auto-escaped by Askama.
  Not a security issue.
- **No API keys, secrets, or internal paths** are rendered in any template. Database
  URLs, RECAPTCHA_SECRET_KEY, and STORAGE_PATH are only used server-side.
- **Error messages** shown to users are hardcoded strings (e.g., "Database error.
  Please try again."), not raw error details. Internal errors are logged via
  `tracing::error!()` server-side only.
- **CatchPanicLayer** is configured (`main.rs:185`) to prevent stack traces from
  reaching users on panics.
- **panic_test page** is registered as a route but this is intentional for testing
  and does not leak sensitive information (it triggers CatchPanicLayer which returns
  a plain 500).

---

## Defense-in-depth measures noted

1. **Runtime assertions in security-critical paths**: Both `big_number_calculator/mod.rs`
   and `x86_assembler/parser.rs` contain assertions that verify HTML escaping is a
   no-op on output data before it reaches `|safe`. These will crash the request
   (caught by CatchPanicLayer) rather than allow XSS if a regression occurs.

2. **Type-level safety**: `SafeExpr` and `SafeAsm` types ensure validated code
   cannot be bypassed. The executor functions are `pub(super)` to prevent external
   callers from skipping validation.

3. **Multiple escaping layers**: User-facing pages that render user data through
   `|safe` always apply explicit HTML escaping first (via `html_escape::escape_text()`
   or `util::html_escape()`).

4. **Vim highlighting restricted to static content**: The module has an explicit
   warning comment about not using it on untrusted input, and all template calls
   use hardcoded strings/paths.

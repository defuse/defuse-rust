# Path Traversal and File Access Security Review

## Scope

All file I/O in the defuse-rust codebase, with a focus on whether
user-controlled input can be used to read or write files outside
intended directories.

---

## FINDING 1: Blog slug redirect probes filesystem with user-controlled path (Info Leak)

**Severity: Low**
**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/middleware/url_canonicalization.rs`, lines 167-192

```rust
fn check_blog_slug_redirect(path: &str) -> Option<String> {
    if !path.starts_with("/blog/") {
        return None;
    }
    let after_blog = &path[6..]; // Skip "/blog/"
    if after_blog.contains('.') || after_blog.is_empty() {
        return None;
    }
    if path.ends_with('/') {
        return None;
    }
    // Check if the .html file exists
    let html_path = format!("static{}.html", path);
    if std::path::Path::new(&html_path).exists() {
        return Some(format!("{}.html", path));
    }
    None
}
```

The URL path is used to construct a filesystem path: `format!("static{}.html", path)`.
An attacker could send a request like `GET /blog/../../Cargo` which would
construct the path `static/blog/../../Cargo.html` = `Cargo.html`. If that file
existed, the function would return `Some("/blog/../../Cargo.html")` and produce
a 301 redirect, leaking the existence of that file.

**However, the practical impact is very low for the following reasons:**

1. The `.html` suffix is always appended, so only files ending in `.html` can
   be probed. This limits what can be discovered.
2. The `after_blog.contains('.')` check blocks paths containing dots, which
   means `..` (the core of path traversal) is already blocked. A path like
   `/blog/../foo` has `../foo` after `/blog/`, and that contains `.`, so it
   returns `None` immediately.

**The dot check makes this safe in practice.** The check was added because
"blog slugs don't contain dots" but it also serves as an inadvertent
path-traversal guard. Worth noting in a comment, since removing or relaxing
the dot check in the future would introduce a real vulnerability.

**Recommendation:** Add a comment noting the security significance of the dot
check, so future maintainers don't remove it without understanding the
consequence:

```rust
// Skip if already has an extension (contains a dot after /blog/)
// This would break if blog slugs contained ., but they do not.
// SECURITY: This also blocks path traversal (../) since dots are rejected.
```

---

## FINDING 2: `.env` file with database credentials exists on disk at the project root

**Severity: Informational (deployment concern)**
**File:** `/home/taylor/defuse-rewrite/defuse-rust/.env`

The `.env` file contains plaintext database credentials (MySQL passwords,
reCAPTCHA secret key). The static file server is rooted at `static/`, not
the project root, so `ServeDir` cannot serve it. However, this file must
never be deployed to a production server, or if it is, it must be excluded
from the serving directory.

The `.env` is listed in `.gitignore`, so it won't be committed. The
`ServeDir::new("static")` call in `main.rs` roots file serving in the
`static/` subdirectory, not the project root, so a request to `GET /.env`
would look for `static/.env`, which does not exist.

**Status: Safe**, but worth verifying during deployment that the working
directory of the production binary does not place `.env` inside `static/`.

---

## FINDING 3: tower-http ServeDir path traversal protection

**Severity: Safe (no issue)**
**Files:**
- `/home/taylor/defuse-rewrite/defuse-rust/src/main.rs`, line 128
- `/home/taylor/defuse-rewrite/defuse-rust/src/storage_routes.rs`, lines 33-48

Both `ServeDir::new("static")` and `ServeDir::new(extras_path.join("files"))`
(and similar) rely on tower-http 0.5's `ServeDir`. This implementation
internally calls `http::Uri::path()` which already decodes percent-encoding,
and then uses Rust's `std::path::Path::join()` which does NOT allow `..`
components to escape the root. Specifically, tower-http's `ServeDir` calls
`sanitize_path()` internally, which rejects any path containing `..`
components.

This means requests like:
- `GET /files/../../.env`
- `GET /files/%2e%2e/%2e%2e/.env`
- `GET /..%252f..%252f.env`

All correctly either get sanitized or rejected by tower-http before any
filesystem access occurs.

**Status: Safe.** tower-http 0.5 has well-tested path traversal protection.

---

## FINDING 4: Storage root directory is not directly served

**Severity: Safe (no issue)**
**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/storage_routes.rs`

```rust
// /storage itself contains credentials, only ever serve dirs in "storage/extras"!
let extras_path = storage_path.join("extras");
```

The code correctly pins each route to a subdirectory under `extras/` and never
exposes the storage root. The comment shows this was an intentional security
decision. Combined with ServeDir's traversal protection, there is no way to
access files outside the individual `extras/{subdir}/` directories.

**Status: Safe.**

---

## FINDING 5: TRENT temp file paths -- well defended

**Severity: Safe (no issue)**
**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/trent.rs`, lines 526-529

```rust
fn temp_path(drawing_num: i32, hash: &str) -> String {
    // Defense-in-depth against path traversal attacks.
    assert!(trent::is_sha256_hex(hash));
    format!("/tmp/trent-{}-{}", drawing_num, hash)
}
```

The `hash` parameter is validated by `is_sha256_hex()` which requires exactly
64 hex characters (`[0-9a-fA-F]`). This makes path traversal via the hash
impossible since `/`, `.`, and `\` are all rejected.

The `drawing_num` is an `i32`, so its string representation is limited to
digits and a leading `-` sign. Neither character enables path traversal.

**The `load_temp_file` function also validates the hash before calling
`temp_path`**, providing defense even if `temp_path` were called from an
unexpected code path:

```rust
async fn load_temp_file(drawing_num: i32, hash: &str) -> Option<Vec<u8>> {
    if !trent::is_sha256_hex(hash) {
        return None;
    }
    tokio::fs::read(temp_path(drawing_num, hash)).await.ok()
}
```

Note: `temp_path` uses `assert!()` rather than returning an error, so in
release builds with assertions enabled, a bad hash would panic. In release
builds with `debug_assertions = false`, the assert may be compiled out,
but `load_temp_file` and `delete_temp_files` both check `is_sha256_hex`
before calling `temp_path`, so the assert is defense-in-depth only.

**Minor note:** `save_files_to_temp` calls `temp_path` with hashes that were
set earlier in `handle_create` via either `trent::sha256_hex(data)` (which
always produces valid hex) or from the form's `file1hash`/`file2hash`/`file3hash`
fields. In the confirmed path, hashes come from user input, but
`validate_create_request` calls `validate_files` which checks
`sha256_hex(content) != file.hash`, so the hash must be a valid SHA-256 hex
string that matches actual file content. This path is safe.

**Status: Safe.** Defense in depth is well-implemented.

---

## FINDING 6: Vim highlight cache -- no user-controlled paths

**Severity: Safe (no issue)**
**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/vim_highlight.rs`

Cache file paths are constructed from MD5 hashes:

```rust
Some(cache_dir_path.join(format!("string-{}{}", hash, CACHE_SUFFIX)))
```

Where `hash` is computed from `Md5::digest(content.as_bytes())` formatted as
hex. These are server-generated hex strings, never user input. No user-controlled
data enters the file path.

The `process_file` function takes a `&Path` but all callers pass
hardcoded paths:
- `highlight_file(Path::new("static/source/HtmlEscape.php"), ...)` (html_sanitize.rs)
- `highlight_file(Path::new("static/source/breach.php"), ...)` (mitigating_breach.rs)
- `highlight_storage_file(relative_path, ...)` where `relative_path` is a hardcoded
  string literal from the page handler.

No user input reaches `process_file` or `highlight_storage_file`.

**Status: Safe.**

---

## FINDING 7: Quantum time capsule reads hardcoded static file paths

**Severity: Safe (no issue)**
**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/pages/services/quantum_computer_time_capsule.rs`, lines 262-291

All file reads use hardcoded relative paths like:
```rust
tokio::fs::read_to_string("static/timecapsule/archive-header.txt").await?;
```

No user input influences these paths.

**Status: Safe.**

---

## FINDING 8: x86 assembler writes to temp directories only

**Severity: Safe (no issue)**
**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/libs/x86_assembler/executor.rs`

The assembler creates files in a `TempDir` (which is automatically cleaned up)
and only writes user input through the `SafeAsm` type which requires prior
validation. Filenames are hardcoded (`code.s`, `code.o`, `code.bin`). No path
traversal is possible.

**Status: Safe.**

---

## FINDING 9: Static file serving working directory assumption

**Severity: Informational**
**File:** `/home/taylor/defuse-rewrite/defuse-rust/src/main.rs`, line 128

```rust
ServeDir::new("static")
```

This uses a relative path, meaning the static directory is resolved relative to
the current working directory when the server starts. If the server were started
from the wrong directory, `static` would resolve to an unexpected location.

The same applies to the blog slug redirect's `format!("static{}.html", path)`
filesystem check and the time capsule's `"static/timecapsule/..."` reads.

This is standard practice for Rust web servers, but worth documenting in
deployment instructions: the server MUST be started from the project root
directory (where `static/` exists as a subdirectory).

**Status: Informational.** Not a vulnerability, but a deployment footgun.

---

## Summary

| # | Area | Severity | Status |
|---|------|----------|--------|
| 1 | Blog slug filesystem probe | Low | Safe due to dot check; add security comment |
| 2 | `.env` on disk | Informational | Safe in current config; deployment concern |
| 3 | ServeDir traversal | Safe | tower-http prevents it |
| 4 | Storage root not exposed | Safe | Correctly scoped to extras/ subdirs |
| 5 | TRENT temp files | Safe | Hash validation prevents traversal |
| 6 | Vim cache paths | Safe | No user input in paths |
| 7 | Time capsule file reads | Safe | Hardcoded paths only |
| 8 | x86 assembler temp files | Safe | TempDir with hardcoded names |
| 9 | Relative `static` path | Informational | Deployment documentation concern |

**Overall assessment:** The codebase has no exploitable path traversal
vulnerabilities. The tower-http ServeDir provides robust protection for static
and storage file serving. User-controlled data that touches the filesystem
(TRENT temp files, vim cache) is properly validated. The one area worth
hardening is adding a security comment to the blog slug dot-check so the
inadvertent protection is preserved by future maintainers.

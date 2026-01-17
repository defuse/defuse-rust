# CRITICAL: /source/ Directory Executes PHP Instead of Serving Source Code

**Status:** OPEN
**Severity:** CRITICAL
**Discovered:** 2026-01-16
**Affects:** defuse.ca (production)

## Summary

The `/source/` directory on defuse.ca is executing PHP files instead of serving them as raw source code for download. This exposes potentially vulnerable code paths to the internet.

## Proof of Concept

```bash
$ curl -s "https://defuse.ca/source/pdfcleaner.php"
Not a PDF file!

$ curl -s -I "https://defuse.ca/source/pdfcleaner.php" | grep Content-Type
Content-Type: text/html; charset=UTF-8
```

The response is `text/html` and contains PHP execution output, NOT raw PHP source code.

## Security Impact

`pdfcleaner.php` contains system command execution:
- `pdf2ps`
- `ps2pdf`
- `pdftotext`

These commands process user-uploaded PDF files. The PDF parsing libraries on an old Debian system are likely to have known vulnerabilities that could lead to:
- Remote code execution via malicious PDF
- Server compromise

## Expected Behavior

Files in `/source/` should be served as raw text/downloads:
- `Content-Type: text/plain` or `application/octet-stream`
- Response body should contain `<?php` (raw source code)
- `Content-Disposition: attachment` to trigger download

## Affected Files

At minimum:
- `/source/pdfcleaner.php` - CRITICAL (has command execution)
- `/source/breach.php` - Lower risk (only defines functions)
- Potentially other PHP files in `/source/`

## Remediation

1. **Immediate**: Remove or disable `/source/pdfcleaner.php` on production
2. **Proper fix**: Configure web server to serve `/source/*.php` as plain text, not execute them

## Rust Implementation

The Rust rewrite MUST serve `/source/` files as raw downloads, not execute them. Test `source_php_not_executed` will verify this.

# Issue: Add Content-Security-Policy header

## Summary

Add a Content-Security-Policy (CSP) header to improve security against XSS attacks.

## Details

Currently defuse.ca does not have a CSP header. Adding one to the Rust rewrite would be a security improvement, but requires careful implementation.

## Considerations

- **Inline scripts/styles**: The site likely has inline JS and CSS that would break with a strict CSP
- **Options**:
  1. Add nonces to all inline scripts/styles
  2. Use `unsafe-inline` (defeats much of the purpose)
  3. Move all JS/CSS to external files
  4. Use CSP in report-only mode first to identify violations

## Suggested approach

1. Start with `Content-Security-Policy-Report-Only` to log violations without breaking anything
2. Audit all inline scripts and styles
3. Either refactor to external files or implement nonce-based CSP
4. Add integration test to verify header presence

## References

- MDN CSP docs: https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP
- CSP Evaluator: https://csp-evaluator.withgoogle.com/

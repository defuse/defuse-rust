//! Shared utility functions.

/// Escape HTML special characters to prevent XSS.
///
/// Escapes: `&`, `<`, `>`, `"`, `'`
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

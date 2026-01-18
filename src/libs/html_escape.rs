//! HTML text escaping that preserves visual appearance.
//!
//! This is a direct port of the PHP HtmlEscape class from defuse.ca.
//! It converts text so that it looks and behaves exactly like it does
//! in a text editor when displayed in HTML.
//!
//! The processing order is critical and must match the PHP implementation:
//! 1. Tab → Spaces (cursor-position-aware)
//! 2. HTML entity escaping
//! 3. Double space → " &nbsp;"
//! 4. Leading space → &nbsp;
//! 5. Trailing space before line ending → &nbsp;
//! 6. Line ending conversion (if br_tags enabled)

use regex::Regex;
use std::sync::LazyLock;

// Pre-compiled regex for leading space replacement
// Note: PHP's /^\x20/m matches space at start of line (after \n)
static LEADING_SPACE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^ ").unwrap());

/// Escape text for HTML display, preserving visual appearance.
///
/// # Arguments
/// * `text` - The input text to escape
/// * `br_tags` - Whether to convert line endings to `<br />` tags
/// * `tab_width` - The tab stop width (must be >= 1)
///
/// # Returns
/// The escaped HTML string that will render identically to how the
/// original text appears in a text editor.
pub fn escape_text(text: &str, br_tags: bool, tab_width: usize) -> String {
    // Step 1: Replace tabs with spaces
    // Must be done before htmlspecialchars because the tab width
    // is dependent upon the cursor position.
    let esc = tabs_to_spaces(text, tab_width);

    // Step 2: Escape all characters that have a special meaning in HTML
    let esc = html_special_chars(&esc);

    // Step 3: Replace repeated spaces with &nbsp;
    // This is tricky. Spaces cannot simply be replaced with &nbsp;
    // because the line of text will not break, so we have to leave
    // normal spaces in between pairs of &nbsp; to let the line break.
    // The space must come before the &nbsp; because we want three
    // spaces in a row to turn into " &nbsp; " not "&nbsp;  " (which
    // will look like two spaces in the browser).
    let esc = esc.replace("  ", " &nbsp;");

    // Step 4: HTML ignores leading spaces in elements like <p> and <div>
    // so we have to replace spaces at the beginning of the line with &nbsp;
    // NOTE: PHP's /^\x20/m matches after \n but NOT after standalone \r
    let esc = LEADING_SPACE_REGEX.replace_all(&esc, "&nbsp;").into_owned();

    // Step 5: The same thing happens when the space is at the end of a line.
    // Trailing spaces matter when someone copies text from the page.
    // Note: Can't use regex lookahead in Rust, so we replace " \r" and " \n" directly.
    // Order matters: replace " \r\n" pattern via " \r" first, then " \n"
    let esc = esc.replace(" \r", "&nbsp;\r").replace(" \n", "&nbsp;\n");

    // Step 6: Add <br /> tags if requested
    if br_tags {
        // To add <br /> tags, we first normalize the line endings to \n
        // First convert Windows-style CRLF lines to \n
        let esc = esc.replace("\r\n", "\n");
        // Then convert Mac-style CR lines to \n. Order matters here.
        // If we did this before replacing \r\n, we would replace \r\n with
        // \n\n, which will be two lines instead of one.
        let esc = esc.replace("\r", "\n");
        // Then add a <br /> before each \n
        esc.replace("\n", "<br />\n")
    } else {
        esc
    }
}

/// Convert tabs to spaces based on cursor position.
///
/// This mimics how text editors handle tabs - advancing to the next
/// tab stop position. The cursor position resets to 0 after each
/// newline character (\n or \r).
fn tabs_to_spaces(text: &str, tab_width: usize) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    let mut cursor: usize = 0;

    for ch in text.chars() {
        if ch == '\t' {
            // Add spaces until the cursor position is divisible by
            // tab_width, adding at least one space so that if cursor
            // is already divisible by tab_width, we add tab_width spaces.
            result.push(' ');
            cursor += 1;
            while cursor % tab_width != 0 {
                result.push(' ');
                cursor += 1;
            }
        } else {
            result.push(ch);
            cursor += 1;
            // Reset the cursor position to zero on CR or LF
            if ch == '\n' || ch == '\r' {
                cursor = 0;
            }
        }
    }

    result
}

/// Escape HTML special characters (equivalent to PHP htmlspecialchars with ENT_QUOTES).
///
/// Converts:
/// - `&` → `&amp;`
/// - `<` → `&lt;`
/// - `>` → `&gt;`
/// - `"` → `&quot;`
/// - `'` → `&#039;` (ENT_QUOTES uses numeric entity for single quote)
fn html_special_chars(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 2);

    for ch in text.chars() {
        match ch {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#039;"),
            _ => result.push(ch),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Basic entity escaping
    #[test]
    fn test_escape_less_than() {
        assert_eq!(escape_text("<", true, 8), "&lt;");
    }

    #[test]
    fn test_escape_greater_than() {
        assert_eq!(escape_text(">", true, 8), "&gt;");
    }

    #[test]
    fn test_escape_ampersand() {
        assert_eq!(escape_text("&", true, 8), "&amp;");
    }

    #[test]
    fn test_escape_double_quote() {
        assert_eq!(escape_text("\"", true, 8), "&quot;");
    }

    #[test]
    fn test_escape_single_quote() {
        assert_eq!(escape_text("'", true, 8), "&#039;");
    }

    #[test]
    fn test_escape_all_special() {
        assert_eq!(escape_text("<>&\"'", true, 8), "&lt;&gt;&amp;&quot;&#039;");
    }

    // Tab conversion
    #[test]
    fn test_tab_at_start() {
        // Tab at position 0 with tw=8 → 8 spaces
        assert_eq!(
            escape_text("\t", true, 8),
            "&nbsp;&nbsp; &nbsp; &nbsp; &nbsp;"
        );
    }

    #[test]
    fn test_tab_at_pos_1() {
        // 'a' + tab: 1 char + 7 spaces = 8 visual positions
        assert_eq!(escape_text("a\t", true, 8), "a &nbsp; &nbsp; &nbsp; ");
    }

    #[test]
    fn test_tab_at_pos_7() {
        // 7 chars + 1 space = 8 visual positions
        assert_eq!(escape_text("aaaaaaa\t", true, 8), "aaaaaaa ");
    }

    #[test]
    fn test_tab_width_4() {
        assert_eq!(escape_text("\t", true, 4), "&nbsp;&nbsp; &nbsp;");
    }

    #[test]
    fn test_tab_after_newline() {
        // Cursor resets after newline
        assert_eq!(
            escape_text("aaaa\n\t", true, 8),
            "aaaa<br />\n&nbsp;&nbsp; &nbsp; &nbsp; &nbsp;"
        );
    }

    // Space handling
    #[test]
    fn test_single_space() {
        assert_eq!(escape_text("a b", true, 8), "a b");
    }

    #[test]
    fn test_double_space() {
        assert_eq!(escape_text("a  b", true, 8), "a &nbsp;b");
    }

    #[test]
    fn test_triple_space() {
        assert_eq!(escape_text("a   b", true, 8), "a &nbsp; b");
    }

    #[test]
    fn test_leading_space() {
        assert_eq!(escape_text(" a", true, 8), "&nbsp;a");
    }

    #[test]
    fn test_leading_two_spaces() {
        assert_eq!(escape_text("  a", true, 8), "&nbsp;&nbsp;a");
    }

    #[test]
    fn test_trailing_space_lf() {
        assert_eq!(escape_text("a \n", true, 8), "a&nbsp;<br />\n");
    }

    // Line endings
    #[test]
    fn test_br_unix_lf() {
        assert_eq!(escape_text("a\nb", true, 8), "a<br />\nb");
    }

    #[test]
    fn test_br_windows_crlf() {
        assert_eq!(escape_text("a\r\nb", true, 8), "a<br />\nb");
    }

    #[test]
    fn test_br_mac_cr() {
        assert_eq!(escape_text("a\rb", true, 8), "a<br />\nb");
    }

    #[test]
    fn test_no_br_lf() {
        assert_eq!(escape_text("a\nb", false, 8), "a\nb");
    }

    #[test]
    fn test_no_br_crlf() {
        assert_eq!(escape_text("a\r\nb", false, 8), "a\r\nb");
    }

    // Leading space after CR quirk
    #[test]
    fn test_leading_after_cr() {
        // NOTE: PHP's /^\x20/m only matches after \n, not after standalone \r
        // So leading space after \r stays as regular space
        assert_eq!(escape_text("a\r b", true, 8), "a<br />\n b");
    }

    #[test]
    fn test_leading_after_lf() {
        assert_eq!(escape_text("a\n b", true, 8), "a<br />\n&nbsp;b");
    }

    // Complex cases
    #[test]
    fn test_html_in_code() {
        assert_eq!(
            escape_text("<div>\n  text\n</div>", true, 8),
            "&lt;div&gt;<br />\n&nbsp;&nbsp;text<br />\n&lt;/div&gt;"
        );
    }

    #[test]
    fn test_unicode_preserved() {
        assert_eq!(escape_text("日本語 テスト", true, 8), "日本語 テスト");
    }

    #[test]
    fn test_unicode_double_space() {
        assert_eq!(escape_text("日本語  テスト", true, 8), "日本語 &nbsp;テスト");
    }
}

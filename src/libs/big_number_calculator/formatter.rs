//! Output formatting for calculator results.
//!
//! Provides digit grouping with spaces and line breaking for long numbers.
//! Matches the PHP implementation's formatting exactly.

/// Group digits with spaces at the specified interval.
///
/// For example, with interval=3: "123456789" -> "&nbsp;&nbsp;123 456 789"
/// The leading &nbsp; pads to align the first group.
///
/// Handles negative numbers by preserving the leading minus sign.
pub fn group_digits(text: &str, interval: usize) -> String {
    if interval == 0 {
        return text.to_string();
    }

    // Handle negative numbers
    let (prefix, digits) = if text.starts_with('-') {
        ("-", &text[1..])
    } else {
        ("", text)
    };

    // Calculate padding needed for first group
    let out_digits = digits.len() % interval;

    let mut result = String::new();

    // Add prefix (minus sign if negative)
    result.push_str(prefix);

    // Add padding and first partial group if needed
    if out_digits > 0 {
        // Pad with &nbsp; to align
        for _ in 0..(interval - out_digits) {
            result.push_str("&nbsp;");
        }
        result.push_str(&digits[..out_digits]);
        result.push(' ');
    }

    // Add remaining full groups
    let remaining = &digits[out_digits..];
    for (i, ch) in remaining.chars().enumerate() {
        if i > 0 && i % interval == 0 {
            result.push(' ');
        }
        result.push(ch);
    }

    result
}

/// Break long lines at the specified length with <br /> tags.
///
/// For example, with line_length=60, a 150 character string becomes
/// three lines separated by <br /> tags.
pub fn break_lines(text: &str, line_length: usize) -> String {
    if line_length == 0 || text.len() <= line_length {
        return text.to_string();
    }

    let mut result = Vec::new();
    let mut start = 0;

    while start < text.len() {
        let end = std::cmp::min(start + line_length, text.len());
        result.push(&text[start..end]);
        start = end;
    }

    result.join("<br />")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_digits_exact_multiple() {
        // 9 digits, interval 3 = 3 groups, no padding needed
        assert_eq!(group_digits("123456789", 3), "123 456 789");
    }

    #[test]
    fn test_group_digits_with_padding() {
        // 8 digits, interval 3 = needs 1 nbsp padding
        assert_eq!(group_digits("12345678", 3), "&nbsp;12 345 678");
    }

    #[test]
    fn test_group_digits_small() {
        // 2 digits, interval 3 = needs 1 nbsp padding
        // PHP adds trailing space after first partial group when no more digits follow
        assert_eq!(group_digits("42", 3), "&nbsp;42 ");
    }

    #[test]
    fn test_group_digits_single() {
        // 1 digit, interval 3 = needs 2 nbsp padding
        // PHP adds trailing space after first partial group when no more digits follow
        assert_eq!(group_digits("5", 3), "&nbsp;&nbsp;5 ");
    }

    #[test]
    fn test_group_digits_hex() {
        // Hex grouping by 4
        assert_eq!(group_digits("ffffffff", 4), "ffff ffff");
    }

    #[test]
    fn test_group_digits_negative() {
        assert_eq!(group_digits("-12345678", 3), "-&nbsp;12 345 678");
    }

    #[test]
    fn test_break_lines_short() {
        assert_eq!(break_lines("short", 60), "short");
    }

    #[test]
    fn test_break_lines_exact() {
        let text = "a".repeat(60);
        assert_eq!(break_lines(&text, 60), text);
    }

    #[test]
    fn test_break_lines_long() {
        let text = "a".repeat(150);
        let expected = format!("{}<br />{}<br />{}", "a".repeat(60), "a".repeat(60), "a".repeat(30));
        assert_eq!(break_lines(&text, 60), expected);
    }
}

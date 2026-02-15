//! Shared utility functions.

use axum::http::HeaderMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Trusted proxy IPs that are allowed to set X-Forwarded-For / X-Real-IP headers.
/// Only connections from these IPs will have forwarding headers trusted.
const TRUSTED_PROXIES: &[IpAddr] = &[
    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
    IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
];

/// Extract the client IP address from connection info and headers.
///
/// If the connection is from a trusted proxy (localhost), checks X-Forwarded-For
/// and X-Real-IP headers. Otherwise, uses the actual connection IP.
///
/// SECURITY: Forwarding headers are only trusted from TRUSTED_PROXIES to prevent
/// IP spoofing from direct connections.
pub fn client_ip(connection_ip: IpAddr, headers: &HeaderMap) -> String {
    if TRUSTED_PROXIES.contains(&connection_ip) {
        // Check X-Forwarded-For first (standard proxy header)
        let forwarded_ip = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string());

        // Check X-Real-IP as fallback
        let forwarded_ip = forwarded_ip.or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        });

        // Use forwarded IP if available, otherwise connection IP
        forwarded_ip.unwrap_or_else(|| connection_ip.to_string())
    } else {
        // Direct connection - use actual connection IP, ignore any headers
        connection_ip.to_string()
    }
}

/// Escape HTML special characters to prevent XSS.
/// Also encodes non-ASCII characters as HTML entities to match PHP's htmlentities().
///
/// Escapes: `&`, `<`, `>`, `"`, `'`, and non-ASCII characters
pub fn html_escape(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            // These ones matter for security
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#x27;"),
            // Use named entities for common characters to match PHP htmlentities()
            c if !c.is_ascii() => {
                if let Some(entity) = named_html_entity(c) {
                    result.push_str(entity);
                } else {
                    result.push_str(&format!("&#{};", c as u32));
                }
            }
            c => result.push(c),
        }
    }
    result
}

/// Returns the named HTML entity for common characters to match PHP output
/// NOT to be used for escaping for security purposes
fn named_html_entity(c: char) -> Option<&'static str> {
    // Latin-1 Supplement named entities (most common ones from PHP)
    match c {
        '\u{00A0}' => Some("&nbsp;"),
        '\u{00A1}' => Some("&iexcl;"),
        '\u{00A2}' => Some("&cent;"),
        '\u{00A3}' => Some("&pound;"),
        '\u{00A4}' => Some("&curren;"),
        '\u{00A5}' => Some("&yen;"),
        '\u{00A6}' => Some("&brvbar;"),
        '\u{00A7}' => Some("&sect;"),
        '\u{00A8}' => Some("&uml;"),
        '\u{00A9}' => Some("&copy;"),
        '\u{00AA}' => Some("&ordf;"),
        '\u{00AB}' => Some("&laquo;"),
        '\u{00AC}' => Some("&not;"),
        '\u{00AD}' => Some("&shy;"),
        '\u{00AE}' => Some("&reg;"),
        '\u{00AF}' => Some("&macr;"),
        '\u{00B0}' => Some("&deg;"),
        '\u{00B1}' => Some("&plusmn;"),
        '\u{00B2}' => Some("&sup2;"),
        '\u{00B3}' => Some("&sup3;"),
        '\u{00B4}' => Some("&acute;"),
        '\u{00B5}' => Some("&micro;"),
        '\u{00B6}' => Some("&para;"),
        '\u{00B7}' => Some("&middot;"),
        '\u{00B8}' => Some("&cedil;"),
        '\u{00B9}' => Some("&sup1;"),
        '\u{00BA}' => Some("&ordm;"),
        '\u{00BB}' => Some("&raquo;"),
        '\u{00BC}' => Some("&frac14;"),
        '\u{00BD}' => Some("&frac12;"),
        '\u{00BE}' => Some("&frac34;"),
        '\u{00BF}' => Some("&iquest;"),
        '\u{00C0}' => Some("&Agrave;"),
        '\u{00C1}' => Some("&Aacute;"),
        '\u{00C2}' => Some("&Acirc;"),
        '\u{00C3}' => Some("&Atilde;"),
        '\u{00C4}' => Some("&Auml;"),
        '\u{00C5}' => Some("&Aring;"),
        '\u{00C6}' => Some("&AElig;"),
        '\u{00C7}' => Some("&Ccedil;"),
        '\u{00C8}' => Some("&Egrave;"),
        '\u{00C9}' => Some("&Eacute;"),
        '\u{00CA}' => Some("&Ecirc;"),
        '\u{00CB}' => Some("&Euml;"),
        '\u{00CC}' => Some("&Igrave;"),
        '\u{00CD}' => Some("&Iacute;"),
        '\u{00CE}' => Some("&Icirc;"),
        '\u{00CF}' => Some("&Iuml;"),
        '\u{00D0}' => Some("&ETH;"),
        '\u{00D1}' => Some("&Ntilde;"),
        '\u{00D2}' => Some("&Ograve;"),
        '\u{00D3}' => Some("&Oacute;"),
        '\u{00D4}' => Some("&Ocirc;"),
        '\u{00D5}' => Some("&Otilde;"),
        '\u{00D6}' => Some("&Ouml;"),
        '\u{00D7}' => Some("&times;"),
        '\u{00D8}' => Some("&Oslash;"),
        '\u{00D9}' => Some("&Ugrave;"),
        '\u{00DA}' => Some("&Uacute;"),
        '\u{00DB}' => Some("&Ucirc;"),
        '\u{00DC}' => Some("&Uuml;"),
        '\u{00DD}' => Some("&Yacute;"),
        '\u{00DE}' => Some("&THORN;"),
        '\u{00DF}' => Some("&szlig;"),
        '\u{00E0}' => Some("&agrave;"),
        '\u{00E1}' => Some("&aacute;"),
        '\u{00E2}' => Some("&acirc;"),
        '\u{00E3}' => Some("&atilde;"),
        '\u{00E4}' => Some("&auml;"),
        '\u{00E5}' => Some("&aring;"),
        '\u{00E6}' => Some("&aelig;"),
        '\u{00E7}' => Some("&ccedil;"),
        '\u{00E8}' => Some("&egrave;"),
        '\u{00E9}' => Some("&eacute;"),
        '\u{00EA}' => Some("&ecirc;"),
        '\u{00EB}' => Some("&euml;"),
        '\u{00EC}' => Some("&igrave;"),
        '\u{00ED}' => Some("&iacute;"),
        '\u{00EE}' => Some("&icirc;"),
        '\u{00EF}' => Some("&iuml;"),
        '\u{00F0}' => Some("&eth;"),
        '\u{00F1}' => Some("&ntilde;"),
        '\u{00F2}' => Some("&ograve;"),
        '\u{00F3}' => Some("&oacute;"),
        '\u{00F4}' => Some("&ocirc;"),
        '\u{00F5}' => Some("&otilde;"),
        '\u{00F6}' => Some("&ouml;"),
        '\u{00F7}' => Some("&divide;"),
        '\u{00F8}' => Some("&oslash;"),
        '\u{00F9}' => Some("&ugrave;"),
        '\u{00FA}' => Some("&uacute;"),
        '\u{00FB}' => Some("&ucirc;"),
        '\u{00FC}' => Some("&uuml;"),
        '\u{00FD}' => Some("&yacute;"),
        '\u{00FE}' => Some("&thorn;"),
        '\u{00FF}' => Some("&yuml;"),
        _ => None,
    }
}

/// Escape a string for safe inclusion in JavaScript single-quoted strings.
/// Matches PHP's jse() function - escapes all non-alphanumeric characters as \xHH.
pub fn js_escape(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c);
        } else {
            // Escape as \xHH for ASCII, or handle multi-byte chars
            // TODO: this does not properly handle multi-byte characters!
            for byte in c.to_string().as_bytes() {
                result.push_str(&format!("\\x{:02X}", byte));
            }
        }
    }
    result
}

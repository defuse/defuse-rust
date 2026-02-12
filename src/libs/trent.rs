//! TRENT - Trusted Random Entropy
//!
//! Database service for the trusted third party random number generator.
//! Uses a lazily-initialized connection pool that's only created when first accessed.
//!
//! Port of defuse.ca/src/pages/services/trustedthirdparty.php

use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::MySqlPool;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Global connection pool - lazily initialized on first use
static POOL: OnceLock<MySqlPool> = OnceLock::new();

/// A drawing record from the database
#[derive(Debug)]
pub struct Drawing {
    pub drawingnum: i32,
    pub complete: bool,
    pub passwordhash: String,
    pub starttime: u32,
    pub reviewtime: u32,
    pub printout: String,
    pub userprintout: String,
}

/// Connect to the database eagerly at startup to fail fast if misconfigured.
pub async fn ensure_db_connection_works() -> Result<(), sqlx::Error> {
    get_pool().await?;
    Ok(())
}

/// Get or create the database connection pool
async fn get_pool() -> Result<&'static MySqlPool, sqlx::Error> {
    if let Some(pool) = POOL.get() {
        return Ok(pool);
    }

    let url = std::env::var("TRENT_DATABASE_URL")
        .expect("TRENT_DATABASE_URL must be set for TRENT page");
    let pool = MySqlPool::connect(&url).await?;

    // Race-safe: if another task initialized first, use theirs and drop ours
    Ok(POOL.get_or_init(|| pool))
}

/// Get a drawing by its number
pub async fn get_drawing(drawing_num: i32) -> Result<Option<Drawing>, sqlx::Error> {
    let pool = get_pool().await?;

    let row: Option<(i32, i8, String, u32, u32, Vec<u8>, Vec<u8>)> = sqlx::query_as(
        "SELECT drawingnum, complete, passwordhash, starttime, reviewtime, printout, userprintout
         FROM drawings WHERE drawingnum = ?"
    )
    .bind(drawing_num)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(drawingnum, complete, passwordhash, starttime, reviewtime, printout, userprintout)| {
        Drawing {
            drawingnum,
            complete: complete == 1,
            passwordhash,
            starttime,
            reviewtime,
            printout: String::from_utf8_lossy(&printout).into_owned(),
            userprintout: String::from_utf8_lossy(&userprintout).into_owned(),
        }
    }))
}

/// Reserve a new drawing number
/// Returns (drawing_num, password) where password is 32 hex chars
pub async fn reserve_drawing(review_time: u32) -> Result<(i32, String), sqlx::Error> {
    let pool = get_pool().await?;

    let starttime = now();
    let password = generate_password();
    let passwordhash = hash_password(&password);

    let result = sqlx::query(
        "INSERT INTO drawings (complete, passwordhash, starttime, reviewtime, printout, userprintout)
         VALUES (0, ?, ?, ?, '', '')"
    )
    .bind(&passwordhash)
    .bind(starttime)
    .bind(review_time)
    .execute(pool)
    .await?;

    // Get the auto-increment ID from the insert result (same connection)
    Ok((result.last_insert_id() as i32, password))
}

/// Complete a drawing by storing the printout
pub async fn complete_drawing(
    drawing_num: i32,
    printout: &str,
    userprintout: &str,
) -> Result<(), sqlx::Error> {
    let pool = get_pool().await?;

    sqlx::query(
        "UPDATE drawings SET complete = 1, printout = ?, userprintout = ? WHERE drawingnum = ?"
    )
    .bind(printout)
    .bind(userprintout)
    .bind(drawing_num)
    .execute(pool)
    .await?;

    Ok(())
}

/// Generate a random password (32 hex chars from 16 random bytes)
/// Matches PHP: bin2hex(mcrypt_create_iv(16, MCRYPT_DEV_URANDOM))
fn generate_password() -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Hash a password with SHA256 (lowercase hex)
/// Matches PHP: hash("SHA256", $password)
pub fn hash_password(password: &str) -> String {
    let hash = Sha256::digest(password.as_bytes());
    hex::encode(hash)
}

/// Select a random number in [low, high] using 32 bytes of randomness
/// This is a streaming modular reduction that matches PHP's SelectRandomNumber exactly
pub fn select_random_number(random_bytes: &[u8; 32], low: i64, high: i64) -> i64 {
    let divisor = (high - low + 1).unsigned_abs();
    if divisor == 0 {
        return low;
    }

    let mut remainder: u64 = 0;
    for &byte in random_bytes {
        let total = remainder * 256 + byte as u64;
        remainder = total % divisor;
    }

    low + remainder as i64
}

/// Generate 32 random bytes for number selection
pub fn generate_random_bytes() -> [u8; 32] {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes
}

/// Get current unix timestamp
pub fn now() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32
}

/// Format a timestamp as PHP's date("D M j G:i:s T Y")
/// Example: "Mon Jan 15 14:30:45 UTC 2024"
pub fn format_date(timestamp: u32) -> String {
    use chrono::{TimeZone, Utc};
    let dt = Utc.timestamp_opt(timestamp as i64, 0).unwrap();
    // PHP: D = "Mon", M = "Jan", j = day without leading zero, G = 24h hour without leading zero
    // T = timezone abbreviation, Y = 4-digit year
    dt.format("%a %b %-d %-H:%M:%S UTC %Y").to_string()
}

/// Format bytes in human-readable form
/// Matches PHP's format_bytes function
pub fn format_bytes(size: u64) -> String {
    const UNITS: &[&str] = &[" B", " KB", " MB", " GB", " TB"];
    let mut size = size as f64;
    let mut i = 0;
    while size >= 1024.0 && i < 4 {
        size /= 1024.0;
        i += 1;
    }
    // Round to 2 decimal places, then trim trailing zeros like PHP does
    let rounded = (size * 100.0).round() / 100.0;
    let formatted = format!("{:.2}", rounded);
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    format!("{}{}", trimmed, UNITS[i])
}

/// Count lines in file content (matches PHP's fgets line counting)
pub fn count_lines(content: &[u8]) -> usize {
    if content.is_empty() {
        return 0;
    }
    // PHP's fgets reads lines including \n, counts each line
    // A file with "a\nb\n" has 2 lines, "a\nb" also has 2 lines
    content.iter().filter(|&&b| b == b'\n').count()
        + if content.last() != Some(&b'\n') { 1 } else { 0 }
}

/// Get a specific line from file content (0-indexed)
/// Returns the line including trailing newline if present
pub fn get_line(content: &[u8], line_idx: usize) -> Option<String> {
    let mut current_line = 0;
    let mut start = 0;

    for (i, &byte) in content.iter().enumerate() {
        if byte == b'\n' {
            if current_line == line_idx {
                // Include the newline in the result (matches PHP fgets)
                return Some(String::from_utf8_lossy(&content[start..=i]).into_owned());
            }
            current_line += 1;
            start = i + 1;
        }
    }

    // Handle last line without trailing newline
    if current_line == line_idx && start < content.len() {
        return Some(String::from_utf8_lossy(&content[start..]).into_owned());
    }

    None
}

/// Select random lines from file content
/// Returns Vec of (line_number, line_preview)
pub fn select_random_lines(
    content: &[u8],
    num_lines: usize,
    allow_repeat: bool,
) -> Vec<(usize, String)> {
    let total_lines = count_lines(content);
    if total_lines == 0 || num_lines == 0 {
        return Vec::new();
    }

    let mut results = Vec::with_capacity(num_lines);
    let mut excluded: Vec<usize> = Vec::new();

    for _ in 0..num_lines {
        // Keep selecting until we get a non-excluded line (or allow repeats)
        loop {
            let random_bytes = generate_random_bytes();
            let line_idx = select_random_number(&random_bytes, 0, total_lines as i64 - 1) as usize;

            if allow_repeat || !excluded.contains(&line_idx) {
                if !allow_repeat {
                    excluded.push(line_idx);
                }
                let line_text = get_line(content, line_idx).unwrap_or_default();
                results.push((line_idx, line_text));
                break;
            }
        }
    }

    results
}

/// Generate the random lines output for the printout
/// file_num should be 1, 2, or 3
pub fn get_random_lines_output(
    content: &[u8],
    num_lines: usize,
    no_line_repeat: bool,
    file_num: u8,
) -> String {
    let lines = select_random_lines(content, num_lines, !no_line_repeat);
    let mut output = String::new();

    for (i, (line_num, line_preview)) in lines.into_iter().enumerate() {
        output.push_str(&format!("FILE{} RANDOM LINE {}:\n", file_num, i + 1));
        output.push_str(&format!("RANDOM LINE NUMBER (FILE{}): {}\n", file_num, line_num));
        output.push_str(&format!("LINE PREVIEW: {}\n\n", line_preview));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_random_number_basic() {
        // Test with known bytes - all zeros should give low value
        let zeros = [0u8; 32];
        assert_eq!(select_random_number(&zeros, 0, 10), 0);
        assert_eq!(select_random_number(&zeros, 5, 15), 5);
    }

    #[test]
    fn test_count_lines() {
        assert_eq!(count_lines(b""), 0);
        assert_eq!(count_lines(b"a"), 1);
        assert_eq!(count_lines(b"a\n"), 1);
        assert_eq!(count_lines(b"a\nb"), 2);
        assert_eq!(count_lines(b"a\nb\n"), 2);
        assert_eq!(count_lines(b"a\nb\nc"), 3);
    }

    #[test]
    fn test_get_line() {
        let content = b"line0\nline1\nline2\n";
        assert_eq!(get_line(content, 0), Some("line0\n".to_string()));
        assert_eq!(get_line(content, 1), Some("line1\n".to_string()));
        assert_eq!(get_line(content, 2), Some("line2\n".to_string()));
        assert_eq!(get_line(content, 3), None);

        // Without trailing newline
        let content2 = b"a\nb";
        assert_eq!(get_line(content2, 0), Some("a\n".to_string()));
        assert_eq!(get_line(content2, 1), Some("b".to_string()));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(100), "100 B");
        assert_eq!(format_bytes(1024), "1 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1 MB");
    }

    #[test]
    fn test_format_date() {
        // 2024-01-15 14:30:45 UTC = 1705329045
        let formatted = format_date(1705329045u32);
        assert!(formatted.contains("Jan"));
        assert!(formatted.contains("15"));
        assert!(formatted.contains("UTC"));
        assert!(formatted.contains("2024"));
    }
}

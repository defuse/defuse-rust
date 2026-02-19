//! TRENT - Trusted Random Entropy
//!
//! Business logic for the trusted third party random number generator.
//! Uses a lazily-initialized connection pool that's only created when first accessed.
//!
//! Port of defuse.ca/src/pages/services/trustedthirdparty.php
//! 
//! TODO: This uses 32-bit integers for timestamps, needs to be updated before
//! 32-bit timestamps overflow!
//! 
//! TODO: I feel like this shouldn't just be a bunch of naked functions sitting
//! in this file but should be encapsulated into a trait.

use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::MySqlPool;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

// =============================================================================
// Public types
// =============================================================================

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

impl Drawing {
    /// The timestamp after which the drawing can be completed.
    pub fn draw_date(&self) -> u32 {
        self.starttime + self.reviewtime
    }
}

/// Result of reserving a new drawing number.
pub struct ReservationResult {
    pub drawing_num: i32,
    pub password: String,
    pub drawing_date: String,
}

/// Per-file slot in a drawing request: the uploaded file's content hash,
/// raw bytes (if available), and number of random lines to select from it.
#[derive(Clone)]
pub struct FileInput {
    pub hash: String,
    pub content: Option<Vec<u8>>,
    pub randlines: i32,
}

/// Parsed and validated parameters for creating or completing a drawing.
/// Built early in the page handler from form data, and used for
/// confirmation display and completion.
#[derive(Clone)]
pub struct DrawingParams {
    pub drawing_num: i32,
    pub passcode: String,
    pub name: String,
    pub description: String,
    pub files: [FileInput; 3],
    pub lowval: i32,
    pub highval: i32,
    pub numgen: i32,
    pub chosentwice: bool,
}

/// A drawing request that has been fully validated (params checked against
/// the database record and file contents verified). Can only be constructed
/// via `validate_create_request`.
pub struct ValidatedDrawing {
    pub params: DrawingParams,
}

/// Errors that can occur when validating a drawing creation request.
#[derive(Debug)]
pub enum CreateError {
    DrawingNotFound(i32),
    DatabaseError(String),
    TextTooLarge,
    NonLatin1Characters,
    IncorrectPassword(i32),
    AlreadyComplete(i32),
    ReviewPeriodNotComplete { drawing_num: i32, draw_date: String },
    InvalidRange,
    NegativeValues,
    TooManyNumbers,
    FileTooLarge,
    FileHashMismatch,
    FileNotLatin1,
    MissingFile,
    NotEnoughLines,
    RangeWithoutNumgen,
}

impl std::fmt::Display for CreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DrawingNotFound(n) => write!(f, "Drawing #{} does not exist.", n),
            Self::DatabaseError(msg) => write!(f, "{}", msg),
            Self::TextTooLarge => write!(f, "Name and description must each be less than 1 MB."),
            Self::NonLatin1Characters => write!(f,
                "Name and description can only contain Latin-1 characters (standard Western European letters, numbers, and symbols). \
                 Emojis, Chinese/Japanese/Korean characters, and other special Unicode characters are not supported."),
            Self::IncorrectPassword(n) => write!(f, "Incorrect password for drawing #{}.", n),
            Self::AlreadyComplete(n) => write!(f, "The random numbers for drawing #{} have already been chosen.", n),
            Self::ReviewPeriodNotComplete { drawing_num, draw_date } => write!(f,
                "The review period for drawing #{} is not complete. You will be able to do the drawing after {}",
                drawing_num, draw_date),
            Self::InvalidRange => write!(f, "The number range is invalid."),
            Self::NegativeValues => write!(f, "We couldn't possibly generate a NEGATIVE amount of random numbers..."),
            Self::TooManyNumbers => write!(f, "Sorry, we can only generate 1000 random numbers at a time."),
            Self::FileTooLarge => write!(f, "Sorry, maximum file size is 10MB."),
            Self::FileHashMismatch => write!(f, "File content does not match its claimed hash."),
            Self::FileNotLatin1 => write!(f, "Uploaded files can only contain Latin-1 characters."),
            Self::MissingFile => write!(f, "Please upload a file for each set of random lines requested."),
            Self::NotEnoughLines => write!(f, "One of the files doesn't have enough lines to be able to choose the requested number of lines."),
            Self::RangeWithoutNumgen => write!(f, "You set a range but didn't request any random numbers. Please set the amount of numbers to generate."),
        }
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Connect to the database eagerly at startup to fail fast if misconfigured.
pub async fn ensure_db_connection_works() -> Result<(), sqlx::Error> {
    get_pool().await?;
    Ok(())
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
            // Safe: input validation ensures all data is Latin-1 (Unicode U+0000-U+00FF),
            // which is always valid UTF-8 (1 or 2 byte sequences), so lossy is a no-op.
            printout: String::from_utf8_lossy(&printout).into_owned(),
            userprintout: String::from_utf8_lossy(&userprintout).into_owned(),
        }
    }))
}

/// Reserve a new drawing number.
/// The drawing date is computed from the actual start time stored in the database.
pub async fn reserve_drawing(review_time: u32) -> Result<ReservationResult, sqlx::Error> {
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

    Ok(ReservationResult {
        drawing_num: result.last_insert_id() as i32,
        password,
        drawing_date: format_date(starttime + review_time),
    })
}

/// Complete a drawing: build the printout and mark it as complete in the database.
/// Returns (printout, userprintout) on success for display to the user.
pub async fn complete_drawing(validated: &ValidatedDrawing) -> Result<(String, String), sqlx::Error> {
    let (printout, userprintout) = build_printout(validated);

    let pool = get_pool().await?;

    // SECURITY: "AND complete = 0" prevents a TOCTOU race where two concurrent
    // requests both pass validation and try to complete the same drawing.
    let result = sqlx::query(
        "UPDATE drawings SET complete = 1, printout = ?, userprintout = ? WHERE drawingnum = ? AND complete = 0"
    )
    .bind(&printout)
    .bind(&userprintout)
    .bind(validated.params.drawing_num)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }

    Ok((printout, userprintout))
}

/// Validate drawing creation parameters: fetch the drawing from the database,
/// check all fields against the record, and verify file contents.
/// Returns a `ValidatedDrawing` on success, which is required by `complete_drawing`.
pub async fn validate_create_request(params: DrawingParams) -> Result<ValidatedDrawing, CreateError> {
    let drawing = match get_drawing(params.drawing_num).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err(CreateError::DrawingNotFound(params.drawing_num)),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Err(CreateError::DatabaseError(
                format!("Database error encountered when looking up Drawing #{}.", params.drawing_num),
            ));
        }
    };

    // SECURITY: Only authorized drawing creator can roll the dice
    if hash_password(&params.passcode) != drawing.passwordhash {
        return Err(CreateError::IncorrectPassword(params.drawing_num));
    }

    // SECURITY: Don't allow re-rolls of the dice.
    if drawing.complete {
        return Err(CreateError::AlreadyComplete(params.drawing_num));
    }

    // SECURITY: Don't allow bypassing the review period
    if now() < drawing.draw_date() {
        return Err(CreateError::ReviewPeriodNotComplete {
            drawing_num: params.drawing_num,
            draw_date: format_date(drawing.draw_date()),
        });
    }

    if params.name.len() > MAX_TEXT_FIELD_SIZE || params.description.len() > MAX_TEXT_FIELD_SIZE {
        return Err(CreateError::TextTooLarge);
    }

    if !is_latin1_safe(&params.name) || !is_latin1_safe(&params.description) {
        return Err(CreateError::NonLatin1Characters);
    }

    if params.numgen == 0 && (params.lowval != 0 || params.highval != 0) {
        return Err(CreateError::RangeWithoutNumgen);
    }

    // The params.numgen != 0 conjunct is needed so that numgen=0, lowval=0, highval=0 case is allowed through.
    if params.lowval >= params.highval && params.numgen != 0 {
        return Err(CreateError::InvalidRange);
    }
    const MAX_RANGE_VAL: i32 = 1_000_000_000;
    if params.lowval < -MAX_RANGE_VAL || params.lowval > MAX_RANGE_VAL
        || params.highval < -MAX_RANGE_VAL || params.highval > MAX_RANGE_VAL
    {
        return Err(CreateError::InvalidRange);
    }
    if params.numgen < 0 || params.files.iter().any(|f| f.randlines < 0) {
        return Err(CreateError::NegativeValues);
    }
    if params.numgen > 1000 || params.files.iter().any(|f| f.randlines > 1000) {
        return Err(CreateError::TooManyNumbers);
    }

    validate_files(&params)?;

    Ok(ValidatedDrawing { params })
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
/// TODO: this can be moved into the page handler
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

/// Compute SHA-256 hash of data, returned as lowercase hex.
pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

/// Check if a string is a valid SHA256 hex hash (64 hex characters).
pub fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

// =============================================================================
// Private helpers
// =============================================================================

/// Global connection pool - lazily initialized on first use
static POOL: OnceLock<MySqlPool> = OnceLock::new();

/// Maximum file size: 10MB
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Maximum name/description size: 1MB each
const MAX_TEXT_FIELD_SIZE: usize = 1024 * 1024;

/// Validate file contents: sizes, Latin-1 safety, presence/randlines consistency,
/// and sufficient line counts when repeats are disabled.
// TODO: The UI states "Maximum line length is 1000 characters" but this is
// not enforced. Very long lines would end up in the LINE PREVIEW printout.
fn validate_files(params: &DrawingParams) -> Result<(), CreateError> {
    for file in &params.files {
        if let Some(content) = &file.content {
            if content.len() > MAX_FILE_SIZE {
                return Err(CreateError::FileTooLarge);
            }
            if sha256_hex(content) != file.hash {
                return Err(CreateError::FileHashMismatch);
            }
            if !is_file_latin1_safe(content) {
                return Err(CreateError::FileNotLatin1);
            }
            assert!(file.randlines >= 0); // checked in validate_create_request
            if file.randlines > 0 {
                let line_count = count_lines(content);
                if line_count == 0 || (!params.chosentwice && line_count < file.randlines as usize) {
                    return Err(CreateError::NotEnoughLines);
                }
            }
        } else if file.randlines > 0 {
            return Err(CreateError::MissingFile);
        }
    }
    Ok(())
}

/// Draws the random numbers and builds the userprintout and printout strings.
/// printout contains the drawing numbers
/// userprintout is the drawing description
/// Returns (printout, userprintout).
fn build_printout(validated: &ValidatedDrawing) -> (String, String) {
    let params = &validated.params;
    let userprintout = format!("NAME: {}\nDESCRIPTION:\n{}", params.name, params.description);

    let mut printout = String::new();
    printout.push_str(&format!("DRAWING NUMBER: {}\n", params.drawing_num));
    printout.push_str(&format!("DRAWING DATE: {}\n", format_date(now())));
    printout.push_str(&format!("AMOUNT OF NUMBERS: {}\n", params.numgen));
    printout.push_str(&format!("RANGE: {} TO {}\n", params.lowval, params.highval));
    printout.push_str(&format!("ALLOW REPEAT LINES: {}\n\n",
        if params.chosentwice { "Yes" } else { "No" }));

    if params.files.iter().any(|f| f.randlines > 0) {
        printout.push_str("NOTE: Line numbers start at 0. The first line is line 0.\n\n");
    }

    for (i, file) in params.files.iter().enumerate() {
        let file_num = i + 1;
        if file.content.is_some() {
            assert!(is_sha256_hex(&file.hash), "validated file {} has invalid hash", file_num);
            printout.push_str(&format!("FILE{} SHA256: {}\n\n", file_num, file.hash));

            if file.randlines > 0 {
                let content = file.content.as_ref().expect("validated file has no content");
                let lines = select_random_lines(content, file.randlines as usize, params.chosentwice);
                for (j, (line_num, line_preview)) in lines.into_iter().enumerate() {
                    printout.push_str(&format!("FILE{} RANDOM LINE {}:\n", file_num, j + 1));
                    printout.push_str(&format!("RANDOM LINE NUMBER (FILE{}): {}\n", file_num, line_num));
                    printout.push_str(&format!("LINE PREVIEW: {}\n\n", line_preview));
                }
            }
        }
    }

    for i in 1..=params.numgen {
        let randnum = select_random_number(params.lowval as i64, params.highval as i64);
        printout.push_str(&format!("RANDOM NUMBER, NUMBER {}: {}\n", i, randnum));
    }

    (printout, userprintout)
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

/// Generate a random password (32 hex chars from 16 random bytes)
/// Matches PHP: bin2hex(mcrypt_create_iv(16, MCRYPT_DEV_URANDOM))
fn generate_password() -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Hash a password with SHA256 (lowercase hex)
/// Matches PHP: hash("SHA256", $password)
fn hash_password(password: &str) -> String {
    // We don't need salt or a slow hashing function because passwords are 128-bit keys
    let hash = Sha256::digest(password.as_bytes());
    hex::encode(hash)
}

/// Select a random number in [low, high] using 32 bytes of OS randomness.
/// Matches PHP's SelectRandomNumber.
fn select_random_number(low: i64, high: i64) -> i64 {
    assert!(high >= low, "select_random_number: high ({}) < low ({})", high, low);
    let range = (high - low)
        .checked_add(1)
        .expect("select_random_number: range overflows i64") as u64;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    low + reduce_mod(&bytes, range) as i64
}

/// Streaming modular reduction: reduce 32 bytes of randomness modulo `divisor`.
fn reduce_mod(random_bytes: &[u8; 32], divisor: u64) -> u64 {
    assert!(divisor > 0, "reduce_mod: divisor must be > 0");
    let mut remainder: u64 = 0;
    for &byte in random_bytes {
        let total = remainder
            .checked_mul(256)
            .and_then(|v| v.checked_add(byte as u64))
            .expect("reduce_mod: overflow in modular reduction");
        remainder = total % divisor;
    }
    remainder
}

/// Count lines in file content (matches PHP's fgets line counting)
fn count_lines(content: &[u8]) -> usize {
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
fn get_line(content: &[u8], line_idx: usize) -> Option<String> {
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

/// Select random lines from file content.
/// If `allow_repeat` is true, the same line can be selected more than once.
/// Returns Vec of (line_number, line_text).
fn select_random_lines(
    content: &[u8],
    num_lines: usize,
    allow_repeat: bool,
) -> Vec<(usize, String)> {
    let total_lines = count_lines(content);
    if total_lines == 0 || num_lines == 0 {
        return Vec::new();
    }
    assert!(
        allow_repeat || num_lines <= total_lines,
        "cannot select {} unique lines from a file with only {} lines",
        num_lines, total_lines,
    );

    let mut results = Vec::with_capacity(num_lines);
    let mut excluded: Vec<usize> = Vec::new();

    // TODO: This is vulnerable to a CPU DoS attack where you upload a file with
    // lots of lines N and ask for N lines without replacement. It will take
    // something like N^2 loops. Replace this with an algorithm that actually
    // removes the drawn line when allow_repeat = false.
    for _ in 0..num_lines {
        loop {
            let line_idx = select_random_number(0, total_lines as i64 - 1) as usize;

            if allow_repeat || !excluded.contains(&line_idx) {
                if !allow_repeat {
                    excluded.push(line_idx);
                }
                let line_text = get_line(content, line_idx)
                    .expect("line index out of bounds despite count_lines check");
                results.push((line_idx, line_text));
                break;
            }
        }
    }

    results
}

/// Check if a string contains only Latin-1 compatible characters (code points 0-255).
fn is_latin1_safe(s: &str) -> bool {
    s.chars().all(|c| (c as u32) <= 255)
}

/// Check if file content is Latin-1 safe: must be valid UTF-8 with all
/// characters in the Latin-1 range (U+0000 to U+00FF).
fn is_file_latin1_safe(content: &[u8]) -> bool {
    match std::str::from_utf8(content) {
        Ok(s) => is_latin1_safe(s),
        Err(_) => false,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reduce_mod() {
        let zeros = [0u8; 32];
        assert_eq!(reduce_mod(&zeros, 11), 0);
        assert_eq!(reduce_mod(&zeros, 1), 0);
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

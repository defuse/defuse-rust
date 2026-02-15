//! Pastebin database service.
//!
//! Provides encrypted paste storage compatible with the PHP pastebin implementation.
//! 
//! This is code intended to be backwards-compatible with old PHP code.
//! Do not copy/paste this code in a new pastebin implementation, as there are
//! several things a new version should fix:
//!     - There is no authentication (which is fine for this use case, since the
//!       database and this code are running on the same server, without
//!       isolation.
//!     - It uses null-byte padding (which is fine for this use case since we 
//!       only officially support text inputs, not files.)
//! A modern pastebin implementation should use a library like libsodium.
//!
//! Database schema:
//! ```sql
//! TABLE pastes:
//!   token     CHAR(64) PRIMARY KEY  -- HMAC-SHA256 hex of URL key
//!   data      LONGTEXT              -- Base64(IV || ciphertext)
//!   time      INT                   -- Unix timestamp of expiration
//!   jscrypt   TINYINT(1)            -- 1 if client-side encrypted
//! ```

use sqlx::MySqlPool;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use super::pastebin_crypto::{decrypt, encrypt, get_database_id, CryptoError};
use super::passgen::generate_alphanumeric_password;

/// Global connection pool - lazily initialized on first use
static POOL: OnceLock<MySqlPool> = OnceLock::new();

/// Get or create the database connection pool
async fn get_pool() -> Result<&'static MySqlPool, PastebinError> {
    if let Some(pool) = POOL.get() {
        return Ok(pool);
    }

    let url = std::env::var("PASTEBIN_DATABASE_URL")
        .expect("PASTEBIN_DATABASE_URL must be set for pastebin");
    let pool = MySqlPool::connect(&url)
        .await
        .map_err(PastebinError::DatabaseError)?;

    // Race-safe: if another task initialized first, use theirs and drop ours
    Ok(POOL.get_or_init(|| pool))
}

/// Connect to the database eagerly at startup to fail fast if misconfigured.
/// This populates the OnceLock so subsequent calls to get_pool() are instant.
pub async fn ensure_db_connection_works() -> Result<(), PastebinError> {
    get_pool().await?;
    Ok(())
}

/// Default lifetime: 10 days in seconds
const DEFAULT_LIFETIME_SECS: i64 = 864000;

/// Standard URL key length (22 alphanumeric characters)
const STANDARD_KEY_LENGTH: usize = 22;

/// Short URL key length (8 alphanumeric characters)
const SHORT_KEY_LENGTH: usize = 8;

/// Maximum attempts to find a unique key before giving up
const MAX_KEY_ATTEMPTS: usize = 100;

/// Error type for pastebin operations
#[derive(Debug)]
pub enum PastebinError {
    NotFound,
    DatabaseError(sqlx::Error),
    CryptoError(CryptoError),
    KeyGenerationFailed,
}

impl std::fmt::Display for PastebinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PastebinError::NotFound => write!(f, "Paste not found"),
            PastebinError::DatabaseError(e) => write!(f, "Database error: {}", e),
            PastebinError::CryptoError(e) => write!(f, "Crypto error: {}", e),
            PastebinError::KeyGenerationFailed => write!(f, "Failed to generate unique key"),
        }
    }
}

impl std::error::Error for PastebinError {}

impl From<sqlx::Error> for PastebinError {
    fn from(e: sqlx::Error) -> Self {
        PastebinError::DatabaseError(e)
    }
}

impl From<CryptoError> for PastebinError {
    fn from(e: CryptoError) -> Self {
        PastebinError::CryptoError(e)
    }
}

/// Information about a paste
pub struct PasteInfo {
    /// The decrypted paste text (or ciphertext for jscrypt pastes)
    pub text: String,
    /// Time left in seconds before expiration
    pub timeleft: i64,
    /// Whether this is a client-side encrypted paste
    pub jscrypt: bool,
}

#[derive(Clone)]
pub struct PastebinService {
    pool: &'static MySqlPool,
}

impl PastebinService {
    /// Create a new PastebinService instance.
    /// The database connection pool is lazily initialized on first call and reused thereafter.
    pub async fn new() -> Result<Self, PastebinError> {
        Ok(Self { pool: get_pool().await? })
    }

    /// Create a new paste and return the URL key.
    ///
    /// # Arguments
    /// * `text` - The paste content (after line ending normalization)
    /// * `jscrypt` - Whether this is client-side encrypted
    /// * `lifetime_secs` - How long until expiration (None = default 10 days)
    /// * `short_url` - Use short 8-char URL instead of standard 22-char
    pub async fn create_paste(
        &self,
        text: &str,
        jscrypt: bool,
        lifetime_secs: Option<i64>,
        short_url: bool,
    ) -> Result<String, PastebinError> {
        // Delete expired pastes before creating new one
        self.delete_expired().await?;

        let lifetime = lifetime_secs.unwrap_or(DEFAULT_LIFETIME_SECS);
        let key_length = if short_url {
            SHORT_KEY_LENGTH
        } else {
            STANDARD_KEY_LENGTH
        };

        // Generate a unique URL key
        let mut url_key = String::new();
        for _ in 0..MAX_KEY_ATTEMPTS {
            let candidate = generate_alphanumeric_password(key_length);
            let token = get_database_id(&candidate);

            // Check if token already exists
            let exists: Option<(String,)> =
                sqlx::query_as("SELECT token FROM pastes WHERE token = ?")
                    .bind(&token)
                    .fetch_optional(self.pool)
                    .await?;

            if exists.is_none() {
                url_key = candidate;
                break;
            }
        }

        if url_key.is_empty() {
            return Err(PastebinError::KeyGenerationFailed);
        }

        // Calculate expiration time
        let expiration = Self::now() + lifetime;

        // Always server-side encrypt, even for jscrypt pastes (matching PHP behavior).
        // This protects jscrypt ciphertext from offline password cracking if the
        // database is compromised.
        let token = get_database_id(&url_key);
        let data = encrypt(&url_key, text);

        // Insert into database
        sqlx::query("INSERT INTO pastes (token, data, time, jscrypt) VALUES (?, ?, ?, ?)")
            .bind(&token)
            .bind(&data)
            .bind(expiration)
            .bind(if jscrypt { 1i8 } else { 0i8 })
            .execute(self.pool)
            .await?;

        Ok(url_key)
    }

    /// Get a paste by its URL key.
    ///
    /// For server-encrypted pastes, returns the decrypted text.
    /// For jscrypt pastes, returns the encrypted data (client decrypts with password).
    pub async fn get_paste(&self, url_key: &str) -> Result<PasteInfo, PastebinError> {
        // Delete expired pastes first
        self.delete_expired().await?;

        let token = get_database_id(url_key);

        // Fetch the paste
        let result: Option<(String, i64, i8)> =
            sqlx::query_as("SELECT data, time, jscrypt FROM pastes WHERE token = ?")
                .bind(&token)
                .fetch_optional(self.pool)
                .await?;

        match result {
            Some((data, expiration_time, jscrypt_flag)) => {
                let jscrypt = jscrypt_flag == 1;
                let timeleft = expiration_time - Self::now();

                // If already expired but not yet cleaned up, treat as not found
                if timeleft <= 0 {
                    return Err(PastebinError::NotFound);
                }

                // Always decrypt server-side encryption (applied to all pastes).
                // For jscrypt pastes, this returns the client-side ciphertext
                // which the browser then decrypts with the user's password.
                let text = decrypt(url_key, &data)?;

                Ok(PasteInfo {
                    text,
                    timeleft,
                    jscrypt,
                })
            }
            None => Err(PastebinError::NotFound),
        }
    }

    /// Delete all expired pastes
    pub async fn delete_expired(&self) -> Result<(), PastebinError> {
        let now = Self::now();
        sqlx::query("DELETE FROM pastes WHERE time < ?")
            .bind(now)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Delete a specific paste by URL key
    pub async fn delete_paste(&self, url_key: &str) -> Result<(), PastebinError> {
        let token = get_database_id(url_key);
        sqlx::query("DELETE FROM pastes WHERE token = ?")
            .bind(&token)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Get current unix timestamp
    fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }
}

/// Format time remaining in human-readable format
pub fn format_timeleft(seconds: i64) -> String {
    if seconds <= 0 {
        return "expired".to_string();
    }

    const MINUTE: i64 = 60;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;

    if seconds >= DAY {
        let days = seconds / DAY;
        let hours = (seconds % DAY) / HOUR;
        if hours > 0 {
            format!("{} days, {} hours", days, hours)
        } else {
            format!("{} days", days)
        }
    } else if seconds >= HOUR {
        let hours = seconds / HOUR;
        let minutes = (seconds % HOUR) / MINUTE;
        if minutes > 0 {
            format!("{} hours, {} minutes", hours, minutes)
        } else {
            format!("{} hours", hours)
        }
    } else if seconds >= MINUTE {
        let minutes = seconds / MINUTE;
        format!("{} minutes", minutes)
    } else {
        format!("{} seconds", seconds)
    }
}

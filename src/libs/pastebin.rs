//! Pastebin database service.
//!
//! Provides encrypted paste storage compatible with the PHP pastebin implementation.
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
use std::time::{SystemTime, UNIX_EPOCH};

use super::pastebin_crypto::{decrypt, encrypt, get_database_id, CryptoError};
use super::passgen::generate_alphanumeric_password;

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
    pool: MySqlPool,
}

impl PastebinService {
    /// Create a new PastebinService with the given database pool
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Connect to the database and create a new service
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = MySqlPool::connect(database_url).await?;
        Ok(Self::new(pool))
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
                    .fetch_optional(&self.pool)
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

        // Encrypt or store as-is for jscrypt
        let token = get_database_id(&url_key);
        let data = if jscrypt {
            // For jscrypt, store the client-encrypted data as-is
            text.to_string()
        } else {
            // Server-side encryption
            encrypt(&url_key, text)
        };

        // Insert into database
        sqlx::query("INSERT INTO pastes (token, data, time, jscrypt) VALUES (?, ?, ?, ?)")
            .bind(&token)
            .bind(&data)
            .bind(expiration)
            .bind(if jscrypt { 1i8 } else { 0i8 })
            .execute(&self.pool)
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
                .fetch_optional(&self.pool)
                .await?;

        match result {
            Some((data, expiration_time, jscrypt_flag)) => {
                let jscrypt = jscrypt_flag == 1;
                let timeleft = expiration_time - Self::now();

                // If already expired but not yet cleaned up, treat as not found
                if timeleft <= 0 {
                    return Err(PastebinError::NotFound);
                }

                let text = if jscrypt {
                    // Return as-is for client-side decryption
                    data
                } else {
                    // Decrypt server-side encrypted data
                    decrypt(url_key, &data)?
                };

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
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete a specific paste by URL key
    pub async fn delete_paste(&self, url_key: &str) -> Result<(), PastebinError> {
        let token = get_database_id(url_key);
        sqlx::query("DELETE FROM pastes WHERE token = ?")
            .bind(&token)
            .execute(&self.pool)
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

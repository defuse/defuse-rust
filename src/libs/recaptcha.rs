//! reCAPTCHA verification helper
//!
//! Verifies Google reCAPTCHA responses with support for a bypass header
//! used in automated testing.

use sha2::{Digest, Sha256};

/// Known SHA256 hash of the bypass key for automated testing
const BYPASS_HASH: &str = "082ce4a67e9ba423a366558c86a506a3cdc59664cf02a00cb3306957c2ae8534";

/// Verify a reCAPTCHA response
///
/// # Arguments
/// * `response` - The g-recaptcha-response value from the form
/// * `remote_ip` - The client's IP address
/// * `bypass_header` - Optional X-Captcha-Bypass header value for testing
///
/// # Returns
/// * `Ok(true)` - Verification passed
/// * `Ok(false)` - Verification failed
/// * `Err(...)` - Error during verification
pub async fn verify(
    response: Option<&str>,
    remote_ip: &str,
    bypass_header: Option<&str>,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    // Check for bypass header (for automated testing)
    if let Some(key) = bypass_header {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let hash = hex::encode(hasher.finalize());
        if hash == BYPASS_HASH {
            return Ok(true);
        }
    }

    // Get the reCAPTCHA response - if empty/missing, fail
    let response = match response {
        Some(r) if !r.is_empty() => r,
        _ => return Ok(false),
    };

    // Get the secret key from environment
    let secret = match std::env::var("RECAPTCHA_SECRET_KEY") {
        Ok(s) => s,
        Err(_) => {
            tracing::warn!("RECAPTCHA_SECRET_KEY not set, reCAPTCHA verification will fail");
            return Ok(false);
        }
    };

    // Make request to Google's verification API
    let client = reqwest::Client::new();
    let params = [
        ("secret", secret.as_str()),
        ("response", response),
        ("remoteip", remote_ip),
    ];

    let resp = client
        .post("https://www.google.com/recaptcha/api/siteverify")
        .form(&params)
        .send()
        .await?;

    // Parse the response
    #[derive(serde::Deserialize)]
    struct RecaptchaResponse {
        success: bool,
    }

    let result: RecaptchaResponse = resp.json().await?;
    Ok(result.success)
}

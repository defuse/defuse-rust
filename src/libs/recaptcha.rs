//! reCAPTCHA verification helper
//!
//! Verifies Google reCAPTCHA responses with support for a bypass header
//! used in automated testing.

use std::sync::OnceLock;
use std::time::Duration;

use sha2::{Digest, Sha256};

/// How long a single verification may take, end to end.
///
/// reqwest applies no timeout of its own, so without this a slow or blackholed
/// connection parks the handler forever while it holds a request slot. Ten seconds is
/// generous for an overloaded Google and still bounds the failure.
const VERIFY_TIMEOUT: Duration = Duration::from_secs(10);

/// How long the TCP + TLS handshake may take, inside the overall budget above.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Longest token worth sending to Google.
///
/// reCAPTCHA v2 tokens run roughly 500-2000 characters. The form field is unbounded
/// otherwise, so whatever arrives gets re-uploaded to Google -- the requester chooses
/// how many bytes the server sends. 8 KB is four times the top of the observed range.
const MAX_TOKEN_LEN: usize = 8 * 1024;

/// The ceiling must stay well clear of real token sizes, or a future format change
/// would silently start failing every captcha. v2 tokens top out around 2000 bytes, so
/// require at least four times that. Enforced at compile time rather than by a test,
/// because a value that is too small breaks the site rather than a test run.
const _: () = assert!(MAX_TOKEN_LEN >= 4 * 2000);

/// Most of Google's answer we will read. A siteverify response is a couple of hundred
/// bytes of JSON; reading it unbounded lets whatever answers for that host decide how
/// much gets buffered.
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// The shared HTTP client, built once.
///
/// A fresh `Client` per request meant a fresh connection pool and a fresh DNS
/// resolution every time, and `getaddrinfo` is a blocking-pool draw.
pub fn http_client() -> &'static reqwest::Client {
    static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(VERIFY_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .expect("failed to build the reCAPTCHA HTTP client")
    })
}

/// Whether a token is worth spending a network round trip on. An absent or oversized
/// token is a failed captcha, decided here rather than by Google.
fn is_plausible_token(token: &str) -> bool {
    !token.is_empty() && token.len() <= MAX_TOKEN_LEN
}

/// Known SHA256 hash of the bypass key for automated testing (preimage is random 256 bits so unsalted hashing is fine)
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

    // Get the secret key from environment - fail loudly if not configured
    // This check happens before the response check so that missing config
    // is caught even when the user doesn't submit a captcha response
    let secret = std::env::var("RECAPTCHA_SECRET_KEY")
        .expect("RECAPTCHA_SECRET_KEY must be set for reCAPTCHA verification");

    // Get the reCAPTCHA response - if absent or implausible, fail without asking Google
    let response = match response {
        Some(r) if is_plausible_token(r) => r,
        Some(r) => {
            tracing::warn!(
                "captcha token was implausible ({} bytes, limit {}); rejected without asking \
                 Google",
                r.len(),
                MAX_TOKEN_LEN,
            );
            return Ok(false);
        }
        None => return Ok(false),
    };

    // Make request to Google's verification API
    let client = http_client();
    let params = [
        ("secret", secret.as_str()),
        ("response", response),
        ("remoteip", remote_ip),
    ];

    let mut resp = client
        .post("https://www.google.com/recaptcha/api/siteverify")
        .form(&params)
        .send()
        .await?;

    // Parse the response
    #[derive(serde::Deserialize)]
    struct RecaptchaResponse {
        success: bool,
    }

    let body = read_bounded(&mut resp).await?;
    let result: RecaptchaResponse = serde_json::from_slice(&body)?;
    Ok(result.success)
}

/// Read at most `MAX_RESPONSE_BYTES` of a response body.
///
/// Streams rather than trusting `Content-Length`, which may be absent or wrong, and
/// errors instead of truncating -- a clipped body would parse as a failed captcha and
/// blame the visitor for an infrastructure problem.
async fn read_bounded(
    resp: &mut reqwest::Response,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut body = Vec::new();

    while let Some(chunk) = resp.chunk().await? {
        if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(format!(
                "reCAPTCHA response exceeded {MAX_RESPONSE_BYTES} bytes; refusing to buffer it"
            )
            .into());
        }
        body.extend_from_slice(&chunk);
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::{is_plausible_token, MAX_TOKEN_LEN};

    #[test]
    fn a_normal_token_is_worth_asking_google_about() {
        assert!(is_plausible_token(&"a".repeat(500)));
        assert!(is_plausible_token(&"a".repeat(2000)));
    }

    #[test]
    fn an_absent_token_is_rejected_without_a_round_trip() {
        assert!(!is_plausible_token(""));
    }

    #[test]
    fn an_oversized_token_is_rejected_without_a_round_trip() {
        assert!(!is_plausible_token(&"a".repeat(MAX_TOKEN_LEN + 1)));
        assert!(!is_plausible_token(&"a".repeat(1024 * 1024)));
    }

    #[test]
    fn a_token_at_the_ceiling_is_still_sent() {
        assert!(is_plausible_token(&"a".repeat(MAX_TOKEN_LEN)));
    }
}

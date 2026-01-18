//! Quantum Computer Time Capsule page handler.
//!
//! Allows users to submit encrypted messages that will only be readable
//! when large-scale quantum computers exist.

use askama::Template;
use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler, PostBody};
use crate::libs::{recaptcha, timecapsule};
use crate::libs::util::html_escape;

pub struct Handler;

/// Form submission result state
#[derive(Default)]
struct SubmissionResult {
    /// Success message with the encrypted text
    success_message: Option<String>,
    /// Error message to display
    error_message: Option<String>,
    /// Preserved textarea content on error
    textarea_contents: String,
}

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move {
            let (message_count, last_message_ago) = get_archive_stats().await;

            QuantumTimeCapsulePage {
                ctx,
                result: SubmissionResult::default(),
                message_count,
                last_message_ago,
            }
            .into_response()
        })
    }

    fn post(&self, ctx: PageContext, _state: &AppState, body: PostBody) -> Option<BoxFuture> {
        Some(Box::pin(async move {
            let result = match body {
                PostBody::UrlEncoded(bytes) => {
                    process_submission(&bytes, &ctx).await
                }
                PostBody::Multipart { .. } => {
                    // This page doesn't support multipart forms
                    SubmissionResult::default()
                }
            };

            let (message_count, last_message_ago) = get_archive_stats().await;

            QuantumTimeCapsulePage {
                ctx,
                result,
                message_count,
                last_message_ago,
            }
            .into_response()
        }))
    }
}

/// Get archive statistics (message count and time since last message)
async fn get_archive_stats() -> (String, String) {
    let message_count = match timecapsule::get_message_count().await {
        Ok(count) => count.to_string(),
        Err(_) => "ERROR".to_string(),
    };

    let last_message_ago = match timecapsule::get_last_timestamp().await {
        Ok(Some(timestamp)) => {
            let now = timecapsule::current_timestamp();
            let seconds_ago = now - timestamp;
            timecapsule::time_for_human(seconds_ago)
        }
        Ok(None) => "ERROR".to_string(),
        Err(_) => "ERROR".to_string(),
    };

    (message_count, last_message_ago)
}

/// Process the form submission
async fn process_submission(bytes: &[u8], ctx: &PageContext) -> SubmissionResult {
    let form: TimeCapsuleForm = serde_urlencoded::from_bytes(bytes).unwrap_or_default();

    // If no ciphertext, this isn't a message submission (maybe an upvote)
    if form.ciphertext.is_empty() {
        return SubmissionResult::default();
    }

    // Preserve the plaintext message for error recovery
    let textarea_contents = form.message.clone();

    // Build the encrypted message line
    // Format: time:ISO8601 algorithm:... presentpublickey:... futurepublickey:... ciphertext:...
    let now = chrono::Utc::now();
    let formatted_date = now.format("%Y-%m-%dT%H:%M:%S%:z").to_string();

    let encrypted_message = format!(
        "time:{} algorithm:{} presentpublickey:{} futurepublickey:{} ciphertext:{}",
        formatted_date,
        form.algorithm,
        form.present_public_key,
        form.future_public_key,
        form.ciphertext
    );

    // Validate message size and format
    if encrypted_message.len() >= 200000
        || encrypted_message.contains('\n')
        || encrypted_message.contains('\r')
    {
        return SubmissionResult {
            error_message: Some("Something went wrong, your message was too big or the encrypted version contains newlines.".to_string()),
            textarea_contents,
            ..Default::default()
        };
    }

    // Verify reCAPTCHA
    let bypass_header = ctx.captcha_bypass_header.as_deref();
    let recaptcha_response = if form.g_recaptcha_response.is_empty() {
        None
    } else {
        Some(form.g_recaptcha_response.as_str())
    };

    match recaptcha::verify(recaptcha_response, &ctx.client_ip, bypass_header).await {
        Ok(true) => {}
        Ok(false) | Err(_) => {
            return SubmissionResult {
                error_message: Some("Please solve the CAPTCHA.".to_string()),
                textarea_contents,
                ..Default::default()
            };
        }
    }

    // Save to database
    match timecapsule::add_entry(&encrypted_message).await {
        Ok(true) => {
            // Success! Return the encrypted message for display
            SubmissionResult {
                success_message: Some(encrypted_message),
                textarea_contents: String::new(), // Clear on success
                ..Default::default()
            }
        }
        Ok(false) | Err(_) => {
            SubmissionResult {
                error_message: Some("Sorry, there was an error adding the message to the database.".to_string()),
                textarea_contents,
                ..Default::default()
            }
        }
    }
}

#[derive(Template)]
#[template(path = "pages/services/quantum_computer_time_capsule.html")]
struct QuantumTimeCapsulePage {
    ctx: PageContext,
    result: SubmissionResult,
    message_count: String,
    last_message_ago: String,
}

impl QuantumTimeCapsulePage {
    /// Get HTML-escaped textarea contents
    fn textarea_contents_escaped(&self) -> String {
        html_escape(&self.result.textarea_contents)
    }

    /// Get HTML-escaped encrypted message for display
    fn encrypted_message_escaped(&self) -> String {
        match &self.result.success_message {
            Some(msg) => html_escape(msg),
            None => String::new(),
        }
    }
}

#[derive(Deserialize, Default)]
struct TimeCapsuleForm {
    #[serde(default)]
    message: String,
    #[serde(default)]
    algorithm: String,
    #[serde(default)]
    present_public_key: String,
    #[serde(default)]
    future_public_key: String,
    #[serde(default)]
    ciphertext: String,
    #[serde(default, rename = "g-recaptcha-response")]
    g_recaptcha_response: String,
}

// =============================================================================
// Archive Download Handler
// =============================================================================

/// Handle the archive download request.
/// Returns a text file with all messages and source code.
pub async fn download_archive() -> Response {
    // Generate the filename with current UTC timestamp
    let now = chrono::Utc::now();
    let filename = format!(
        "MessagesToTheQuantumComputingFuture-{}.txt",
        now.format("%Y-%m-%dT%H-%M-%S%:z")
    );

    // Build the archive content
    let content = match build_archive_content().await {
        Ok(content) => content,
        Err(e) => {
            tracing::error!("Failed to build archive: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate archive")
                .into_response();
        }
    };

    // Return with appropriate headers
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/plain"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        content,
    )
        .into_response()
}

/// Build the full archive content
async fn build_archive_content() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut content = String::new();

    // Header
    content.push_str("==== TIME CAPSULE FOR A QUANTUM-COMPUTING FUTURE ====\n\n");

    // Archive header text
    let header_text = tokio::fs::read_to_string("static/timecapsule/archive-header.txt").await?;
    content.push_str(&header_text);

    // Source code: timecapsule-save.js
    content.push_str("\n==== SOURCE CODE: timecapsule-save.js ====\n");
    let save_js = tokio::fs::read_to_string("static/timecapsule/timecapsule-save.js").await?;
    content.push_str(&save_js);

    // Source code: tweetnacl-time-capsule.js
    content.push_str("\n==== SOURCE CODE: tweetnacl-time-capsule.js ====\n");
    let nacl_js = tokio::fs::read_to_string("static/timecapsule/tweetnacl-time-capsule.js").await?;
    content.push_str(&nacl_js);

    // Source code: tweetnacl-util-time-capsule.js
    content.push_str("\n==== SOURCE CODE: tweetnacl-util-time-capsule.js ====\n");
    let util_js =
        tokio::fs::read_to_string("static/timecapsule/tweetnacl-util-time-capsule.js").await?;
    content.push_str(&util_js);

    // Messages
    content.push_str("\n==== MESSAGES FOR THE FUTURE ====\n\n");
    let messages = timecapsule::get_all_entries_in_order().await?;
    for msg in messages {
        content.push_str(&msg);
        content.push('\n');
    }

    // Footer
    content.push_str("\n==== ADDITIONAL INFORMATION ====\n\n");
    let footer_text = tokio::fs::read_to_string("static/timecapsule/archive-footer.txt").await?;
    content.push_str(&footer_text);

    Ok(content)
}

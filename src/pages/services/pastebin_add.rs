//! POST handler for creating pastes.
//!
//! Endpoint: POST /bin/add.php

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Form,
};
use serde::Deserialize;

use crate::app_state::AppState;

/// Default lifetime: 10 days in seconds
const DEFAULT_LIFETIME_SECS: i64 = 864000;

/// Maximum lifetime: 6 months in seconds
const MAX_LIFETIME_SECS: i64 = 15552000;

#[derive(Deserialize, Default)]
pub struct AddPasteForm {
    #[serde(default)]
    paste: String,
    #[serde(default)]
    jscrypt: Option<String>,
    #[serde(default)]
    lifetime: Option<String>,
    #[serde(default)]
    shorturl: Option<String>,
}

/// Handler for POST /bin/add.php
pub async fn handler(State(state): State<AppState>, Form(form): Form<AddPasteForm>) -> Response {
    // Check for empty paste
    if form.paste.is_empty() {
        return (StatusCode::OK, "Empty post!").into_response();
    }

    // Normalize line endings (CRLF -> LF, CR -> LF)
    let text = form.paste.replace("\r\n", "\n").replace('\r', "\n");

    // Parse form options
    let jscrypt = form.jscrypt.as_deref() == Some("yes");
    let short_url = form.shorturl.as_deref() == Some("yes");

    // Parse and validate lifetime
    let lifetime_secs = match form.lifetime.as_deref() {
        None | Some("") => DEFAULT_LIFETIME_SECS,
        Some(s) => match s.parse::<i64>() {
            Ok(lt) if lt > 0 && lt <= MAX_LIFETIME_SECS => lt,
            Ok(lt) if lt <= 0 => {
                return (StatusCode::BAD_REQUEST, "Invalid lifetime: must be positive").into_response();
            }
            Ok(_) => {
                return (StatusCode::BAD_REQUEST, "Invalid lifetime: exceeds maximum of 6 months").into_response();
            }
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "Invalid lifetime: not a valid number").into_response();
            }
        }
    };

    // Create the paste
    match state
        .pastebin
        .create_paste(&text, jscrypt, Some(lifetime_secs), short_url)
        .await
    {
        Ok(url_key) => {
            // Redirect to view page
            let location = format!("/b/{}", url_key);
            Response::builder()
                .status(StatusCode::FOUND)
                .header(header::LOCATION, location)
                .body(axum::body::Body::empty())
                .unwrap()
        }
        Err(e) => {
            tracing::error!("Failed to create paste: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create paste").into_response()
        }
    }
}

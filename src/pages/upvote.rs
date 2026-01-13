//! Upvote AJAX endpoint
//!
//! Handles vote submissions via POST and returns XML response.
//! Port of defuse.ca/src/libs/Upvote.php::process_ajax()

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Form,
};
use serde::Deserialize;

use crate::db::upvotes::VoteAction;
use crate::middleware::ClientIp;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct VoteForm {
    upvotes_id: String,
    upvotes_direction: String,
}

/// POST /upvote - Process a vote and return XML response
pub async fn post(
    State(state): State<AppState>,
    Extension(client_ip): Extension<ClientIp>,
    Form(form): Form<VoteForm>,
) -> Response {
    let client_ip = client_ip.0;

    // Parse direction
    let direction = match form.upvotes_direction.as_str() {
        "up" => VoteAction::Upvote,
        "down" => VoteAction::Downvote,
        _ => {
            return xml_response("fail", "N", "N", 0);
        }
    };

    // Process the vote
    let result = match state
        .upvotes
        .process_vote(&form.upvotes_id, &client_ip, direction)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Vote processing error: {}", e);
            return xml_response("fail", "N", "N", 0);
        }
    };

    // Determine arrow states based on user's current action
    let (uparrow, downarrow) = match result.user_action {
        Some(VoteAction::Upvote) => ("Y", "N"),
        Some(VoteAction::Downvote) => ("N", "Y"),
        None => ("N", "N"),
    };

    xml_response("pass", uparrow, downarrow, result.total())
}

/// Build XML response matching PHP format
fn xml_response(status: &str, uparrow: &str, downarrow: &str, total: i32) -> Response {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<response>
<status>{}</status>
<uparrow>{}</uparrow>
<downarrow>{}</downarrow>
<total>{}</total>
</response>
"#,
        html_escape(status),
        html_escape(uparrow),
        html_escape(downarrow),
        total
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

/// Simple HTML entity escaping for XML
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

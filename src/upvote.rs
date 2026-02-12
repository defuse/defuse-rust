//! Upvote AJAX endpoint
//!
//! Handles vote submissions via POST and returns XML response.
//! Port of defuse.ca/src/libs/Upvote.php::process_ajax()

use axum::{
    extract::{ConnectInfo, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Form,
};
use serde::Deserialize;
use std::net::SocketAddr;

use crate::app_state::AppState;
use crate::libs::{upvotes::VoteAction, util::{client_ip, html_escape}};

#[derive(Deserialize)]
pub struct VoteForm {
    upvotes_id: String,
    upvotes_direction: String,
}

/// POST /upvote - Process a vote and return XML response
pub async fn post(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<VoteForm>,
) -> Response {

    // AUDIT: encapsulate more of this into the upvote library

    let client_ip = client_ip(addr.ip(), &headers);

    // Process the vote
    let result = match state
        .upvotes
        .process_vote(&form.upvotes_id, &client_ip, &form.upvotes_direction)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Vote processing error: {}", e);
            return xml_response("fail", "N", "N", 0);
        }
    };

    // Determine arrow states based on user's current vote
    let (uparrow, downarrow) = match result.user_vote {
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

//! Special utility endpoints
//!
//! Simple PHP endpoints that don't use the normal page framework:
//! - /ip.php - Raw IP address
//! - /ip-insecure.php - IP address in HTML
//! - /getmyip.php - IP, hostname, and user-agent
//! - /s.php - "Shout" page (display large text)

use axum::{
    extract::{ConnectInfo, Query},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};
use base64::{engine::general_purpose::STANDARD_NO_PAD as BASE64, Engine};
use serde::Deserialize;
use std::net::SocketAddr;

use crate::libs::util::html_escape;

// =============================================================================
// /ip.php - Raw IP address
// =============================================================================

/// Returns the client's IP address as plain text
pub async fn ip_php(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    let ip = addr.ip().to_string();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain")],
        format!("{}\n", ip),
    )
}

// =============================================================================
// /ip-insecure.php - IP in HTML with styling
// =============================================================================

/// Returns the client's IP address in a styled HTML page
pub async fn ip_insecure_php(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> Html<String> {
    let ip = html_escape(&addr.ip().to_string());
    Html(format!(
        r#"<html>
<head>
    <title>IP</title>
    <link rel="stylesheet" media="all" type="text/css" href="/main.css" />
</head>
<body>
<div style="font-size: 30pt; text-align: center;">
HTTP IP:
{}
</div>
</body>
</html>
"#,
        ip
    ))
}

// =============================================================================
// /getmyip.php - IP, hostname, and user-agent
// =============================================================================

/// Returns IP address, hostname (reverse DNS), and user-agent
pub async fn getmyip_php(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Html<String> {
    let ip = addr.ip().to_string();

    // Reverse DNS lookup for hostname
    let hostname = match tokio::net::lookup_host(format!("{}:0", ip)).await {
        Ok(_) => {
            // Try to get the hostname via reverse lookup
            match dns_lookup::lookup_addr(&addr.ip()) {
                Ok(host) => host,
                Err(_) => ip.clone(),
            }
        }
        Err(_) => ip.clone(),
    };

    // Get user agent from headers
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    Html(format!(
        r#"<html>
<head>
<style>
body{{
	color: white;
	background-color:black;
}}
</style>
</head>
<body>
<center>
IP address: <br />{}
<br /><br />Hostname: <br />{}
<br /><br />{}
</center>
</body>
</html>
"#,
        html_escape(&ip),
        html_escape(&hostname),
        html_escape(user_agent)
    ))
}

// =============================================================================
// /s.php - "Shout" page
// =============================================================================

#[derive(Deserialize, Default)]
pub struct ShoutParams {
    /// Text to encode and redirect
    e: Option<String>,
    /// Base64-encoded text to display
    s: Option<String>,
}

/// "Shout" page - displays text in large font
///
/// - ?e=text -> redirect to ?s=base64(text)
/// - ?s=base64 -> display decoded text in 300pt font
/// - no params -> show input form
pub async fn shout_php(Query(params): Query<ShoutParams>) -> Response {
    // If ?e= param, encode and redirect (302 Found)
    if let Some(text) = params.e {
        let encoded = BASE64.encode(text.as_bytes());
        let redirect_url = format!("/s.php?s={}", urlencoding::encode(&encoded));
        return (
            StatusCode::FOUND,
            [(header::LOCATION, redirect_url)],
        ).into_response();
    }

    // If ?s= param, decode and display
    if let Some(encoded) = params.s {
        // Strip padding for compatibility (we encode without padding but accept both)
        let stripped = encoded.trim_end_matches('=');
        let decoded = match BASE64.decode(stripped) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(_) => String::new(),
        };

        return Html(format!(
            r#"<html>
<body>

    <div style="font-size: 300pt;">
    <b>{}</b>
    </div>
    </body>
</html>
"#,
            html_escape(&decoded)
        ))
        .into_response();
    }

    // No params - show form
    Html(
        r#"<html>
<body>

    <form action="s.php" method="get">
    <input type="text" name="e" value="Shout!" />
    <input type="submit" value="shout" />
    </form>
</body>
</html>
"#
        .to_string(),
    )
    .into_response()
}

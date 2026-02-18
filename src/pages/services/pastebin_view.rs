//! View handler for pastes.
//!
//! Endpoints:
//! - GET /b/{key} - View paste (HTML)
//! - GET /b/{key}?raw=true - Raw paste content (text/plain)
//! - GET /b/ - Redirect to /pastebin.htm
//! - GET /b - Redirect to /pastebin.htm
//!

use axum::{
    extract::{Path, Query},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::libs::pastebin::{format_timeleft, PastebinError, PastebinService};
use crate::libs::util::html_escape;

#[derive(Deserialize, Default)]
pub struct ViewQuery {
    #[serde(default)]
    raw: Option<String>,
    #[serde(default)]
    delete: Option<String>,
}

/// SHA256 hash of the delete secret (same as PHP)
const DELETE_SECRET_HASH: &str = "a7c61e0ed10927d12ed8fa6c080874b31d1b589e679f8abb33cde3cfa00ac954";

/// Handler for GET /b/{key}
pub async fn handler(
    Path(key): Path<String>,
    Query(query): Query<ViewQuery>,
    _headers: HeaderMap,
) -> Response {
    // We've removed the bin.defuse.ca/key -> defuse.ca/b/key redirect since
    // these URLs haven't been generated in years and the maximum post lifetime
    // is 6 months.

    // Handle empty key (redirect to pastebin main page)
    if key.is_empty() {
        return redirect_301("/pastebin.htm");
    }

    let pastebin = match PastebinService::new().await {
        Ok(svc) => svc,
        Err(e) => {
            tracing::error!("Failed to connect to pastebin database: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    // Check for delete request with secret
    if let Some(delete_secret) = &query.delete {
        let hash = sha256_hex(delete_secret);
        if hash == DELETE_SECRET_HASH {
            let _ = pastebin.delete_paste(&key).await;
        }
    }

    // Fetch the paste
    let paste_result = pastebin.get_paste(&key).await;

    // Check if raw mode requested
    let is_raw = query.raw.as_deref() == Some("true");

    if is_raw {
        return handle_raw_paste(paste_result).await;
    }

    // HTML view
    handle_html_view(paste_result).await
}

/// Handle raw paste request (text/plain)
async fn handle_raw_paste(paste_result: Result<crate::libs::pastebin::PasteInfo, PastebinError>) -> Response {
    match paste_result {
        Ok(paste) => {
            if paste.jscrypt {
                // Can't return raw for client-encrypted pastes
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                    "ERROR: This paste was encrypted with client-side encryption.",
                )
                    .into_response()
            } else {
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                    paste.text,
                )
                    .into_response()
            }
        }
        Err(_) => {
            (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Sorry, the paste you were looking for could not be found.",
            )
                .into_response()
        }
    }
}

/// Handle HTML view of paste
async fn handle_html_view(
    paste_result: Result<crate::libs::pastebin::PasteInfo, PastebinError>,
) -> Response {
    let html = match paste_result {
        Ok(paste) => render_paste_html(&paste),
        Err(_) => render_not_found_html(),
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache, must-revalidate")
        .header(header::EXPIRES, "Mon, 01 Jan 1990 00:00:00 GMT")
        .body(axum::body::Body::from(html))
        .unwrap()
}

/// Render the HTML for a paste view
fn render_paste_html(paste: &crate::libs::pastebin::PasteInfo) -> String {
    let timeleft_display = format_timeleft(paste.timeleft);

    let content_section = if paste.jscrypt {
        // For jscrypt pastes, show password prompt
        let escaped_ciphertext = js_string_escape(&paste.text);
        format!(r#"
<div id="passwordprompt">
    <b>Enter Password:</b>
    <input type="password" id="password" name="password" value="" /><input type="button" name="decrypt" value="Decrypt" onClick="decryptPaste();" />
    <noscript>
        <b>[ Please Enable JavaScript ]</b>
    </noscript>
</div>
<div id="tofill" class="codebox"></div>
<script type="text/javascript">
function decryptPaste(){{
    try {{
        var encrypted = "{}";
        var password = document.getElementById("password").value;
        var plaintext = encrypt.decrypt(password, encrypted);
        document.getElementById("passwordprompt").innerHTML = "";
        document.getElementById("paste").value = plaintext;
        var lines = plaintext.split("\n");
        var fancyLines = [];
        var i = 0;
        fancyLines.push("<ol>");
        for(i = 0; i < lines.length; i++)
        {{
            var bgColor = i % 2;
            var line = lines[i].replace("\n", "");
            line = line.replace("\r", "");
            fancyLines.push("<li><div class=\"div" + bgColor + "\">" + fxw.allhtmlsani(line) + "</div></li>");
        }}
        fancyLines.push("</ol>");
        var fill = document.getElementById("tofill");
        fill.style.display = "block";
        fill.innerHTML = fancyLines.join('');
    }} catch (e) {{
        if (e.constructor == sjcl.exception.corrupt) {{
            alert('Wrong password or corrupted/invalid ciphertext.');
        }} else {{
            alert(e);
        }}
    }}
}}
</script>"#, escaped_ciphertext)
    } else {
        // For server-encrypted pastes, display the content
        let lines: Vec<&str> = paste.text.split('\n').collect();
        let mut html_lines = String::from("<div class=\"codebox\"><ol>");
        for (i, line) in lines.iter().enumerate() {
            let class = if i % 2 == 0 { "div0" } else { "div1" };
            let escaped = html_escape(line);
            // Convert tabs to 4 nbsp
            let escaped = escaped.replace('\t', "&nbsp;&nbsp;&nbsp;&nbsp;");
            // Convert double spaces to double nbsp
            let escaped = escaped.replace("  ", "&nbsp;&nbsp;");
            html_lines.push_str(&format!("<li><div class=\"{}\">{}</div></li>", class, escaped));
        }
        html_lines.push_str("</ol></div>");
        html_lines
    };

    // Textarea content - only for non-jscrypt pastes
    let textarea_content = if paste.jscrypt {
        String::new()
    } else {
        html_escape(&paste.text)
    };

    format!(
        r#"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd">
<html xmlns="http://www.w3.org/1999/xhtml" >
<head>
    <title>Defuse Security's Encrypted Pastebin</title>
    <style type="text/css">
    body {{
        background-color: #F0F0F0;
        color:black;
        padding: 0;
        margin: 0;
        font-family: verdana, tahoma, arial, helvetica, sans-serif, "MS Sans Serif";
        font-size: 10pt;
    }}

    .codebox {{
        font-family:monospace;
        background-color: #F0F0F0;
        padding-left: 20px;
        clear:both;
    }}

    textarea {{
        width:100%;
        height: 200px;
        background-color: white;
        color:black;
        border:solid black 1px;
        font-family: monospace;
        resize: none;
    }}

    .div0 {{
        font-family: monospace;
        background-color: #F0F0F0;
        margin-right: 10px;
    }}
    .div1 {{
        background-color: #FFFFFF;
        font-family: monospace;
        margin-right: 10px;
    }}

    #timeleft {{
        font-weight: bold;
        padding-bottom: 10px;
    }}

    #header {{
        margin: 0;
        padding: 10px;
        font-size: 15pt;
        float: left;
    }}

    #header a {{
        color: black;
        text-decoration: none;
    }}

    #header a:visited {{
        color: black;
    }}

    #header a:hover {{
        text-decoration: underline;
    }}

    #timeleft {{
        padding: 10px;
        text-align: right;
        color: #404040;
    }}

    #pasteform {{
        padding-left: 10px;
        padding-right: 10px;
    }}

    #encinfo {{
        padding-left: 10px;
        padding-top: 5px;
        padding-bottom: 20px;
    }}

    #passwordprompt {{
        padding-left: 10px;
        font-weight: bold;
        margin-bottom: 10px;
        clear: both;
    }}

    h2 {{
        font-size: 15pt;
    }}

    #sorry {{
        clear: both;
        text-align: center;
    }}
    </style>
</head>
<body>
<script type="text/javascript" src="/js/sjcl.js"></script>
<script type="text/javascript" src="/js/encrypt.js"></script>
<script type="text/javascript" src="/js/defuse.js"></script>

<script type="text/javascript">
<!--
function encryptPaste()
{{
    var pass1 = document.getElementById("pass1").value;
    var pass2 = document.getElementById("pass2").value;
    if(pass1 == pass2 && pass1 != "")
    {{
        var plain = document.getElementById("paste").value;
        var ct = encrypt.encrypt(pass1, plain);
        document.getElementById("paste").value = ct;
        document.getElementById("jscrypt").value = "yes";
        document.pasteform.submit();
    }}
    else if(pass1 != pass2)
    {{
        alert("Passwords do not match.");
    }}
    else if(pass1 == "")
    {{
        alert("You must provide a password.");
    }}
}}
-->
</script>

<h1 id="header"><a href="https://defuse.ca/pastebin.htm">Defuse Security</a>'s Pastebin</h1>

<div id="timeleft">This post will be deleted in {}.</div>

{}

<form name="pasteform" id="pasteform" action="/bin/add.php" method="post">

<textarea id="paste" name="paste" spellcheck="false" rows="30" cols="80">{}</textarea>

<input id="jscrypt" type="hidden" name="jscrypt" value="no" />
<input style="width:300px;" type="submit" name="submitpaste" value="Post Without Password Encryption" />
<input type="checkbox" name="shorturl" value="yes" /> Use shorter URL.
 Expire in
 <select name="lifetime">
     <option value="15552000">6 Months</option>
     <option value="2592000">30 Days</option>
     <option value="864000" selected="selected">10 Days</option>
     <option value="86400">1 Day</option>
     <option value="3600">60 Minutes</option>
     <option value="600">10 Minutes</option>
 </select>
</form>

<div id="encinfo">
    Password:
    <input type="password" id="pass1" value="" size="8" /> &nbsp;
    Verify: <input type="password" id="pass2" value="" size="8" />
    <input type="button" value="Encrypt &amp; Post" onclick="encryptPaste()" />
    <noscript>
        <b>[ Please Enable JavaScript ]</b>
    </noscript>
</div>

<p style="padding: 20px;">
<strong>Important Note:</strong> This page contains user-submitted content. In
no way is Defuse Security responsible for its contents. If this page contains
illegal information please <a href="https://defuse.ca/contact.htm">report it to
us</a>.
</p>
</body>
</html>"#,
        timeleft_display,
        content_section,
        textarea_content
    )
}

/// Render the not found HTML
fn render_not_found_html() -> String {
    r#"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd">
<html xmlns="http://www.w3.org/1999/xhtml" >
<head>
    <title>Defuse Security's Encrypted Pastebin</title>
    <style type="text/css">
    body {
        background-color: #F0F0F0;
        color:black;
        padding: 0;
        margin: 0;
        font-family: verdana, tahoma, arial, helvetica, sans-serif, "MS Sans Serif";
        font-size: 10pt;
    }

    #header {
        margin: 0;
        padding: 10px;
        font-size: 15pt;
        float: left;
    }

    #header a {
        color: black;
        text-decoration: none;
    }

    #header a:visited {
        color: black;
    }

    #header a:hover {
        text-decoration: underline;
    }

    #sorry {
        clear: both;
        text-align: center;
    }
    </style>
</head>
<body>
<script type="text/javascript" src="/js/sjcl.js"></script>
<script type="text/javascript" src="/js/encrypt.js"></script>
<script type="text/javascript" src="/js/defuse.js"></script>

<h1 id="header"><a href="https://defuse.ca/pastebin.htm">Defuse Security</a>'s Pastebin</h1>

<div id="sorry">Sorry, the paste you were looking for could not be found.</div>

<p style="padding: 20px;">
<strong>Important Note:</strong> This page contains user-submitted content. In
no way is Defuse Security responsible for its contents. If this page contains
illegal information please <a href="https://defuse.ca/contact.htm">report it to
us</a>.
</p>
</body>
</html>"#.to_string()
}

/// Escape a string for use in JavaScript string literal
/// Matches PHP's js_string_escape function
fn js_string_escape(data: &str) -> String {
    let mut safe = String::with_capacity(data.len() * 4);
    for c in data.chars() {
        if c.is_ascii_alphanumeric() {
            safe.push(c);
        } else {
            // Escape as \xHH for each byte
            for b in c.to_string().bytes() {
                safe.push_str(&format!("\\x{:02X}", b));
            }
        }
    }
    safe
}

/// Create a 301 Moved Permanently redirect response
/// TODO: this is duplicate code, move it into a utility
fn redirect_301(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::MOVED_PERMANENTLY)
        .header(header::LOCATION, location)
        .body(axum::body::Body::empty())
        .unwrap()
}

/// Compute SHA256 hash of a string, returning lowercase hex
fn sha256_hex(input: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

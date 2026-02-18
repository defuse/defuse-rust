//! ETag Middleware for Static Files
//!
//! Adds ETag headers to responses that have Last-Modified headers (static files).
//! ETag format matches Apache: "<size_hex>-<mtime_hex>"
//!
//! Supports conditional requests: If a request includes an `If-None-Match` header
//! matching the computed ETag, returns 304 Not Modified with an empty body.
//!
//! SECURITY: Does NOT add ETag to responses with Cache-Control: no-store
//! to ensure sensitive pages like passgen remain uncacheable.

use axum::{
    body::Body,
    http::{header, Method, Request, Response, StatusCode},
};
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Tower layer for ETag headers
#[derive(Clone)]
pub struct EtagLayer;

impl<S> Layer<S> for EtagLayer {
    type Service = EtagMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        EtagMiddleware { inner }
    }
}

/// The actual middleware service
#[derive(Clone)]
pub struct EtagMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for EtagMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();

        // Capture If-None-Match from request before passing to inner service
        let if_none_match = req
            .headers()
            .get(header::IF_NONE_MATCH)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned());

        // Only handle conditional requests for GET and HEAD (per RFC 7232)
        let is_conditional_method =
            req.method() == Method::GET || req.method() == Method::HEAD;

        Box::pin(async move {
            let mut response = inner.call(req).await?;

            // Only add ETag if:
            // 1. Response has Last-Modified (indicates static file)
            // 2. Response does NOT have Cache-Control: no-store (security)
            // 3. Response doesn't already have an ETag
            let headers = response.headers();

            let has_last_modified = headers.contains_key(header::LAST_MODIFIED);
            let has_etag = headers.contains_key(header::ETAG);
            let has_no_store = headers
                .get(header::CACHE_CONTROL)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.contains("no-store"))
                .unwrap_or(false);

            if has_last_modified && !has_etag && !has_no_store {
                if let Some(etag) = compute_etag(response.headers()) {
                    response
                        .headers_mut()
                        .insert(header::ETAG, etag.parse().unwrap());

                    // Handle conditional request (If-None-Match)
                    if is_conditional_method {
                        if let Some(ref inm) = if_none_match {
                            if etag_matches(inm, &etag) {
                                *response.status_mut() = StatusCode::NOT_MODIFIED;
                                // Remove entity headers per RFC 7232, keep cache headers
                                response.headers_mut().remove(header::CONTENT_TYPE);
                                response.headers_mut().remove(header::CONTENT_LENGTH);
                                response.headers_mut().remove(header::CONTENT_ENCODING);
                                *response.body_mut() = Body::empty();
                            }
                        }
                    }
                }
            }

            Ok(response)
        })
    }
}

/// Check if an If-None-Match header value matches a computed ETag.
/// Handles multiple ETags (comma-separated) and wildcard (*).
fn etag_matches(if_none_match: &str, etag: &str) -> bool {
    let inm = if_none_match.trim();
    if inm == "*" {
        return true;
    }
    inm.split(',').any(|tag| tag.trim() == etag)
}

/// Compute ETag from Last-Modified and Content-Length headers.
/// Format matches Apache: "<size_hex>-<mtime_hex>"
fn compute_etag(headers: &axum::http::HeaderMap) -> Option<String> {
    // Get Last-Modified header and parse to timestamp
    let last_modified = headers.get(header::LAST_MODIFIED)?;
    let last_modified_str = last_modified.to_str().ok()?;

    // Parse HTTP date format: "Sun, 26 May 2019 16:59:52 GMT"
    let mtime = parse_http_date(last_modified_str)?;

    // Get Content-Length
    let content_length = headers
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    // Format like Apache: "<size_hex>-<mtime_hex>"
    Some(format!("\"{:x}-{:x}\"", content_length, mtime))
}

/// Parse HTTP date format to Unix timestamp.
/// Format: "Sun, 26 May 2019 16:59:52 GMT"
fn parse_http_date(date_str: &str) -> Option<u64> {
    let dt = chrono::DateTime::parse_from_rfc2822(date_str).ok()?;
    Some(dt.timestamp() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_date() {
        // Known date: May 26, 2019 16:59:52 UTC = 1558889992
        let ts = parse_http_date("Sun, 26 May 2019 16:59:52 GMT");
        assert_eq!(ts, Some(1558889992));
    }

    #[test]
    fn test_compute_etag_format() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(header::LAST_MODIFIED, "Sun, 26 May 2019 16:59:52 GMT".parse().unwrap());
        headers.insert(header::CONTENT_LENGTH, "9233".parse().unwrap());

        let etag = compute_etag(&headers);
        assert!(etag.is_some());
        let etag = etag.unwrap();
        // Should be in format "<hex>-<hex>" with quotes
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
        assert!(etag.contains('-'));
        // Size 9233 = 0x2411
        assert!(etag.contains("2411"));
    }

    #[test]
    fn test_etag_matches_exact() {
        assert!(etag_matches("\"2411-5c4c5758\"", "\"2411-5c4c5758\""));
    }

    #[test]
    fn test_etag_matches_wildcard() {
        assert!(etag_matches("*", "\"2411-5c4c5758\""));
    }

    #[test]
    fn test_etag_matches_multiple() {
        assert!(etag_matches(
            "\"aaa-111\", \"2411-5c4c5758\", \"bbb-222\"",
            "\"2411-5c4c5758\""
        ));
    }

    #[test]
    fn test_etag_no_match() {
        assert!(!etag_matches("\"wrong-etag\"", "\"2411-5c4c5758\""));
    }

    #[test]
    fn test_etag_no_match_multiple() {
        assert!(!etag_matches(
            "\"aaa-111\", \"bbb-222\"",
            "\"2411-5c4c5758\""
        ));
    }
}

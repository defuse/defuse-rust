//! ETag Middleware for Static Files
//!
//! Adds ETag headers to responses that have Last-Modified headers (static files).
//! ETag format matches Apache: "<size_hex>-<mtime_hex>"
//!
//! SECURITY: Does NOT add ETag to responses with Cache-Control: no-store
//! to ensure sensitive pages like passgen remain uncacheable.

use axum::{
    body::Body,
    http::{header, Request, Response},
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
                    response.headers_mut().insert(
                        header::ETAG,
                        etag.parse().unwrap(),
                    );
                }
            }

            Ok(response)
        })
    }
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
    // HTTP date format: "Day, DD Mon YYYY HH:MM:SS GMT"
    // Example: "Sun, 26 May 2019 16:59:52 GMT"
    let parts: Vec<&str> = date_str.split_whitespace().collect();
    if parts.len() != 6 {
        return None;
    }

    let day: u32 = parts[1].parse().ok()?;
    let month = match parts[2] {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let year: i32 = parts[3].parse().ok()?;

    let time_parts: Vec<&str> = parts[4].split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let hour: u32 = time_parts[0].parse().ok()?;
    let minute: u32 = time_parts[1].parse().ok()?;
    let second: u32 = time_parts[2].parse().ok()?;

    // Convert to Unix timestamp using a simple calculation
    // Days from 1970-01-01 to the given date
    let days = days_since_epoch(year, month, day)?;
    let seconds = days as u64 * 86400 + hour as u64 * 3600 + minute as u64 * 60 + second as u64;

    Some(seconds)
}

/// Calculate days since Unix epoch (1970-01-01)
fn days_since_epoch(year: i32, month: u32, day: u32) -> Option<i64> {
    // Simplified calculation - handles years 1970-2099 correctly
    if year < 1970 {
        return None;
    }

    let mut days: i64 = 0;

    // Add days for complete years
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    // Add days for complete months in current year
    let days_in_months = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        days += days_in_months[(m - 1) as usize] as i64;
        if m == 2 && is_leap_year(year) {
            days += 1;
        }
    }

    // Add days in current month
    days += (day - 1) as i64;

    Some(days)
}

/// Check if a year is a leap year
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_date() {
        // Known date: May 26, 2019 16:59:52 UTC
        let ts = parse_http_date("Sun, 26 May 2019 16:59:52 GMT");
        assert!(ts.is_some());
        // Approximate check (exact value depends on calculation)
        let ts = ts.unwrap();
        assert!(ts > 1558000000 && ts < 1560000000);
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2000)); // divisible by 400
        assert!(!is_leap_year(1900)); // divisible by 100 but not 400
        assert!(is_leap_year(2024)); // divisible by 4
        assert!(!is_leap_year(2023)); // not divisible by 4
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
}

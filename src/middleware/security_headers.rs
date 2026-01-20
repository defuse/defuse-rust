//! Security Headers Middleware
//!
//! Adds security-related HTTP headers to all responses:
//! - Content-Type: text/html; charset=utf-8 (explicit, not relying on defaults)
//! - X-Frame-Options: SAMEORIGIN
//! - Strict-Transport-Security (HSTS) - only over HTTPS, not for localhost
//! - Cache-Control: no-cache (for pages marked with no_cache in registry)
//!
//! Also adds Content-Disposition: attachment for files that should be downloaded:
//! - /source/ files (source code)
//! - /files/ (storage: disk images, archives, etc.)
//! - /mirrors/ (storage: mirrored content)
//! - /upload/ (storage: user uploads)
//! Note: /files2/ is intentionally excluded (should be viewable in browser).

use axum::{
    body::Body,
    http::{header, Request, Response},
};
use std::task::{Context, Poll};
use tower::{Layer, Service};

use super::url_canonicalization::is_accepted_host;
use crate::registry::{resolve_path, PathLookupResult};

/// Tower layer for security headers
#[derive(Clone)]
pub struct SecurityHeadersLayer;

impl<S> Layer<S> for SecurityHeadersLayer {
    type Service = SecurityHeadersMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityHeadersMiddleware { inner }
    }
}

/// The actual middleware service
#[derive(Clone)]
pub struct SecurityHeadersMiddleware<S> {
    inner: S,
}

/// Check if a page should have no-cache headers based on registry metadata
fn check_no_cache(path: &str) -> bool {
    match resolve_path(path) {
        PathLookupResult::Canonical(page) => page.no_cache,
        _ => false,
    }
}

impl<S> Service<Request<Body>> for SecurityHeadersMiddleware<S>
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

        // Check if this is an accepted host (localhost, etc.)
        // Use full host with port to match PHP behavior (e.g., "defuse:10443")
        let host = req
            .headers()
            .get(header::HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_string();

        let is_https = req
            .headers()
            .get("x-forwarded-proto")
            .and_then(|h| h.to_str().ok())
            .map(|p| p == "https")
            .unwrap_or(false);

        let is_accepted = is_accepted_host(&host);

        // Check if this page should not be cached (lookup from registry)
        // SECURITY: Some pages like passgen must not be cached
        let path = req.uri().path().to_string();
        let is_no_cache_page = check_no_cache(&path);

        // Check if this file should be forced to download
        // Includes /source/ and storage directories (except /files2/ which is viewable)
        let is_download_file = path.starts_with("/source/")
            || path.starts_with("/files/")
            || path.starts_with("/mirrors/")
            || path.starts_with("/upload/");

        Box::pin(async move {
            let mut response = inner.call(req).await?;
            let is_success = response.status().is_success();
            let headers = response.headers_mut();

            // Content-Type: only set for HTML pages, not static assets
            // Static file handlers set their own content types (CSS, JS, images)
            // Only override if not already set, or if it's a page (not static file)
            let existing_content_type = headers.get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            // If no content-type set, or it's text/html without charset, set explicitly
            if existing_content_type.is_empty()
                || existing_content_type == "text/html"
                || existing_content_type.starts_with("text/html;")
            {
                headers.insert(
                    header::CONTENT_TYPE,
                    "text/html; charset=utf-8".parse().unwrap(),
                );
            }

            // X-Frame-Options: SAMEORIGIN (always)
            headers.insert(
                header::X_FRAME_OPTIONS,
                "SAMEORIGIN".parse().unwrap(),
            );

            // X-Content-Type-Options: nosniff (prevents MIME-type sniffing)
            headers.insert(
                header::X_CONTENT_TYPE_OPTIONS,
                "nosniff".parse().unwrap(),
            );

            // Referrer-Policy: protects sensitive URLs (like paste keys) from leaking
            // - Same-origin: full URL (useful for navigation)
            // - Cross-origin: only origin (protects paths)
            // - HTTPS→HTTP: no referrer
            headers.insert(
                header::REFERRER_POLICY,
                "strict-origin-when-cross-origin".parse().unwrap(),
            );

            // HSTS: only over HTTPS and not for localhost/accepted hosts
            if is_https && !is_accepted {
                headers.insert(
                    header::STRICT_TRANSPORT_SECURITY,
                    "max-age=31536000; includeSubDomains; preload"
                        .parse()
                        .unwrap(),
                );
            }

            // Cache control for sensitive pages (marked with no_cache in registry)
            // SECURITY: Prevents browsers from caching sensitive content
            if is_no_cache_page {
                headers.insert(
                    header::EXPIRES,
                    "Mon, 01 Jan 1990 00:00:00 GMT".parse().unwrap(),
                );
                headers.insert(
                    header::CACHE_CONTROL,
                    "no-cache, no-store, must-revalidate".parse().unwrap(),
                );
                headers.insert(
                    header::PRAGMA,
                    "no-cache".parse().unwrap(),
                );
            }

            // Content-Disposition for download files
            // Forces browser to download rather than display inline
            // Only for successful responses - error pages should be displayed, not downloaded
            if is_download_file && is_success {
                headers.insert(
                    header::CONTENT_DISPOSITION,
                    "attachment".parse().unwrap(),
                );
            }

            Ok(response)
        })
    }
}

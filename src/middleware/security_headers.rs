//! Security Headers Middleware
//!
//! Adds security-related HTTP headers to all responses:
//! - X-Frame-Options: SAMEORIGIN
//! - Strict-Transport-Security (HSTS) - only over HTTPS, not for localhost
//!
//! Note: Content-Type is set by Askama template responses automatically.

use axum::{
    body::Body,
    http::{header, Request, Response},
};
use std::task::{Context, Poll};
use tower::{Layer, Service};

use super::url_canonicalization::ACCEPTED_HOSTS;

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

/// Check if a host is in the accepted hosts list
fn is_accepted_host(host: &str) -> bool {
    ACCEPTED_HOSTS.iter().any(|h| h.eq_ignore_ascii_case(host))
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

        Box::pin(async move {
            let mut response = inner.call(req).await?;
            let headers = response.headers_mut();

            // X-Frame-Options: SAMEORIGIN (always)
            headers.insert(
                header::X_FRAME_OPTIONS,
                "SAMEORIGIN".parse().unwrap(),
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

            Ok(response)
        })
    }
}

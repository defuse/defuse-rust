//! Client IP extraction middleware
//!
//! Extracts the client IP address early in the request lifecycle and stores
//! it in request extensions. This allows all later middleware and handlers
//! to access the IP without needing ConnectInfo access.

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::Request,
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;

/// Client IP address, stored in request extensions.
/// This is always present after client_ip_middleware runs.
#[derive(Debug, Clone)]
pub struct ClientIp(pub String);

/// Middleware that extracts client IP and stores in extensions.
///
/// Checks in order:
/// 1. X-Forwarded-For header (reverse proxy)
/// 2. X-Real-IP header (reverse proxy)
/// 3. ConnectInfo from Axum (direct connection)
///
/// The IP is always available - ConnectInfo is set up via into_make_service_with_connect_info.
pub async fn client_ip_middleware(mut request: Request<Body>, next: Next) -> Response {
    let headers = request.headers();

    // Check X-Forwarded-For first (for reverse proxy setups)
    let ip = if let Some(forwarded) = headers.get("x-forwarded-for") {
        forwarded
            .to_str()
            .ok()
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string())
    } else {
        None
    };

    // Check X-Real-IP
    let ip = ip.or_else(|| {
        headers
            .get("x-real-ip")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    });

    // Fall back to ConnectInfo (direct connection IP) - should always be present
    let ip = ip.or_else(|| {
        request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0.ip().to_string())
    });

    // IP should always be available - if not, it's a server configuration bug
    let client_ip = ClientIp(
        ip.expect("BUG: Could not determine client IP - ConnectInfo not available. Is into_make_service_with_connect_info set up?")
    );

    request.extensions_mut().insert(client_ip);
    next.run(request).await
}

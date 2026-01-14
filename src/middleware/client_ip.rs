//! Client IP extraction middleware
//!
//! Extracts the client IP address early in the request lifecycle and stores
//! it in request extensions. This allows all later middleware and handlers
//! to access the IP without needing ConnectInfo access.
//!
//! SECURITY: X-Forwarded-For and X-Real-IP headers are only trusted when the
//! connection comes from a whitelisted proxy IP. This prevents IP spoofing
//! from direct connections.

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::Request,
    middleware::Next,
    response::Response,
};
use std::net::{IpAddr, SocketAddr};

/// Trusted proxy IPs that are allowed to set X-Forwarded-For / X-Real-IP headers.
/// Only connections from these IPs will have forwarding headers trusted.
const TRUSTED_PROXIES: &[IpAddr] = &[
    IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
    IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
];

/// Client IP address, stored in request extensions.
/// This is always present after client_ip_middleware runs.
#[derive(Debug, Clone)]
pub struct ClientIp(pub String);

/// Middleware that extracts client IP and stores in extensions.
///
/// If connection is from a trusted proxy (localhost), checks:
/// 1. X-Forwarded-For header
/// 2. X-Real-IP header
///
/// Otherwise (or as fallback), uses the actual connection IP from ConnectInfo.
///
/// SECURITY: Forwarding headers are only trusted from TRUSTED_PROXIES to prevent
/// IP spoofing from direct connections.
pub async fn client_ip_middleware(mut request: Request<Body>, next: Next) -> Response {
    // Get the actual connection IP - should always be present
    let connect_info = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .expect("BUG: ConnectInfo not available. Is into_make_service_with_connect_info set up?");

    let connection_ip = connect_info.0.ip();

    // Only trust forwarding headers if connection is from a trusted proxy
    let client_ip = if TRUSTED_PROXIES.contains(&connection_ip) {
        let headers = request.headers();

        // Check X-Forwarded-For first (standard proxy header)
        let forwarded_ip = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string());

        // Check X-Real-IP as fallback
        let forwarded_ip = forwarded_ip.or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        });

        // Use forwarded IP if available, otherwise connection IP
        forwarded_ip.unwrap_or_else(|| connection_ip.to_string())
    } else {
        // Direct connection - use actual connection IP, ignore any headers
        connection_ip.to_string()
    };

    request.extensions_mut().insert(ClientIp(client_ip));
    next.run(request).await
}

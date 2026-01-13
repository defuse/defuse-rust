//! Shared utility functions

use axum::http::HeaderMap;

/// Extract client IP from headers (X-Forwarded-For, X-Real-IP, or fallback)
///
/// In production, requests come through a reverse proxy that sets X-Forwarded-For.
/// For local development without a proxy, falls back to 127.0.0.1.
pub fn extract_client_ip(headers: &HeaderMap) -> String {
    // Check X-Forwarded-For first (for reverse proxy setups)
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(s) = forwarded.to_str() {
            // Take the first IP if there are multiple
            return s.split(',').next().unwrap_or(s).trim().to_string();
        }
    }

    // Check X-Real-IP
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(s) = real_ip.to_str() {
            return s.to_string();
        }
    }

    // Fallback - in production the reverse proxy should always set headers
    // TODO: Could also try to get from connection info if available
    "127.0.0.1".to_string()
}

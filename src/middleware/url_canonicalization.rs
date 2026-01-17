//! URL Canonicalization Middleware
//!
//! Handles all URL normalization and redirects:
//! - Host canonicalization (redirect to master host)
//! - HTTPS enforcement (with localhost bypass)
//! - URL canonicalization (/page → /page.htm, .html → .htm)
//! - Case normalization (redirect to canonical case from registry)
//! - Alias resolution (redirects)
//!
//! CRITICAL: Configuration is HARDCODED to match PHP behavior exactly.
//! See docs/URL_ROUTING_REQUIREMENTS.md for full specification.

use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode},
};
use std::task::{Context, Poll};
use tower::{Layer, Service};

use crate::registry::lookup_page;

// =============================================================================
// HARDCODED CONFIGURATION - Matching PHP URLParse.php exactly
// =============================================================================

/// The canonical hostname - all other hosts redirect here
pub const MASTER_HOST: &str = "defuse.ca";

/// Hosts that bypass redirects (for local development)
/// These hosts skip host canonicalization AND HTTPS enforcement
/// Note: Must include port for non-standard ports (e.g., "localhost:3000")
pub const ACCEPTED_HOSTS: &[&str] = &[
    "localhost",
    "localhost:3000",
    "localhost:8080",
    "127.0.0.1",
    "127.0.0.1:3000",
    "127.0.0.1:8080",
    "192.168.1.102",
    "defuse.h.defuse.ca",
    "defuse",
    "defuse:10443",
];

/// Whether to enforce HTTPS (redirect HTTP → HTTPS)
pub const FORCE_HTTPS: bool = true;

// =============================================================================
// Middleware Implementation
// =============================================================================

/// Tower layer for URL canonicalization
#[derive(Clone)]
pub struct UrlCanonicalizationLayer;

impl<S> Layer<S> for UrlCanonicalizationLayer {
    type Service = UrlCanonicalizationMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        UrlCanonicalizationMiddleware { inner }
    }
}

/// The actual middleware service
#[derive(Clone)]
pub struct UrlCanonicalizationMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for UrlCanonicalizationMiddleware<S>
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
            // Extract request info
            let host = req
                .headers()
                .get(header::HOST)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("")
                .to_string();

            // Get host without port for MASTER_HOST comparison
            // (MASTER_HOST is "defuse.ca" without port)
            let host_without_port = host.split(':').next().unwrap_or("").to_lowercase();

            let is_https = req
                .headers()
                .get("x-forwarded-proto")
                .and_then(|h| h.to_str().ok())
                .map(|p| p == "https")
                .unwrap_or(false);

            let uri = req.uri().clone();
            let path = uri.path();
            let query = uri.query();

            // ACCEPTED_HOSTS comparison uses FULL host (with port) to match PHP behavior
            // e.g., "defuse:10443" must match exactly
            let is_accepted_host = is_accepted_host(&host);

            // Step 1: Host canonicalization
            // If not master host and not an accepted host, redirect to master
            if !is_accepted_host && host_without_port != MASTER_HOST && !host_without_port.is_empty() {
                // ANTICIPATE: Use HTTPS if FORCE_HTTPS is true or already on HTTPS
                let use_https = FORCE_HTTPS || is_https;
                let redirect_url = build_redirect_url(use_https, MASTER_HOST, path, query);
                return Ok(redirect_301(&redirect_url));
            }

            // Step 2: HTTPS enforcement (skip for accepted hosts)
            if FORCE_HTTPS && !is_https && !is_accepted_host {
                let redirect_url = build_redirect_url(true, &host_without_port, path, query);
                return Ok(redirect_301(&redirect_url));
            }

            // Step 3: URL canonicalization
            if let Some(redirect_url) = canonicalize_url(path, query) {
                return Ok(redirect_301(&redirect_url));
            }

            // No redirect needed, continue to inner service
            inner.call(req).await
        })
    }
}

/// Check if a host is in the accepted hosts list (localhost, dev hosts, etc.)
pub fn is_accepted_host(host: &str) -> bool {
    ACCEPTED_HOSTS.iter().any(|h| h.eq_ignore_ascii_case(host))
}

/// Canonicalize the URL path, returning a redirect URL if needed.
///
/// This handles:
/// - Alias resolution (e.g., /trent → /trustedthirdparty.htm)
/// - Extension canonicalization (/about → /about.htm)
/// - .html → .htm redirect
/// - Case normalization (/About.htm → /about.htm)
/// - Directory trailing slash (/audits → /audits/)
fn canonicalize_url(path: &str, query: Option<&str>) -> Option<String> {
    // Handle root path
    if path == "/" {
        return None; // Home page, no redirect needed
    }

    // Parse the path to extract the page name
    let path_without_leading_slash = path.strip_prefix('/').unwrap_or(path);
    let path_lower = path_without_leading_slash.to_lowercase();

    // Check for invalid patterns: /.htm or /foo/.htm (case-insensitive)
    if path_lower == ".htm" || path_lower == ".html" {
        return None; // Will 404
    }
    if path_lower.ends_with("/.htm") || path_lower.ends_with("/.html") {
        return None; // Will 404
    }

    // Detect and strip extension (case-insensitive matching)
    // .HTM, .Htm, .htm all treated as .htm extension
    let (name_part, had_htm, had_html) = if path_lower.ends_with(".htm") {
        // Strip last 4 chars regardless of case
        (&path_without_leading_slash[..path_without_leading_slash.len() - 4], true, false)
    } else if path_lower.ends_with(".html") {
        // Strip last 5 chars regardless of case
        (&path_without_leading_slash[..path_without_leading_slash.len() - 5], false, true)
    } else {
        (path_without_leading_slash, false, false)
    };

    // Check if it's a directory path (ends with /)
    let _is_directory_request = name_part.ends_with('/') || path.ends_with('/');
    let lookup_name = name_part.trim_end_matches('/');

    // Look up the page (case-insensitive)
    let page_info = lookup_page(lookup_name);

    // If not found without slash, try with slash for directory pages
    let page_info = page_info.or_else(|| {
        let with_slash = format!("{}/", lookup_name);
        lookup_page(&with_slash)
    });

    let page_info = page_info?;

    // Step 4: Handle aliases/redirects
    if let Some(redirect_target) = page_info.redirect {
        // Resolve alias to canonical URL (with .htm anticipation)
        let target_info = lookup_page(redirect_target)
            .expect("BUG: redirect target must exist in registry");
        let canonical = target_info.relative_url();
        return Some(append_query(&canonical, query));
    }

    // Get the canonical URL for this page
    let canonical = page_info.relative_url();

    let canonical_with_query = append_query(&canonical, query);

    // Redirect if:
    // - Had .html extension (always redirect to .htm)
    // - Missing .htm extension (non-directory)
    // - Missing trailing slash (directory)
    // - Case doesn't match canonical
    if had_html {
        // .html → .htm redirect
        return Some(canonical_with_query);
    }

    if !page_info.is_directory() {
        // Non-directory: must have .htm (lowercase)
        // If had .HTM or .Htm etc, redirect to lowercase .htm
        if !had_htm || !path.ends_with(".htm") {
            return Some(canonical_with_query);
        }
        // Check case of the page name part
        if path != canonical.as_str() {
            return Some(canonical_with_query);
        }
    } else {
        // Directory: must have trailing /
        if !path.ends_with('/') {
            return Some(canonical_with_query);
        }
        // Check case (for directory part)
        if path.to_lowercase() != canonical.to_lowercase() {
            return Some(canonical_with_query);
        }
    }

    // URL is already canonical
    None
}

/// Build a full redirect URL
fn build_redirect_url(https: bool, host: &str, path: &str, query: Option<&str>) -> String {
    let scheme = if https { "https" } else { "http" };
    if let Some(q) = query {
        format!("{}://{}{}?{}", scheme, host, path, q)
    } else {
        format!("{}://{}{}", scheme, host, path)
    }
}

/// Append query string to a path
fn append_query(path: &str, query: Option<&str>) -> String {
    if let Some(q) = query {
        format!("{}?{}", path, q)
    } else {
        path.to_string()
    }
}

/// Create a 301 redirect response
fn redirect_301(url: &str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::MOVED_PERMANENTLY)
        .header(header::LOCATION, url)
        .body(Body::empty())
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accepted_hosts() {
        assert!(is_accepted_host("localhost"));
        assert!(is_accepted_host("LOCALHOST"));
        assert!(is_accepted_host("127.0.0.1"));
        assert!(!is_accepted_host("defuse.ca"));
        assert!(!is_accepted_host("evil.com"));
    }

    #[test]
    fn test_canonicalize_adds_htm() {
        let result = canonicalize_url("/about", None);
        assert_eq!(result, Some("/about.htm".to_string()));
    }

    #[test]
    fn test_canonicalize_preserves_query() {
        let result = canonicalize_url("/about", Some("foo=bar"));
        assert_eq!(result, Some("/about.htm?foo=bar".to_string()));
    }

    #[test]
    fn test_canonicalize_home_page() {
        // /index redirects to /
        let result = canonicalize_url("/index", None);
        assert_eq!(result, Some("/".to_string()));
    }

    #[test]
    fn test_canonicalize_alias() {
        let result = canonicalize_url("/key", None);
        assert_eq!(result, Some("/contact.htm".to_string()));
    }

    #[test]
    fn test_canonicalize_html_to_htm() {
        let result = canonicalize_url("/about.html", None);
        assert_eq!(result, Some("/about.htm".to_string()));
    }

    #[test]
    fn test_no_redirect_when_canonical() {
        let result = canonicalize_url("/about.htm", None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_root_no_redirect() {
        let result = canonicalize_url("/", None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_case_canonicalization() {
        // Uppercase should redirect to lowercase (for pages with lowercase canonical name)
        let result = canonicalize_url("/About.htm", None);
        assert_eq!(result, Some("/about.htm".to_string()));
    }

    #[test]
    fn test_alias_chain_redirect() {
        // /key is an alias to contact, should redirect to /contact.htm
        let result = canonicalize_url("/KEY", None);
        assert_eq!(result, Some("/contact.htm".to_string()));
    }

    #[test]
    fn test_uppercase_htm_extension() {
        // .HTM should redirect to .htm (case normalization)
        let result = canonicalize_url("/about.HTM", None);
        assert_eq!(result, Some("/about.htm".to_string()));
    }

    #[test]
    fn test_mixed_case_htm_extension() {
        // .HtM should redirect to .htm
        let result = canonicalize_url("/about.HtM", None);
        assert_eq!(result, Some("/about.htm".to_string()));
    }

    #[test]
    fn test_uppercase_html_extension() {
        // .HTML should redirect to .htm
        let result = canonicalize_url("/about.HTML", None);
        assert_eq!(result, Some("/about.htm".to_string()));
    }

    #[test]
    fn test_accepted_host_with_port() {
        // Full host:port should match ACCEPTED_HOSTS entry
        assert!(is_accepted_host("defuse:10443"));
        assert!(is_accepted_host("DEFUSE:10443"));
        // Without port should match entries without port
        assert!(is_accepted_host("localhost"));
        // But "defuse" without port should also match "defuse" entry
        assert!(is_accepted_host("defuse"));
    }

    #[test]
    fn test_double_slash_no_redirect() {
        // Double slash should not redirect (will 404 naturally)
        let result = canonicalize_url("//about.htm", None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_triple_slash_no_redirect() {
        // Triple slash should not redirect (will 404 naturally)
        let result = canonicalize_url("///about.htm", None);
        assert_eq!(result, None);
    }
}

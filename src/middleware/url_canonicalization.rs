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

use crate::registry::{resolve_path, PathLookupResult};

/// The canonical hostname - all other hosts redirect here
pub const MASTER_HOST: &str = "defuse.ca";

/// Hosts that bypass redirects (for local development)
/// These hosts skip host canonicalization AND HTTPS enforcement
/// Note: Must include port for non-standard ports (e.g., "localhost:3000")
///
/// DO NOT add the real domain name (e.g. "defuse.ca") since that would cause
/// security_headers.rs to not add HSTS headers when it should.
/// TODO: Rename this and is_accepted_host() to be more clear that they are DEV hosts only
pub const ACCEPTED_HOSTS: &[&str] = &[
    "localhost",
    "localhost:3000",
    "localhost:8080",
    "localhost:8443",
    "127.0.0.1",
    "127.0.0.1:3000",
    "127.0.0.1:8080",
    "127.0.0.1:8443",
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
/// Uses resolve_path() as the single source of truth for URL canonicalization.
/// Returns Some(redirect_url) if path needs redirect, None if already canonical or 404.
fn canonicalize_url(path: &str, query: Option<&str>) -> Option<String> {
    // Check for blog slug redirect first (/blog/slug → /blog/slug.html)
    if let Some(redirect) = check_blog_slug_redirect(path) {
        return Some(append_query(&redirect, query));
    }

    match resolve_path(path) {
        PathLookupResult::Canonical(_) => None,
        PathLookupResult::Redirect { canonical_path, .. } => {
            Some(append_query(&canonical_path, query))
        }
        PathLookupResult::NotFound => None, // Let dispatcher handle 404
    }
}

/// Check if a blog URL without extension should redirect to .html version.
/// Returns Some(redirect_path) if /blog/slug should redirect to /blog/slug.html.
fn check_blog_slug_redirect(path: &str) -> Option<String> {
    // Only handle /blog/ paths
    if !path.starts_with("/blog/") {
        return None;
    }

    // Skip if already has an extension (contains a dot after /blog/)
    let after_blog = &path[6..]; // Skip "/blog/"
    if after_blog.contains('.') || after_blog.is_empty() {
        return None;
    }

    // Skip if path ends with / (directory listing)
    if path.ends_with('/') {
        return None;
    }

    // Check if the .html file exists
    let html_path = format!("static{}.html", path);
    if std::path::Path::new(&html_path).exists() {
        return Some(format!("{}.html", path));
    }

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

    #[test]
    fn test_directory_page_without_trailing_slash_redirects() {
        // Directory page without trailing slash should redirect to add it
        let result = canonicalize_url("/test-directory", None);
        assert_eq!(result, Some("/test-directory/".to_string()));
    }

    #[test]
    fn test_directory_page_with_trailing_slash_no_redirect() {
        // Directory page with trailing slash is canonical - no redirect
        let result = canonicalize_url("/test-directory/", None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_directory_page_htm_extension_not_valid() {
        // Directory pages should not be accessible via .htm - should 404
        let result = canonicalize_url("/test-directory.htm", None);
        assert_eq!(result, None); // No redirect, will 404
    }

    #[test]
    fn test_blog_slug_redirects_when_html_exists() {
        // Blog slug should redirect to .html if the file exists
        // This test relies on static/blog/archives.html existing
        let result = check_blog_slug_redirect("/blog/archives");
        assert_eq!(result, Some("/blog/archives.html".to_string()));
    }

    #[test]
    fn test_blog_slug_no_redirect_when_file_missing() {
        // Blog slug should NOT redirect if .html file doesn't exist
        let result = check_blog_slug_redirect("/blog/nonexistent-post-xyz123");
        assert_eq!(result, None);
    }

    #[test]
    fn test_blog_slug_no_redirect_with_extension() {
        // Blog URL with .html extension should not redirect (already has extension)
        let result = check_blog_slug_redirect("/blog/some-post.html");
        assert_eq!(result, None);
    }

    #[test]
    fn test_blog_slug_no_redirect_non_blog_path() {
        // Non-blog paths should not be affected
        let result = check_blog_slug_redirect("/about");
        assert_eq!(result, None);
    }

    #[test]
    fn test_blog_slug_no_redirect_trailing_slash() {
        // Blog directory path should not redirect
        let result = check_blog_slug_redirect("/blog/");
        assert_eq!(result, None);
    }
}

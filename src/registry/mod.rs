//! Page Registry - Single source of truth for all pages and their metadata.
//!
//! This is the Rust equivalent of PHP's $PAGE_INFO array in URLParse.php.
//! To add a new page:
//! 1. Create the handler in src/pages/{name}.rs with HANDLER static
//! 2. Add `pub mod {name};` to src/pages/mod.rs
//! 3. Add entry to PAGE_REGISTRY in registry/pages.rs with `handler: {name}`
//! 4. Create the template in templates/pages/{name}.html

mod pages;

pub use pages::PAGE_REGISTRY;

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::handler::PageHandler;

/// Set of valid upvote permanent IDs, built from PAGE_REGISTRY on first access.
static VALID_UPVOTE_IDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    PAGE_REGISTRY
        .values()
        .filter_map(|page| page.upvote.as_ref().map(|u| u.id))
        .collect()
});

/// Check if an upvote permanent_id is registered in the page registry.
pub fn is_valid_upvote_id(id: &str) -> bool {
    VALID_UPVOTE_IDS.contains(id)
}

/// Configuration for page upvoting. If a page has this, it will show vote arrows.
/// Database records for upvotes are added automatically if missing, so just
/// defining this on a page is sufficient to add it to the system.
#[derive(Debug, Clone)]
pub struct UpvoteConfig {
    /// Unique page ID for the upvote system (must be valid CSS class name)
    pub id: &'static str,
    /// Category for grouping pages (e.g., "defuse_pages", "audits")
    pub category: &'static str,
    /// Optional title override, if None, uses the page's title
    pub title: Option<&'static str>,
    /// Optional description override, if None, uses the page's description
    pub description: Option<&'static str>,
}

/// Information about a single page
///
/// NOTE: Every page must explicitly specify all fields including `upvote`.
/// This prevents accidentally omitting upvote config for pages that should have it.
pub struct PageInfo {
    /// The handler for this page (implements PageHandler trait).
    /// None for aliases (they redirect, don't render).
    pub handler: Option<&'static dyn PageHandler>,

    /// URL slug - the canonical name as it appears in URLs (e.g., "about", "BH2016")
    /// - Empty string "" = homepage (directory-style, canonical URL is "/")
    /// - Ends with "/" = directory (canonical URL is "/{slug}")
    /// - Otherwise = regular page (canonical URL is "/{slug}.htm")
    pub slug: &'static str,

    /// Page title (empty = use DEFAULT_TITLE)
    pub title: &'static str,

    /// Meta description (empty = use DEFAULT_META_DESCRIPTION)
    pub description: &'static str,

    /// Meta keywords (empty = use DEFAULT_META_KEYWORDS)
    pub keywords: &'static str,

    /// Legacy hit counter ID from PHP version.
    /// PHP used the actual include()'d filename as the hit count key.
    /// So, in this rust version, we need to provide the legacy file paths.
    ///
    /// This MUST match the ID used in the PHP PHPCount database.
    /// Format: "pages/{file}.php" or "pages/{file}.html"
    ///
    /// For new pages, manually set it equal to the slug.
    /// TODO: make this an Option and by default use the slug?
    pub legacy_hit_count_id: &'static str,

    /// Redirect target - if Some, this page is an alias
    /// Some("") means redirect to home page
    /// None means this is a real page, not an alias
    pub redirect: Option<&'static str>,

    /// Should this page have no-cache headers?
    /// SECURITY: Used for pages like passgen to prevent password caching
    pub no_cache: bool,

    /// Upvote configuration - if Some, page shows vote arrows
    pub upvote: Option<UpvoteConfig>,

    /// Optional banner HTML inserted before the page content (before the <h1>).
    /// Used for deprecation notices, etc.
    pub banner: Option<&'static str>,
}

// Manual Clone implementation - needed because of dyn trait object
impl Clone for PageInfo {
    fn clone(&self) -> Self {
        Self {
            handler: self.handler,
            slug: self.slug,
            title: self.title,
            description: self.description,
            keywords: self.keywords,
            legacy_hit_count_id: self.legacy_hit_count_id,
            redirect: self.redirect,
            no_cache: self.no_cache,
            upvote: self.upvote.clone(),
            banner: self.banner,
        }
    }
}

// Manual Debug implementation - dyn PageHandler doesn't implement Debug
impl std::fmt::Debug for PageInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PageInfo")
            .field("handler", &self.handler.map(|_| "<handler>"))
            .field("slug", &self.slug)
            .field("title", &self.title)
            .field("redirect", &self.redirect)
            .field("no_cache", &self.no_cache)
            .finish_non_exhaustive()
    }
}


/// Helper macro for defining alias pages (redirects)
macro_rules! alias {
    ($slug:expr => $target:expr) => {
        PageInfo {
            handler: None,
            slug: $slug,
            title: "",
            description: "",
            keywords: "",
            legacy_hit_count_id: "",
            redirect: Some($target),
            no_cache: false,
            upvote: None,
            banner: None,
        }
    };
}
pub(crate) use alias;

/// Helper macro for defining regular pages WITH a handler implementation.
/// Required: handler (first!), slug, title, description, keywords, legacy_hit_count_id, upvote
/// Optional: no_cache (defaults to false)
///
/// The handler field takes a module path (e.g., `about` or `research::my_page`) which expands to `&pages::about::Handler`.
macro_rules! page {
    (
        handler: $($handler:ident)::+,
        slug: $slug:expr,
        title: $title:expr,
        description: $description:expr,
        keywords: $keywords:expr,
        legacy_hit_count_id: $legacy_hit_count_id:expr,
        upvote: $upvote:expr $(,)?
    ) => {
        PageInfo {
            handler: Some(&crate::pages::$($handler)::+::Handler),
            slug: $slug,
            title: $title,
            description: $description,
            keywords: $keywords,
            legacy_hit_count_id: $legacy_hit_count_id,
            redirect: None,
            no_cache: false,
            upvote: $upvote,
            banner: None,
        }
    };
    (
        handler: $($handler:ident)::+,
        slug: $slug:expr,
        title: $title:expr,
        description: $description:expr,
        keywords: $keywords:expr,
        legacy_hit_count_id: $legacy_hit_count_id:expr,
        upvote: $upvote:expr,
        no_cache: $no_cache:expr $(,)?
    ) => {
        PageInfo {
            handler: Some(&crate::pages::$($handler)::+::Handler),
            slug: $slug,
            title: $title,
            description: $description,
            keywords: $keywords,
            legacy_hit_count_id: $legacy_hit_count_id,
            redirect: None,
            no_cache: $no_cache,
            upvote: $upvote,
            banner: None,
        }
    };
    (
        handler: $($handler:ident)::+,
        slug: $slug:expr,
        title: $title:expr,
        description: $description:expr,
        keywords: $keywords:expr,
        legacy_hit_count_id: $legacy_hit_count_id:expr,
        upvote: $upvote:expr,
        banner: $banner:expr $(,)?
    ) => {
        PageInfo {
            handler: Some(&crate::pages::$($handler)::+::Handler),
            slug: $slug,
            title: $title,
            description: $description,
            keywords: $keywords,
            legacy_hit_count_id: $legacy_hit_count_id,
            redirect: None,
            no_cache: false,
            upvote: $upvote,
            banner: $banner,
        }
    };
}
pub(crate) use page;

impl PageInfo {
    /// Is this a directory-style URL? (no .htm extension)
    /// Derived from slug: empty string or ends with "/"
    pub fn is_directory(&self) -> bool {
        self.slug.is_empty() || self.slug.ends_with('/')
    }

    /// Get the relative URL path for this page (for form actions, links, etc.)
    /// Returns paths like "/", "/about.htm", "/audits/"
    /// Includes the leading "/".
    pub fn relative_url(&self) -> String {
        if self.slug.is_empty() {
            "/".to_string()
        } else if self.is_directory() {
            format!("/{}/", self.slug.trim_end_matches('/'))
        } else {
            format!("/{}.htm", self.slug)
        }
    }

    /// Get the page ID for PHPCount hit tracking
    pub fn hit_counter_id(&self) -> &'static str {
        self.legacy_hit_count_id
    }

    /// Get title, falling back to default if empty
    pub fn title_or_default(&self) -> &'static str {
        if self.title.is_empty() { DEFAULT_TITLE } else { self.title }
    }

    /// Get description, falling back to default if empty
    pub fn description_or_default(&self) -> &'static str {
        if self.description.is_empty() { DEFAULT_META_DESCRIPTION } else { self.description }
    }

    /// Get keywords, falling back to default if empty
    pub fn keywords_or_default(&self) -> &'static str {
        if self.keywords.is_empty() { DEFAULT_META_KEYWORDS } else { self.keywords }
    }
}

// Default metadata values (matching PHP)
pub const DEFAULT_TITLE: &str = "Defuse Security Research and Development";
pub const DEFAULT_META_DESCRIPTION: &str = "Defuse Security. Home of PIE Bin, TRENT, and more...";
pub const DEFAULT_META_KEYWORDS: &str = "defuse security, encryption, privacy, programming, code, research";

/// Page info for the 404 Not Found page
pub static NOT_FOUND_PAGE_INFO: PageInfo = PageInfo {
    handler: None, // 404 handler is called directly by dispatcher, not via trait
    slug: "404",
    title: "Page Not Found - Defuse Security",
    description: "",
    keywords: "",
    legacy_hit_count_id: "",
    redirect: None,
    no_cache: false,
    upvote: None,
    banner: None,
};

/// Look up a page by name/slug (case-insensitive)
pub fn lookup_page(name: &str) -> Option<&'static PageInfo> {
    let lowercase = name.to_lowercase();
    PAGE_REGISTRY.get(lowercase.as_str()).map(|p| p as &'static PageInfo)
}

/// Result of resolving a URL path to a page
#[derive(Debug, Clone)]
pub enum PathLookupResult {
    /// Page found and URL is already canonical - serve it
    Canonical(&'static PageInfo),

    /// Page found but URL should redirect to canonical form
    Redirect {
        canonical_path: String,
    },

    /// Path is invalid or page not found - 404
    NotFound,
}

/// Resolve a URL path to a page, determining if a redirect is needed.
///
/// This is the single source of truth for URL → page resolution.
/// Returns:
/// - `Canonical` if the path matches the page's canonical URL exactly
/// - `Redirect` if the page exists but the URL should redirect (wrong case, extension, etc.)
/// - `NotFound` if the path is invalid or no page exists
pub fn resolve_path(path: &str) -> PathLookupResult {
    // Handle root path and empty string → home page
    if path == "/" || path.is_empty() {
        let page = lookup_page("").expect("home page must exist");
        let page = resolve_alias(page);
        let canonical = page.relative_url();
        return if path == canonical {
            PathLookupResult::Canonical(page)
        } else {
            PathLookupResult::Redirect { canonical_path: canonical }
        };
    }

    let path_without_slash = path.strip_prefix('/').unwrap_or(path);

    // If path ends with /, it's claiming to be a directory - look up directly
    if path_without_slash.ends_with('/') {
        return match lookup_page(path_without_slash) {
            Some(page) => {
                let page = resolve_alias(page);
                let canonical = page.relative_url();
                if path == canonical {
                    PathLookupResult::Canonical(page)
                } else {
                    PathLookupResult::Redirect { canonical_path: canonical }
                }
            }
            None => PathLookupResult::NotFound,
        };
    }

    // Detect and strip .htm or .html extension (case-insensitive)
    let path_lower = path_without_slash.to_lowercase();
    let (name, had_extension) = if path_lower.ends_with(".htm") {
        (&path_without_slash[..path_without_slash.len() - 4], true)
    } else if path_lower.ends_with(".html") {
        (&path_without_slash[..path_without_slash.len() - 5], true)
    } else {
        (path_without_slash, false)
    };

    // Reject invalid paths like "/.htm" (empty) or "/foo/.htm" (ends with /)
    if name.is_empty() || name.ends_with('/') {
        return PathLookupResult::NotFound;
    }

    // Look up the page, with fallback for directory pages
    let page = lookup_page(name).or_else(|| {
        // Try with trailing slash for directory pages, but only if no .htm/.html extension
        // e.g. we don't want audits.htm to serve the audits/ directory
        if had_extension {
            return None;
        }
        let with_slash = format!("{}/", name);
        lookup_page(&with_slash)
    });

    match page {
        Some(page) => {
            let page = resolve_alias(page);
            let canonical = page.relative_url();
            if path == canonical {
                PathLookupResult::Canonical(page)
            } else {
                PathLookupResult::Redirect { canonical_path: canonical }
            }
        }
        None => PathLookupResult::NotFound,
    }
}

/// Resolve alias chains to get the final target page
fn resolve_alias(page: &'static PageInfo) -> &'static PageInfo {
    if let Some(target) = page.redirect {
        let target_page = lookup_page(target).expect("BUG: redirect target must exist");
        resolve_alias(target_page) // Handle chains
    } else {
        page
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_home_page_exists() {
        assert!(lookup_page("").is_some());
    }

    #[test]
    fn test_case_insensitive_lookup() {
        assert!(lookup_page("about").is_some());
        assert!(lookup_page("About").is_some());
        assert!(lookup_page("ABOUT").is_some());
    }

    #[test]
    fn test_alias_detection() {
        let key = lookup_page("key").unwrap();
        assert_eq!(key.redirect, Some("contact"));
    }

    // ==================== resolve_path tests ====================

    #[test]
    fn test_resolve_path_canonical() {
        // Canonical URLs should return Canonical
        assert!(matches!(resolve_path("/"), PathLookupResult::Canonical(_)));
        assert!(matches!(resolve_path("/about.htm"), PathLookupResult::Canonical(_)));
        assert!(matches!(resolve_path("/test-directory/"), PathLookupResult::Canonical(_)));
    }

    #[test]
    fn test_resolve_path_redirects_missing_extension() {
        // /about should redirect to /about.htm
        match resolve_path("/about") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/about.htm");
            }
            other => panic!("Expected Redirect, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_path_redirects_html_to_htm() {
        // /about.html should redirect to /about.htm
        match resolve_path("/about.html") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/about.htm");
            }
            other => panic!("Expected Redirect, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_path_redirects_wrong_case() {
        // /About.HTM should redirect to /about.htm
        match resolve_path("/About.HTM") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/about.htm");
            }
            other => panic!("Expected Redirect, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_path_redirects_alias() {
        // /key should redirect to /contact.htm (key is alias for contact)
        match resolve_path("/key") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/contact.htm");
            }
            other => panic!("Expected Redirect, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_path_redirects_alias_with_extension() {
        // Aliases should redirect even when accessed with .htm extension
        // /pphos.htm should redirect to /password-policy-hall-of-shame.htm
        match resolve_path("/pphos.htm") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/password-policy-hall-of-shame.htm");
            }
            other => panic!("Expected Redirect for /pphos.htm, got {:?}", other),
        }

        // /pphos (without extension) should also redirect
        match resolve_path("/pphos") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/password-policy-hall-of-shame.htm");
            }
            other => panic!("Expected Redirect for /pphos, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_path_redirects_directory_missing_slash() {
        // /test-directory should redirect to /test-directory/
        match resolve_path("/test-directory") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/test-directory/");
            }
            other => panic!("Expected Redirect, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_path_not_found() {
        // Unknown pages
        assert!(matches!(resolve_path("/nonexistent"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/nonexistent.htm"), PathLookupResult::NotFound));

        // Invalid extension-only paths that could trick into finding home page
        assert!(matches!(resolve_path("/.htm"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/.html"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path(".htm"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path(".html"), PathLookupResult::NotFound));

        // Invalid paths with extension after directory slash
        assert!(matches!(resolve_path("/test-directory/.htm"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/test-directory/.html"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/about/.htm"), PathLookupResult::NotFound));

        // Directory pages are NOT accessible via .htm extension
        assert!(matches!(resolve_path("/test-directory.htm"), PathLookupResult::NotFound));
    }

    #[test]
    fn test_resolve_path_double_slashes() {
        // Double slashes should NOT match pages (strips one slash, leaves "/about")
        assert!(matches!(resolve_path("//about.htm"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("//about"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("//"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("///about.htm"), PathLookupResult::NotFound));
    }

    #[test]
    fn test_resolve_path_index_alias() {
        // "index" is an alias for home page
        match resolve_path("/index") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/");
            }
            other => panic!("Expected Redirect for /index, got {:?}", other),
        }

        // /index.htm should also redirect to /
        match resolve_path("/index.htm") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/");
            }
            other => panic!("Expected Redirect for /index.htm, got {:?}", other),
        }

        // /INDEX (wrong case) should redirect to /
        match resolve_path("/INDEX") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/");
            }
            other => panic!("Expected Redirect for /INDEX, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_path_alias_with_html_extension() {
        // Alias with .html extension should redirect
        match resolve_path("/pphos.html") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/password-policy-hall-of-shame.htm");
            }
            other => panic!("Expected Redirect for /pphos.html, got {:?}", other),
        }

        match resolve_path("/key.html") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/contact.htm");
            }
            other => panic!("Expected Redirect for /key.html, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_path_alias_wrong_case() {
        // Alias with wrong case should redirect
        match resolve_path("/PPHOS") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/password-policy-hall-of-shame.htm");
            }
            other => panic!("Expected Redirect for /PPHOS, got {:?}", other),
        }

        match resolve_path("/KEY.HTM") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/contact.htm");
            }
            other => panic!("Expected Redirect for /KEY.HTM, got {:?}", other),
        }

        match resolve_path("/Key.Html") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/contact.htm");
            }
            other => panic!("Expected Redirect for /Key.Html, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_path_directory_wrong_case() {
        // Directory page with wrong case should redirect
        match resolve_path("/TEST-DIRECTORY/") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/test-directory/");
            }
            other => panic!("Expected Redirect for /TEST-DIRECTORY/, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_path_path_traversal() {
        // Path traversal attempts should 404 (no pages with these names)
        assert!(matches!(resolve_path("/../about"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/about/../contact"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/./about"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/.."), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/."), PathLookupResult::NotFound));
    }

    #[test]
    fn test_resolve_path_double_extensions() {
        // Double extensions should 404 (no page named "about.htm")
        assert!(matches!(resolve_path("/about.htm.htm"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/about.html.htm"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/about.htm.html"), PathLookupResult::NotFound));
    }

    #[test]
    fn test_resolve_path_wrong_extensions() {
        // Similar but wrong extensions should 404
        assert!(matches!(resolve_path("/about.htmx"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/about.ht"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/about.htm1"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/about.htmlx"), PathLookupResult::NotFound));
    }

    #[test]
    fn test_resolve_path_trailing_slash_on_non_directory() {
        // Adding trailing slash to a non-directory page should 404
        // (there's no "about/" page, only "about")
        assert!(matches!(resolve_path("/about/"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/contact/"), PathLookupResult::NotFound));
    }

    #[test]
    fn test_resolve_path_nested_paths() {
        // Nested paths should 404 (no nested pages in registry)
        assert!(matches!(resolve_path("/about/foo"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/about/foo.htm"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/foo/bar/baz"), PathLookupResult::NotFound));
    }

    #[test]
    fn test_resolve_path_index_html_alias_edge_case() {
        // "index.html" is itself an alias, so /index.html.htm strips .htm,
        // looks up "index.html" which is an alias to home → redirects to /
        match resolve_path("/index.html.htm") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/");
            }
            other => panic!("Expected Redirect for /index.html.htm, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_path_empty_string() {
        // Empty string should return home page (canonical is "/")
        match resolve_path("") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/");
            }
            other => panic!("Expected Redirect for empty string to /, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_path_without_leading_slash() {
        // Paths without leading slash should still redirect to canonical
        // (HTTP paths should always start with "/", but handle gracefully)
        match resolve_path("about.htm") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/about.htm");
            }
            other => panic!("Expected Redirect for about.htm, got {:?}", other),
        }

        match resolve_path("about") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/about.htm");
            }
            other => panic!("Expected Redirect for about, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_path_mixed_case_extensions() {
        // Various mixed case extension combinations
        match resolve_path("/about.HtM") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/about.htm");
            }
            other => panic!("Expected Redirect for /about.HtM, got {:?}", other),
        }

        match resolve_path("/about.hTmL") {
            PathLookupResult::Redirect { canonical_path } => {
                assert_eq!(canonical_path, "/about.htm");
            }
            other => panic!("Expected Redirect for /about.hTmL, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_path_whitespace_in_path() {
        // Paths with whitespace should 404 (no pages have spaces)
        assert!(matches!(resolve_path("/about "), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/ about"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/about .htm"), PathLookupResult::NotFound));
    }

    #[test]
    fn test_resolve_path_special_characters() {
        // Various special characters that should 404
        assert!(matches!(resolve_path("/about?foo"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/about#section"), PathLookupResult::NotFound));
        assert!(matches!(resolve_path("/about%20page"), PathLookupResult::NotFound));
    }
}

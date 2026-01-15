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

use crate::handler::PageHandler;

/// Configuration for page upvoting
/// If a page has this, it will show vote arrows
#[derive(Debug, Clone)]
pub struct UpvoteConfig {
    /// Unique page ID for the upvote system (must be valid CSS class name)
    pub id: &'static str,
    /// Category for grouping pages (e.g., "defuse_pages", "audits")
    pub category: &'static str,
    /// Optional title override (if None, uses page title)
    pub title: Option<&'static str>,
    /// Optional description override (if None, uses page description)
    pub description: Option<&'static str>,
}

// NOTE: No constructors for UpvoteConfig - use struct literal syntax for explicitness

/// Information about a single page
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

    /// Legacy hit counter ID from PHP version - preserves existing hit counts
    /// This MUST match the ID used in the PHP PHPCount database.
    /// Format: "pages/{file}.php" or "pages/{file}.html"
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
        }
    }
}

// Manual Debug implementation - trait objects can't derive Debug
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

// NOTE: No Default impl - every page must explicitly specify all fields including `upvote`.
// This prevents accidentally omitting upvote config for pages that should have it.

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
/// This is the ONLY place this should be used - in the 404 handler/dispatcher
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
};

/// Look up a page by name (case-insensitive)
pub fn lookup_page(name: &str) -> Option<&'static PageInfo> {
    let lowercase = name.to_lowercase();
    PAGE_REGISTRY.get(lowercase.as_str()).map(|p| p as &'static PageInfo)
}

/// Extract page name from a URL path and look it up.
/// Handles stripping leading slash and .htm/.html extensions.
/// Returns None for paths that don't map to a known page.
pub fn lookup_page_from_path(path: &str) -> Option<&'static PageInfo> {
    // Handle root path
    if path == "/" {
        return lookup_page("");
    }

    let path_without_slash = path.strip_prefix('/').unwrap_or(path);
    let path_lower = path_without_slash.to_lowercase();

    // Strip .htm or .html extension (case-insensitive)
    let name = if path_lower.ends_with(".htm") {
        &path_without_slash[..path_without_slash.len() - 4]
    } else if path_lower.ends_with(".html") {
        &path_without_slash[..path_without_slash.len() - 5]
    } else {
        path_without_slash
    };

    // Try lookup, also try with trailing slash for directories
    lookup_page(name).or_else(|| {
        let with_slash = format!("{}/", name.trim_end_matches('/'));
        lookup_page(&with_slash)
    })
}

/// Get the canonical URL for a page slug
/// Returns the URL path using the canonical case from the registry,
/// with .htm extension (or trailing / for directories)
pub fn canonical_url(slug: &str) -> String {
    if let Some(info) = lookup_page(slug) {
        // Use the canonical case from the registry
        let canonical_slug = info.slug;
        if canonical_slug.is_empty() {
            "/".to_string()
        } else if info.is_directory() {
            format!("/{}/", canonical_slug.trim_end_matches('/'))
        } else {
            format!("/{}.htm", canonical_slug)
        }
    } else if slug.is_empty() {
        "/".to_string()
    } else {
        // Unknown page, use provided slug with .htm
        format!("/{}.htm", slug)
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

    #[test]
    fn test_canonical_url() {
        assert_eq!(canonical_url(""), "/");
        assert_eq!(canonical_url("about"), "/about.htm");
    }

    #[test]
    fn test_lookup_page_from_path() {
        use super::lookup_page_from_path;

        // Basic lookup with .htm
        let info = lookup_page_from_path("/about.htm").unwrap();
        assert_eq!(info.slug, "about");

        // Lookup without extension
        let info = lookup_page_from_path("/about").unwrap();
        assert_eq!(info.slug, "about");

        // Lookup with .html
        let info = lookup_page_from_path("/about.html").unwrap();
        assert_eq!(info.slug, "about");

        // Case-insensitive extension
        let info = lookup_page_from_path("/about.HTM").unwrap();
        assert_eq!(info.slug, "about");

        // Root path
        let info = lookup_page_from_path("/").unwrap();
        assert_eq!(info.slug, "");

        // Unknown page returns None
        assert!(lookup_page_from_path("/nonexistent.htm").is_none());
    }
}

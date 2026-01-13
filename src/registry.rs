//! Page Registry - Single source of truth for all pages and their metadata.
//!
//! This is the Rust equivalent of PHP's $PAGE_INFO array in URLParse.php.
//! To add a new page:
//! 1. Add an entry to PAGE_REGISTRY below
//! 2. Create the template in templates/pages/{name}.html
//! 3. For dynamic pages, create a handler in src/pages/{name}.rs

use std::collections::HashMap;
use std::sync::LazyLock;

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
#[derive(Debug, Clone)]
pub struct PageInfo {
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

// NOTE: No Default impl - every page must explicitly specify all fields including `upvote`.
// This prevents accidentally omitting upvote config for pages that should have it.

/// Helper macro for defining alias pages (redirects)
macro_rules! alias {
    ($slug:expr => $target:expr) => {
        PageInfo {
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

/// Helper macro for defining regular pages
/// Required: slug, title, description, keywords, hit_id, upvote
/// Optional: no_cache (defaults to false)
macro_rules! page {
    (
        slug: $slug:expr,
        title: $title:expr,
        description: $description:expr,
        keywords: $keywords:expr,
        hit_id: $hit_id:expr,
        upvote: $upvote:expr $(,)?
    ) => {
        PageInfo {
            slug: $slug,
            title: $title,
            description: $description,
            keywords: $keywords,
            legacy_hit_count_id: $hit_id,
            redirect: None,
            no_cache: false,
            upvote: $upvote,
        }
    };
    (
        slug: $slug:expr,
        title: $title:expr,
        description: $description:expr,
        keywords: $keywords:expr,
        hit_id: $hit_id:expr,
        upvote: $upvote:expr,
        no_cache: $no_cache:expr $(,)?
    ) => {
        PageInfo {
            slug: $slug,
            title: $title,
            description: $description,
            keywords: $keywords,
            legacy_hit_count_id: $hit_id,
            redirect: None,
            no_cache: $no_cache,
            upvote: $upvote,
        }
    };
}

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
/// This is the ONLY place this should be used - in the 404 handler
pub static NOT_FOUND_PAGE_INFO: PageInfo = PageInfo {
    slug: "404",
    title: "Page Not Found - Defuse Security",
    description: "",
    keywords: "",
    legacy_hit_count_id: "",
    redirect: None,
    no_cache: false,
    upvote: None,
};

/// The page registry - all pages on the site
///
/// Keys are LOWERCASE for case-insensitive lookup.
/// The `slug` field in PageInfo stores the canonical case for URLs.
/// Empty string "" is the home page.
pub static PAGE_REGISTRY: LazyLock<HashMap<&'static str, PageInfo>> = LazyLock::new(|| {
    let pages: &[PageInfo] = &[
        // ===== Home page =====
        page! {
            slug: "",
            title: "",
            description: "",
            keywords: "",
            hit_id: "pages/home.html",
            upvote: None,
        },

        // ===== Home page aliases =====
        alias!("index" => ""),
        alias!("index.html" => ""),
        alias!("index.php" => ""),

        // ===== Main pages =====
        page! {
            slug: "about",
            title: "About - Defuse Security",
            description: "About Defuse Security.",
            keywords: "",
            hit_id: "pages/about.html",
            upvote: None,
        },
        page! {
            slug: "contact",
            title: "Defuse Security's Contact Information",
            description: "Defuse Security's contact information.",
            keywords: "",
            hit_id: "pages/contact.html",
            upvote: None,
        },
        alias!("key" => "contact"),

        // ===== Services =====
        page! {
            slug: "checksums",
            title: "Online Text and File Hash Calculator - MD5, SHA1, SHA256, SHA512, WHIRLPOOL Hash Calculator - Defuse Security",
            description: "Online Hash Tool. Calculate hash of file or text. MD5, SHA1, SHA256, SHA512 and more...",
            keywords: "",
            hit_id: "pages/services/checksums.php",
            upvote: Some(UpvoteConfig {
                id: "onlinechecksums",
                category: "defuse_pages",
                title: Some("Online Hash Calculator"),
                description: Some("A tool for computing hashes (MD5, SHA1, SHA2, etc.) of text and files."),
            }),
        },
        page! {
            slug: "pastebin",
            title: "Encrypted Pastebin - Keep your data private and secure! - Defuse Security",
            description: "An Encrypted, Anonymous, Secure, and PRIVATE Pastebin.",
            keywords: "",
            hit_id: "pages/services/pastebin.php",
            upvote: None,
        },
        page! {
            slug: "trustedthirdparty",
            title: "TRENT - FREE Third party Drawing Service - Defuse Security",
            description: "TRENT, the trusted random number generator for contests and drawings.",
            keywords: "",
            hit_id: "pages/services/trustedthirdparty.php",
            upvote: None,
        },
        alias!("trent" => "trustedthirdparty"),
        page! {
            slug: "big-number-calculator",
            title: "Online Big Number Calculator",
            description: "Calculate enormous mathematical equations from within your browser.",
            keywords: "",
            hit_id: "pages/services/big-number-calculator.php",
            upvote: None,
        },
        page! {
            slug: "online-x86-assembler",
            title: "Online x86 and x64 Intel Instruction Assembler",
            description: "Easily find out which bytes your x86 ASM instructions assemble to.",
            keywords: "",
            hit_id: "pages/services/online-x86-assembler.php",
            upvote: None,
        },
        page! {
            slug: "html-sanitize",
            title: "Online HTML Sanitizer Tool - htmlspecialchars - Defuse Security",
            description: "Convert text containing special characters into proper HTML.",
            keywords: "",
            hit_id: "pages/services/html-sanitize.php",
            upvote: None,
        },
        page! {
            slug: "software-security-auditing",
            title: "Software Security Auditing",
            description: "Get your software audited for security bugs.",
            keywords: "",
            hit_id: "pages/services/software-security-auditing.html",
            upvote: None,
        },
        page! {
            slug: "quantum-computer-time-capsule",
            title: "Send a Message to the Future (Digital Time Capsule)",
            description: "Save a message that will become readable after quantum computers are built.",
            keywords: "",
            hit_id: "pages/services/quantum-computer-time-capsule.php",
            upvote: None,
        },

        // ===== Software =====
        page! {
            slug: "passgen",
            title: "Secure Windows & Linux Password Generator - Defuse Security",
            description: "A secure random password generator for Windows, Linux and Macintosh.",
            keywords: "",
            hit_id: "pages/software/passgen.php",
            upvote: None,
            no_cache: true, // SECURITY: Prevent browsers from caching generated passwords
        },
        alias!("passwords" => "passgen"),
        alias!("password" => "passgen"),
        alias!("pass" => "passgen"),
        page! {
            slug: "helloworld-cms",
            title: "Secure and Light CMS for PHP - Defuse Security",
            description: "A lightweight, ultra-secure CMS for PHP",
            keywords: "",
            hit_id: "pages/software/helloworld-cms.html",
            upvote: None,
        },
        page! {
            slug: "php-hash-cracker",
            title: "Salted Hash Cracking PHP Script - Defuse Security",
            description: "Dictionary hash cracking PHP scripts (supports LOTS of hash types!!)",
            keywords: "",
            hit_id: "pages/software/php-hash-cracker.html",
            upvote: None,
        },
        page! {
            slug: "backup-verify-script",
            title: "Script for Comparing Folders and Validating Backups",
            description: "A command-line script for verifying backups by comparing two folders in Linux",
            keywords: "",
            hit_id: "pages/software/backup-verify-script.html",
            upvote: None,
        },
        page! {
            slug: "sockstress",
            title: "Sockstress Denial of Service Tool & Source Code - Defuse Security",
            description: "A C implementation of the sockstress attack from 2008.",
            keywords: "",
            hit_id: "pages/software/sockstress.html",
            upvote: None,
        },

        // ===== Research =====
        page! {
            slug: "password-policy-hall-of-shame",
            title: "Password Policy Hall of SHAME - Defuse Security",
            description: "List of websites and services that impose password restrictions.",
            keywords: "",
            hit_id: "pages/research/password-policy-hall-of-shame.php",
            upvote: None,
        },
        alias!("pphos" => "password-policy-hall-of-shame"),
        page! {
            slug: "side-channel-attacks-on-everyday-applications",
            title: "Side-Channel Attacks on Everyday Applications (Black Hat 2016)",
            description: "Data and code for my paper applying FLUSH+RELOAD to break privacy.",
            keywords: "",
            hit_id: "pages/research/side-channel-attacks-on-everyday-applications.html",
            upvote: None,
        },
        // BH2016 is the canonical case - case-insensitive lookup handles "bh2016" automatically
        alias!("BH2016" => "side-channel-attacks-on-everyday-applications"),
        page! {
            slug: "concentration-bounds-from-parallel-repetition-theorems",
            title: "Concentration Bounds from Parallel Repetition Theorems",
            description: "My master's thesis.",
            keywords: "",
            hit_id: "pages/research/concentration-bounds-from-parallel-repetition-theorems.html",
            upvote: None,
        },
        page! {
            slug: "godel-second-incompleteness-theorem-by-turing-machines",
            title: "A Simple Proof of Gödel's Second Incompleteness Theorem Using Turing Machines",
            description: "Proving Gödel's second incompleteness theorem in a simpler way using Turing machines.",
            keywords: "godel, second incompleteness theorem, simple proof, turing machines, computability",
            hit_id: "pages/research/godel-second-incompleteness-theorem-by-turing-machines.html",
            upvote: None,
        },
        page! {
            slug: "plausible-reason-p-noteq-np-is-hard-to-prove",
            title: "A Plausible Reason It's So Hard To Prove P!=NP",
            description: "Attempting to show why P!=NP is hard to prove using hash functions.",
            keywords: "p versus np, hard to prove, hash functions, language collisions",
            hit_id: "pages/research/plausible-reason-p-noteq-np-is-hard-to-prove.html",
            upvote: None,
        },
        page! {
            slug: "blind-birthday-attack",
            title: "Blind Birthday Attack",
            description: "Birthday attack without seeing the values.",
            keywords: "birthday attack, blind, double hmac, cryptography",
            hit_id: "pages/research/blind-birthday-attack.php",
            upvote: Some(UpvoteConfig {
                id: "blindbirthdayattack",
                category: "defuse_pages",
                title: Some("Blind Birthday Attack"),
                description: Some("A birthday attack without knowing what the collision actually is."),
            }),
        },

        // ===== Audits =====
        alias!("audits/" => "software-security-auditing"),
        page! {
            slug: "audits/encfs",
            title: "EncFS Security Audit",
            description: "Security audit of the EncFS encrypted filesystem.",
            keywords: "",
            hit_id: "pages/audits/encfs.html",
            upvote: None,
        },
        page! {
            slug: "audits/ecryptfs",
            title: "eCryptfs Security Audit",
            description: "Security audit of the eCryptfs encrypted filesystem.",
            keywords: "",
            hit_id: "pages/audits/ecryptfs.html",
            upvote: None,
        },
        page! {
            slug: "audits/zerobin",
            title: "ZeroBin Security Audit",
            description: "Security audit of the ZeroBin Zero-Knowledge Pastebin",
            keywords: "",
            hit_id: "pages/audits/zerobin.html",
            upvote: None,
        },
        page! {
            slug: "audits/pefs",
            title: "PEFS Security Audit",
            description: "Security audit of the PEFS encrypted filesystem.",
            keywords: "",
            hit_id: "pages/audits/pefs.html",
            upvote: None,
        },
        page! {
            slug: "audits/hash0",
            title: "Hash0 Security Audit",
            description: "Security audit of the Hash0 password system",
            keywords: "",
            hit_id: "pages/audits/hash0.html",
            upvote: None,
        },
        page! {
            slug: "audits/gocryptfs",
            title: "Gocryptfs Security Audit",
            description: "Security audit of the gocryptfs encrypted filesystem",
            keywords: "",
            hit_id: "pages/audits/gocryptfs.html",
            upvote: None,
        },

        // ===== Misc pages =====
        page! {
            slug: "honestyware",
            title: "Honestyware - The right way to sell software.",
            description: "Honestyware is a revolutionary way to sell software that embraces piracy.",
            keywords: "",
            hit_id: "pages/misc/honestyware.html",
            upvote: None,
        },
        page! {
            slug: "reading-list",
            title: "Reading List - Defuse Security",
            description: "Everything I have read so far.",
            keywords: "",
            hit_id: "pages/misc/reading-list.html",
            upvote: None,
        },
        page! {
            slug: "vimrc",
            title: "My .vimrc - Defuse Security",
            description: "My Vim configuration file",
            keywords: "",
            hit_id: "pages/misc/vimrc.html",
            upvote: None,
        },
        page! {
            slug: "transparency",
            title: "Transparency Report",
            description: "",
            keywords: "",
            hit_id: "pages/misc/transparency.html",
            upvote: None,
        },
        page! {
            slug: "the-universe-is-made-of-cheese",
            title: "The Universe is Made of Cheese - A Formal Proof",
            description: "A logical proof that the universe consists entirely of cheese.",
            keywords: "",
            hit_id: "pages/misc/the-universe-is-made-of-cheese.html",
            upvote: None,
        },
        page! {
            slug: "fractal-zoom",
            title: "Fractal Zoom",
            description: "A psychedelic short story.",
            keywords: "fractal zoom, short story, sci-fi, psychedelic",
            hit_id: "pages/misc/fractal-zoom.html",
            upvote: None,
        },
        page! {
            slug: "advice-to-aspiring-computer-engineers",
            title: "Advice to Aspiring Computer Engineers and Scientists",
            description: "Advice for new computer science students.",
            keywords: "",
            hit_id: "pages/misc/advice-to-aspiring-computer-engineers.html",
            upvote: None,
        },
        page! {
            slug: "asuskeyboarddefect",
            title: "ASUS G50 G51 Keyboard Problem",
            description: "Solution to the keyboard problem for the ASUS G50, G51, and G51VX series laptops.",
            keywords: "",
            hit_id: "pages/misc/asuskeyboarddefect.html",
            upvote: None,
        },
        alias!("keyboarddefect" => "asuskeyboarddefect"),

        // TODO: Add remaining pages from PHP $PAGE_INFO
        // See defuse.ca/src/libs/URLParse.php for the full list
    ];

    // Build HashMap with lowercase keys for case-insensitive lookup
    pages
        .iter()
        .map(|p| (p.slug.to_lowercase().leak() as &'static str, p.clone()))
        .collect()
});

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

    let path_without_slash = path.trim_start_matches('/');
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
        let trent = lookup_page("trent").unwrap();
        assert_eq!(trent.redirect, Some("trustedthirdparty"));
    }

    #[test]
    fn test_canonical_url() {
        assert_eq!(canonical_url(""), "/");
        assert_eq!(canonical_url("about"), "/about.htm");
        assert_eq!(canonical_url("audits/encfs"), "/audits/encfs.htm");
    }

    #[test]
    fn test_canonical_case_preserved() {
        // BH2016 should preserve its case
        let info = lookup_page("bh2016").unwrap();
        assert_eq!(info.slug, "BH2016");
        assert_eq!(canonical_url("bh2016"), "/BH2016.htm");
    }

    #[test]
    fn test_directory_url() {
        assert_eq!(canonical_url("audits/"), "/audits/");
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

    #[test]
    fn test_no_cache_flag() {
        use super::lookup_page_from_path;

        // passgen has no_cache: true
        let info = lookup_page_from_path("/passgen.htm").unwrap();
        assert!(info.no_cache);

        // about has no_cache: false (default)
        let info = lookup_page_from_path("/about.htm").unwrap();
        assert!(!info.no_cache);
    }
}

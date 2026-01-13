//! Page Registry - Single source of truth for all pages and their metadata.
//!
//! This is the Rust equivalent of PHP's $PAGE_INFO array in URLParse.php.
//! To add a new page:
//! 1. Add an entry to PAGE_REGISTRY below
//! 2. Create the template in templates/pages/{name}.html
//! 3. For dynamic pages, create a handler in src/pages/{name}.rs

use std::collections::HashMap;
use std::sync::LazyLock;

/// Information about a single page
#[derive(Debug, Clone)]
pub struct PageInfo {
    /// URL slug - the canonical name as it appears in URLs (e.g., "about", "BH2016")
    pub slug: &'static str,

    /// Page title (empty = use DEFAULT_TITLE)
    pub title: &'static str,

    /// Meta description (empty = use DEFAULT_META_DESCRIPTION)
    pub description: &'static str,

    /// Meta keywords (empty = use DEFAULT_META_KEYWORDS)
    pub keywords: &'static str,

    /// PHP-style page ID for backward compatibility with PHPCount database
    /// e.g., Some("pages/services/checksums.php"), Some("pages/home.html")
    /// None means use the slug as the page ID (for new pages)
    pub php_page_id: Option<&'static str>,

    /// Redirect target - if Some, this page is an alias
    /// Some("") means redirect to home page
    /// None means this is a real page, not an alias
    pub redirect: Option<&'static str>,

    /// Is this a directory-style URL? (ends with /, no .htm extension)
    pub is_directory: bool,

    /// Does this page have dynamic logic? (form handling, database, etc.)
    pub is_dynamic: bool,

    /// Should this page have no-cache headers?
    /// SECURITY: Used for pages like passgen to prevent password caching
    pub no_cache: bool,
}

impl Default for PageInfo {
    fn default() -> Self {
        Self {
            slug: "",
            title: "",
            description: "",
            keywords: "",
            php_page_id: None,
            redirect: None,
            is_directory: false,
            is_dynamic: false,
            no_cache: false,
        }
    }
}

impl PageInfo {
    /// Get the page ID for PHPCount hit tracking
    /// Uses php_page_id if set, otherwise falls back to slug
    pub fn hit_counter_id(&self) -> &'static str {
        self.php_page_id.unwrap_or(self.slug)
    }

    /// Check if this page is an alias (has a redirect target)
    pub fn is_alias(&self) -> bool {
        self.redirect.is_some()
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

/// The page registry - all pages on the site
///
/// Keys are LOWERCASE for case-insensitive lookup.
/// The `slug` field in PageInfo stores the canonical case for URLs.
/// Empty string "" is the home page.
pub static PAGE_REGISTRY: LazyLock<HashMap<&'static str, PageInfo>> = LazyLock::new(|| {
    // All pages defined with explicit field names for clarity
    let pages: &[PageInfo] = &[
        // ===== Home page =====
        PageInfo {
            slug: "",
            php_page_id: Some("pages/home.html"),
            is_directory: true,
            is_dynamic: true,
            ..Default::default()
        },

        // ===== Home page aliases =====
        PageInfo { slug: "index",      redirect: Some(""), ..Default::default() },
        PageInfo { slug: "index.html", redirect: Some(""), ..Default::default() },
        PageInfo { slug: "index.php",  redirect: Some(""), ..Default::default() },

        // ===== Main pages =====
        PageInfo {
            slug: "about",
            php_page_id: Some("pages/about.html"),
            title: "About - Defuse Security",
            description: "About Defuse Security.",
            ..Default::default()
        },
        PageInfo {
            slug: "contact",
            php_page_id: Some("pages/contact.html"),
            title: "Defuse Security's Contact Information",
            description: "Defuse Security's contact information.",
            ..Default::default()
        },
        PageInfo { slug: "key", redirect: Some("contact"), ..Default::default() },

        // ===== Services =====
        PageInfo {
            slug: "checksums",
            php_page_id: Some("pages/services/checksums.php"),
            title: "Online Text and File Hash Calculator - MD5, SHA1, SHA256, SHA512, WHIRLPOOL Hash Calculator - Defuse Security",
            description: "Online Hash Tool. Calculate hash of file or text. MD5, SHA1, SHA256, SHA512 and more...",
            is_dynamic: true,
            ..Default::default()
        },
        PageInfo {
            slug: "pastebin",
            title: "Encrypted Pastebin - Keep your data private and secure! - Defuse Security",
            description: "An Encrypted, Anonymous, Secure, and PRIVATE Pastebin.",
            is_dynamic: true,
            ..Default::default()
        },
        PageInfo {
            slug: "trustedthirdparty",
            title: "TRENT - FREE Third party Drawing Service - Defuse Security",
            description: "TRENT, the trusted random number generator for contests and drawings.",
            is_dynamic: true,
            ..Default::default()
        },
        PageInfo { slug: "trent", redirect: Some("trustedthirdparty"), ..Default::default() },
        PageInfo {
            slug: "big-number-calculator",
            title: "Online Big Number Calculator",
            description: "Calculate enormous mathematical equations from within your browser.",
            is_dynamic: true,
            ..Default::default()
        },
        PageInfo {
            slug: "online-x86-assembler",
            title: "Online x86 and x64 Intel Instruction Assembler",
            description: "Easily find out which bytes your x86 ASM instructions assemble to.",
            is_dynamic: true,
            ..Default::default()
        },
        PageInfo {
            slug: "html-sanitize",
            title: "Online HTML Sanitizer Tool - htmlspecialchars - Defuse Security",
            description: "Convert text containing special characters into proper HTML.",
            is_dynamic: true,
            ..Default::default()
        },
        PageInfo {
            slug: "software-security-auditing",
            title: "Software Security Auditing",
            description: "Get your software audited for security bugs.",
            ..Default::default()
        },
        PageInfo {
            slug: "quantum-computer-time-capsule",
            title: "Send a Message to the Future (Digital Time Capsule)",
            description: "Save a message that will become readable after quantum computers are built.",
            is_dynamic: true,
            ..Default::default()
        },

        // ===== Software =====
        PageInfo {
            slug: "passgen",
            title: "Secure Windows & Linux Password Generator - Defuse Security",
            description: "A secure random password generator for Windows, Linux and Macintosh.",
            is_dynamic: true,
            // SECURITY: Prevent browsers from caching generated passwords
            no_cache: true,
            ..Default::default()
        },
        PageInfo { slug: "passwords", redirect: Some("passgen"), ..Default::default() },
        PageInfo { slug: "password",  redirect: Some("passgen"), ..Default::default() },
        PageInfo { slug: "pass",      redirect: Some("passgen"), ..Default::default() },
        PageInfo {
            slug: "helloworld-cms",
            title: "Secure and Light CMS for PHP - Defuse Security",
            description: "A lightweight, ultra-secure CMS for PHP",
            ..Default::default()
        },
        PageInfo {
            slug: "php-hash-cracker",
            title: "Salted Hash Cracking PHP Script - Defuse Security",
            description: "Dictionary hash cracking PHP scripts (supports LOTS of hash types!!)",
            ..Default::default()
        },
        PageInfo {
            slug: "backup-verify-script",
            title: "Script for Comparing Folders and Validating Backups",
            description: "A command-line script for verifying backups by comparing two folders in Linux",
            ..Default::default()
        },
        PageInfo {
            slug: "sockstress",
            title: "Sockstress Denial of Service Tool & Source Code - Defuse Security",
            description: "A C implementation of the sockstress attack from 2008.",
            ..Default::default()
        },

        // ===== Research =====
        PageInfo {
            slug: "password-policy-hall-of-shame",
            title: "Password Policy Hall of SHAME - Defuse Security",
            description: "List of websites and services that impose password restrictions.",
            is_dynamic: true,
            ..Default::default()
        },
        PageInfo { slug: "pphos", redirect: Some("password-policy-hall-of-shame"), ..Default::default() },
        PageInfo {
            slug: "side-channel-attacks-on-everyday-applications",
            title: "Side-Channel Attacks on Everyday Applications (Black Hat 2016)",
            description: "Data and code for my paper applying FLUSH+RELOAD to break privacy.",
            ..Default::default()
        },
        // BH2016 is the canonical case - case-insensitive lookup handles "bh2016" automatically
        PageInfo { slug: "BH2016", redirect: Some("side-channel-attacks-on-everyday-applications"), ..Default::default() },
        PageInfo {
            slug: "concentration-bounds-from-parallel-repetition-theorems",
            title: "Concentration Bounds from Parallel Repetition Theorems",
            description: "My master's thesis.",
            ..Default::default()
        },
        PageInfo {
            slug: "godel-second-incompleteness-theorem-by-turing-machines",
            title: "A Simple Proof of Gödel's Second Incompleteness Theorem Using Turing Machines",
            description: "Proving Gödel's second incompleteness theorem in a simpler way using Turing machines.",
            keywords: "godel, second incompleteness theorem, simple proof, turing machines, computability",
            ..Default::default()
        },
        PageInfo {
            slug: "plausible-reason-p-noteq-np-is-hard-to-prove",
            title: "A Plausible Reason It's So Hard To Prove P!=NP",
            description: "Attempting to show why P!=NP is hard to prove using hash functions.",
            keywords: "p versus np, hard to prove, hash functions, language collisions",
            ..Default::default()
        },

        // ===== Audits =====
        PageInfo { slug: "audits/", redirect: Some("software-security-auditing"), is_directory: true, ..Default::default() },
        PageInfo {
            slug: "audits/encfs",
            title: "EncFS Security Audit",
            description: "Security audit of the EncFS encrypted filesystem.",
            ..Default::default()
        },
        PageInfo {
            slug: "audits/ecryptfs",
            title: "eCryptfs Security Audit",
            description: "Security audit of the eCryptfs encrypted filesystem.",
            ..Default::default()
        },
        PageInfo {
            slug: "audits/zerobin",
            title: "ZeroBin Security Audit",
            description: "Security audit of the ZeroBin Zero-Knowledge Pastebin",
            ..Default::default()
        },
        PageInfo {
            slug: "audits/pefs",
            title: "PEFS Security Audit",
            description: "Security audit of the PEFS encrypted filesystem.",
            ..Default::default()
        },
        PageInfo {
            slug: "audits/hash0",
            title: "Hash0 Security Audit",
            description: "Security audit of the Hash0 password system",
            ..Default::default()
        },
        PageInfo {
            slug: "audits/gocryptfs",
            title: "Gocryptfs Security Audit",
            description: "Security audit of the gocryptfs encrypted filesystem",
            ..Default::default()
        },

        // ===== Misc pages =====
        PageInfo {
            slug: "honestyware",
            title: "Honestyware - The right way to sell software.",
            description: "Honestyware is a revolutionary way to sell software that embraces piracy.",
            ..Default::default()
        },
        PageInfo {
            slug: "reading-list",
            title: "Reading List - Defuse Security",
            description: "Everything I have read so far.",
            ..Default::default()
        },
        PageInfo {
            slug: "vimrc",
            title: "My .vimrc - Defuse Security",
            description: "My Vim configuration file",
            ..Default::default()
        },
        PageInfo {
            slug: "transparency",
            title: "Transparency Report",
            ..Default::default()
        },
        PageInfo {
            slug: "the-universe-is-made-of-cheese",
            title: "The Universe is Made of Cheese - A Formal Proof",
            description: "A logical proof that the universe consists entirely of cheese.",
            ..Default::default()
        },
        PageInfo {
            slug: "fractal-zoom",
            title: "Fractal Zoom",
            description: "A psychedelic short story.",
            keywords: "fractal zoom, short story, sci-fi, psychedelic",
            ..Default::default()
        },
        PageInfo {
            slug: "advice-to-aspiring-computer-engineers",
            title: "Advice to Aspiring Computer Engineers and Scientists",
            description: "Advice for new computer science students.",
            ..Default::default()
        },
        PageInfo {
            slug: "asuskeyboarddefect",
            title: "ASUS G50 G51 Keyboard Problem",
            description: "Solution to the keyboard problem for the ASUS G50, G51, and G51VX series laptops.",
            ..Default::default()
        },
        PageInfo { slug: "keyboarddefect", redirect: Some("asuskeyboarddefect"), ..Default::default() },

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
        } else if info.is_directory {
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
        assert!(trent.is_alias());
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

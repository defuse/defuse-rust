//! Page definitions for the registry.
//!
//! This file contains all page entries for PAGE_REGISTRY.
//! Edit this file to add, remove, or modify pages.

use std::collections::HashMap;
use std::sync::LazyLock;

use super::{alias, page, PageInfo, UpvoteConfig};

/// The page registry - all pages on the site
///
/// Keys are LOWERCASE for case-insensitive lookup.
/// The `slug` field in PageInfo stores the canonical case for URLs.
/// Empty string "" is the home page.
pub static PAGE_REGISTRY: LazyLock<HashMap<&'static str, PageInfo>> = LazyLock::new(|| {
    let pages: &[PageInfo] = &[
        // ===== Home page =====
        page! {
            handler: home,
            slug: "",
            title: "",
            description: "",
            keywords: "",
            legacy_hit_count_id: "pages/home.html",
            upvote: None,
        },

        // ===== Home page aliases =====
        alias!("index" => ""),
        alias!("index.html" => ""),
        alias!("index.php" => ""),

        // ===== Main pages =====
        page! {
            handler: about,
            slug: "about",
            title: "About - Defuse Security",
            description: "About Defuse Security.",
            keywords: "",
            legacy_hit_count_id: "pages/about.html",
            upvote: None,
        },
        page! {
            handler: contact,
            slug: "contact",
            title: "Defuse Security's Contact Information",
            description: "Defuse Security's contact information.",
            keywords: "",
            legacy_hit_count_id: "pages/contact.html",
            upvote: None,
        },
        alias!("key" => "contact"),

        // ===== Services =====
        page! {
            handler: checksums,
            slug: "checksums",
            title: "Online Text and File Hash Calculator - MD5, SHA1, SHA256, SHA512, WHIRLPOOL Hash Calculator - Defuse Security",
            description: "Online Hash Tool. Calculate hash of file or text. MD5, SHA1, SHA256, SHA512 and more...",
            keywords: "",
            legacy_hit_count_id: "pages/services/checksums.php",
            upvote: Some(UpvoteConfig {
                id: "onlinechecksums",
                category: "defuse_pages",
                title: Some("Online Hash Calculator"),
                description: Some("A tool for computing hashes (MD5, SHA1, SHA2, etc.) of text and files."),
            }),
        },

        // ===== Research =====
        page! {
            handler: blind_birthday_attack,
            slug: "blind-birthday-attack",
            title: "Blind Birthday Attack",
            description: "Birthday attack without seeing the values.",
            keywords: "birthday attack, blind, double hmac, cryptography",
            legacy_hit_count_id: "pages/research/blind-birthday-attack.php",
            upvote: Some(UpvoteConfig {
                id: "blindbirthdayattack",
                category: "defuse_pages",
                title: Some("Blind Birthday Attack"),
                description: Some("A birthday attack without knowing what the collision actually is."),
            }),
        },
        page! {
            handler: cbcmodeiv,
            slug: "cbcmodeiv",
            title: "Encryption - CBC Mode IV: Secret or Not?",
            description: "Should the IV in CBC mode be kept secret?",
            keywords: "cbc mode, iv, initialization vector, encryption, cryptography",
            legacy_hit_count_id: "pages/research/cbcmodeiv.php",
            upvote: Some(UpvoteConfig {
                id: "cbcmodeiv",
                category: "defuse_pages",
                title: Some("Encryption - CBC Mode IV: Secret or Not?"),
                description: Some("Should the IV in CBC mode be kept secret?"),
            }),
        },

        // ===== Test pages =====
        page! {
            handler: panic_test,
            slug: "panic-test",
            title: "Panic Test",
            description: "Test page that panics during rendering.",
            keywords: "",
            legacy_hit_count_id: "",
            upvote: None,
        },
    ];

    // Build HashMap with lowercase keys for case-insensitive lookup
    pages
        .iter()
        .map(|p| (p.slug.to_lowercase().leak() as &'static str, p.clone()))
        .collect()
});

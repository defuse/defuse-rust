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
            title: "",
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
        page! {
            handler: donated,
            slug: "donated",
            title: "",
            description: "",
            keywords: "",
            legacy_hit_count_id: "pages/donated.html",
            upvote: None,
        },
        page! {
            handler: security_contact_vulnerability_disclosure,
            slug: "security-contact-vulnerability-disclosure",
            title: "Security Contact and Vulnerability Disclosure",
            description: "How to disclose vulnerabilities in Defuse Security services",
            keywords: "full disclosure, security contact, vulnerabilities",
            legacy_hit_count_id: "pages/security-contact-vulnerability-disclosure.php",
            upvote: None,
        },
        page! {
            handler: privacy_policy,
            slug: "privacy-policy",
            title: "",
            description: "",
            keywords: "",
            legacy_hit_count_id: "pages/privacy.html",
            upvote: None,
        },
        page! {
            handler: transparency,
            slug: "transparency",
            title: "Transparency Report",
            description: "",
            keywords: "",
            legacy_hit_count_id: "pages/transparency.php",
            upvote: None,
        },

        // ===== Services =====
        page! {
            handler: services::checksums,
            slug: "checksums",
            title: "Online Text and File Hash Calculator - MD5, SHA1, SHA256, SHA512, WHIRLPOOL Hash Calculator - Defuse Security",
            description: "Online Hash Tool. Calculate hash of file or text. MD5, SHA1, SHA256, SHA512 and more...",
            keywords: "file hasher, online, hash, md5, sha256, sha1, text hash, checksum",
            legacy_hit_count_id: "pages/services/checksums.php",
            upvote: Some(UpvoteConfig {
                id: "onlinechecksums",
                category: "defuse_pages",
                title: Some("Online Hash Calculator"),
                description: Some("A tool for computing hashes (MD5, SHA1, SHA2, etc.) of text and files."),
            }),
        },
        page! {
            handler: services::software_security_auditing,
            slug: "software-security-auditing",
            title: "Software Security Auditing",
            description: "Get your software audited for security bugs.",
            keywords: "software security, exploits, auditing",
            legacy_hit_count_id: "pages/services/software-security-auditing.php",
            upvote: None,
        },

        // ===== Research =====
        page! {
            handler: research::blind_birthday_attack,
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
            handler: research::cbcmodeiv,
            slug: "cbcmodeiv",
            title: "Should CBC Mode Initialization Vector Be Secret - Defuse Security",
            description: "Should the initialization vector used for CBC mode be kept secret?",
            keywords: "cbc mode, encryption, initialization vector, iv, secret, secure",
            legacy_hit_count_id: "pages/research/cbcmodeiv.php",
            upvote: Some(UpvoteConfig {
                id: "cbcmodeiv",
                category: "defuse_pages",
                title: Some("Encryption - CBC Mode IV: Secret or Not?"),
                description: Some("Should the IV in CBC mode be kept secret?"),
            }),
        },
        page! {
            handler: research::concentration_bounds_from_parallel_repetition_theorems,
            slug: "concentration-bounds-from-parallel-repetition-theorems",
            title: "Concentration Bounds from Parallel Repetition Theorems",
            description: "My master's thesis, showing how concentration bounds can be derived from parallel repetition theorems for nonlocal games and interactive proofs.",
            keywords: "concentration bounds, parallel repetition theorems, quantum information, symmetric key strengthening",
            legacy_hit_count_id: "pages/research/concentration-bounds-from-parallel-repetition-theorems.php",
            upvote: Some(UpvoteConfig {
                id: "mastersthesis",
                category: "defuse_pages",
                title: Some("Concentration Bounds from Parallel Repetition Theorems"),
                description: Some("My master's thesis, showing how concentration bounds can be derived from parallel repetition theorems for nonlocal games and interactive proofs."),
            }),
        },

        // ===== Miscellaneous =====
        page! {
            handler: misc::contributors,
            slug: "contributors",
            title: "Contributors",
            description: "A list of people and organizations that have contributed to Defuse Security",
            keywords: "contribution, donation, thanks",
            legacy_hit_count_id: "pages/misc/contributors.php",
            upvote: None,
        },

        // ===== Software =====
        page! {
            handler: software::backup_verify_script,
            slug: "backup-verify-script",
            title: "Script for Comparing Folders and Validating Backups",
            description: "A command-line script for verifying backups by comparing two folders in Linux",
            keywords: "backup validate, backup verify, compare folders, linux, ruby",
            legacy_hit_count_id: "pages/software/backup-verify-script.php",
            upvote: Some(UpvoteConfig {
                id: "backupverifyscript",
                category: "defuse_pages",
                title: Some("Backup Verifier Script (Ruby)"),
                description: Some("A Ruby script that compares two directories and reports the differences."),
            }),
        },
        page! {
            handler: software::helloworld_cms,
            slug: "helloworld-cms",
            title: "Secure and Light CMS for PHP - Defuse Security",
            description: "A lightweight, ultra-secure CMS for PHP",
            keywords: "secure cms, php cms, light cms, small cms, tiny cms, cms",
            legacy_hit_count_id: "pages/software/helloworld-cms.html",
            upvote: Some(UpvoteConfig {
                id: "helloworld",
                category: "defuse_pages",
                title: Some("HelloWorld! - A Light & Secure CDS for PHP"),
                description: Some("A lightweight, ultra-secure, CMS/CDS for PHP"),
            }),
        },
        page! {
            handler: software::software,
            slug: "software",
            title: "Defuse Security's Software",
            description: "Software created by Defuse Security",
            keywords: "",
            legacy_hit_count_id: "pages/software/software.html",
            upvote: None,
        },
        page! {
            handler: software::php_hash_cracker,
            slug: "php-hash-cracker",
            title: "Salted Hash Cracking PHP Script - Defuse Security",
            description: "Dictionary hash cracking PHP scripts (supports LOTS of hash types!!)",
            keywords: "hash cracking, dictionary attack, php hash cracking script",
            legacy_hit_count_id: "pages/software/php-hash-cracker.php",
            upvote: Some(UpvoteConfig {
                id: "phphashcracker",
                category: "defuse_pages",
                title: Some("Salted Hash Cracking PHP Script"),
                description: Some("A script for cracking hashes when all you have is PHP."),
            }),
        },
        page! {
            handler: software::sockstress,
            slug: "sockstress",
            title: "Sockstress Denial of Service Tool & Source Code - Defuse Security",
            description: "A C implementation of the sockstress attack from 2008.",
            keywords: "sockstress, source code, denial of service, proof of concept, dos, ddos",
            legacy_hit_count_id: "pages/software/sockstress.php",
            upvote: Some(UpvoteConfig {
                id: "sockstress",
                category: "defuse_pages",
                title: Some("Sockstress DoS Tool"),
                description: Some("A public domain C implementation of the sockstress DoS attack."),
            }),
        },
        page! {
            handler: software::winrrng,
            slug: "winrrng",
            title: "Real Random Number Generator for Windows - Defuse Security",
            description: "A real random number generator for Windows",
            keywords: "",
            legacy_hit_count_id: "pages/software/winrrng.html",
            upvote: None,
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

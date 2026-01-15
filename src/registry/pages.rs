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
        page! {
            handler: research::cracking_synergy_bad_cryptography,
            slug: "cracking-synergy-bad-cryptography",
            title: "Cracking Synergy's Bad Cryptography",
            description: "A tool to crack Synergy's homebrew cryptography",
            keywords: "synergy, cryptography, crack, homebrew, keyboard sharing, mouse sharing",
            legacy_hit_count_id: "pages/research/cracking-synergy-bad-cryptography.php",
            upvote: Some(UpvoteConfig {
                id: "synergy_cracking",
                category: "defuse_pages",
                title: Some("Cracking Synergy's Bad Cryptography"),
                description: Some("A tool to crack Synergy's homebrew cryptography."),
            }),
        },
        page! {
            handler: research::in_browser_port_scanning,
            slug: "in-browser-port-scanning",
            title: "Port Scanning Local Network From a Web Browser",
            description: "Malicious web pages can port scan your local network.",
            keywords: "browser, port scan, security",
            legacy_hit_count_id: "pages/research/in-browser-port-scanning.php",
            upvote: Some(UpvoteConfig {
                id: "inbrowserportscanner",
                category: "defuse_pages",
                title: Some("Timing Side Channel Port Scanner in the Browser"),
                description: Some("How web pages can use a timing side channel to \"scan\" your local network."),
            }),
        },
        page! {
            handler: research::filesystem_events_ntfs_permissions,
            slug: "filesystem-events-ntfs-permissions",
            title: "File System Events Disclose NTFS Protected Folder Contents - Defuse Security",
            description: "Obtain list of files in folder protected with NTFS permissions via filesystem events",
            keywords: "",
            legacy_hit_count_id: "pages/research/filesystemevents.html",
            upvote: Some(UpvoteConfig {
                id: "filesystemevents",
                category: "defuse_pages",
                title: Some("File System Events Leak Folder Contents"),
                description: Some("An information disclosure vulnerability in Windows shared folders that lets you see what's in folers you can't access."),
            }),
        },
        page! {
            handler: research::instruction_filters,
            slug: "instruction-filters",
            title: "Instruction Filters as an Exploitation Defense",
            description: "Disabling CPU instructions to thwart ROP and other attacks.",
            keywords: "instruction set filters, insfilter, research",
            legacy_hit_count_id: "pages/research/instruction-filters.php",
            upvote: None,
        },
        page! {
            handler: research::onedetection,
            slug: "onedetection",
            title: "The PUP Confusion Antivirus Detection Evasion Technique - Defuse Security",
            description: "The PUP Confusion Antivirus Detection Evasion Technique. Multiple detections per file.",
            keywords: "antivirus, single detection, only one detection, can't detect more than one, multiple virus, two viruses in one file",
            legacy_hit_count_id: "pages/research/onedetection.html",
            upvote: Some(UpvoteConfig {
                id: "pupconfusion",
                category: "defuse_pages",
                title: Some("The PUP Confusion Technique"),
                description: Some("Undetecting malware by making it look like a Potentially Unwanted Program (PUP)."),
            }),
        },
        page! {
            handler: research::race_conditions_in_web_applications,
            slug: "race-conditions-in-web-applications",
            title: "Practical Race Condition (TOCTTOU) Vulnerabilities in Web Applications - Defuse Security",
            description: "Query-level race conditions can lead to serious but hard to find vulnerabilities in web applications.",
            keywords: "",
            legacy_hit_count_id: "pages/research/race-conditions-in-web-applications.php",
            upvote: Some(UpvoteConfig {
                id: "raceconditions",
                category: "defuse_pages",
                title: Some("Practical Race Condition Vulnerabilities in Web Applications"),
                description: Some("An example of a web application (PHP) vulnerable to a race condition, and how to fix it."),
            }),
        },
        page! {
            handler: research::research,
            slug: "research",
            title: "Defuse Security's Research",
            description: "Research projects by Defuse Security",
            keywords: "",
            legacy_hit_count_id: "pages/research/research.html",
            upvote: None,
        },
        page! {
            handler: research::side_channel_attacks_on_everyday_applications,
            slug: "side-channel-attacks-on-everyday-applications",
            title: "Side-Channel Attacks on Everyday Applications (Black Hat 2016)",
            description: "Data and code for my paper applying FLUSH+RELOAD to break privacy.",
            keywords: "cache side channel, experiment data, flush, reload, privacy",
            legacy_hit_count_id: "pages/research/side-channel-attacks-on-everyday-applications.php",
            upvote: Some(UpvoteConfig {
                id: "side-channel-attacks-on-everyday-applications",
                category: "defuse_pages",
                title: Some("Side-Channel Attacks on Everyday Applications"),
                description: Some("My Black Hat USA 2016 talk about the Flush+Reload side channel."),
            }),
        },
        alias!("bh2016" => "side-channel-attacks-on-everyday-applications"),
        alias!("BH2016" => "side-channel-attacks-on-everyday-applications"),
        page! {
            handler: research::side_channels_in_encoding_functions,
            slug: "side-channels-in-encoding-functions",
            title: "Side Channel Attacks in Hex and Base64 Encoding",
            description: "Do encoding functions like bin2hex and base64_encode create side channels?",
            keywords: "side channel, side channel attack, encoding, bin2hex, base64",
            legacy_hit_count_id: "pages/research/side-channels-in-encoding-functions.php",
            upvote: None,
        },
        page! {
            handler: research::manual_random_number_generator,
            slug: "manual-random-number-generator",
            title: "Manually Generating Random Numbers",
            description: "Manually generating random numbers for cryptographic use.",
            keywords: "random numbers, true random, csprng, cryptographically secure",
            legacy_hit_count_id: "pages/research/manual-random-number-generator.php",
            upvote: Some(UpvoteConfig {
                id: "manualrng",
                category: "defuse_pages",
                title: Some("A Manual Random Number Generator"),
                description: Some("Generating random numbers with paper coins."),
            }),
        },
        page! {
            handler: research::eotp,
            slug: "eotp",
            title: "Encrypting One Time Passwords System - Defuse Security",
            description: "A One Time Password protocol that can be used with encryption.",
            keywords: "encrypting one time passwords, static key, one time password",
            legacy_hit_count_id: "pages/research/eotp.html",
            upvote: Some(UpvoteConfig {
                id: "eotp",
                category: "defuse_pages",
                title: Some("Encrypting One Time Passwords (EOTP)"),
                description: Some("EOTP is a cryptographic One Time Password (OTP) protocol designed to provide a static encryption key across login sessions."),
            }),
        },
        page! {
            handler: research::truecrypt_plausible_deniability_useless_by_game_theory,
            slug: "truecrypt-plausible-deniability-useless-by-game-theory",
            title: "TrueCrypt's Plausible Deniability (Hidden Volumes) is Theoretically Useless",
            description: "How game theory shows that TrueCrypt's hidden volume feature is provably useless in some scenarios.",
            keywords: "game theory, truecrypt plausible deniability",
            legacy_hit_count_id: "pages/research/truecrypt-plausible-deniability-useless-by-game-theory.php",
            upvote: Some(UpvoteConfig {
                id: "truecryptgametheory",
                category: "defuse_pages",
                title: Some("TrueCrypt's Plausible Deniability is Theoretically Useless"),
                description: Some("Why you really ought to have a hidden volume, even if you don't need one."),
            }),
        },
        page! {
            handler: research::web_browser_javascript_cryptography,
            slug: "web-browser-javascript-cryptography",
            title: "Web Browser Cryptography is a Good Thing",
            description: "Arguments for and against doing cryptography in the browser.",
            keywords: "javascript cryptography, web browser cryptography, browser cryptography, encryption",
            legacy_hit_count_id: "pages/research/web-browser-javascript-cryptography.php",
            upvote: Some(UpvoteConfig {
                id: "webbrowsercryptography",
                category: "defuse_pages",
                title: Some("Web Browser Cryptography is a Good Thing"),
                description: Some("Why we should support the development of browser-based crypto applications."),
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

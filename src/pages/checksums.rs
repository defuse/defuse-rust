use askama::Template;
use askama_axum::IntoResponse;
use axum::Form;
use axum::http::HeaderMap;
use serde::Deserialize;

use md5::Md5;
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512, Digest};

use crate::context::PageContext;

// Supported algorithms in display order (matching PHP)
const SUPPORTED_ALGORITHMS: &[&str] = &[
    "md5", "sha1", "sha256", "sha384", "sha512",
    "md5(md5())",
];

#[derive(Template)]
#[template(path = "pages/checksums.html")]
pub struct ChecksumsPage {
    pub title: &'static str,
    pub input: String,
    pub normalize: bool,
    pub results: Vec<HashResult>,
    pub supported_algorithms: &'static [&'static str],
    // Base template context
    pub is_home: bool,
    pub client_ip: String,
    pub dnt_enabled: bool,
    pub page_hits: u64,
    pub unique_hits: u64,
}

pub struct HashResult {
    pub algorithm: &'static str,
    pub hash: String,
}

#[derive(Deserialize)]
pub struct ChecksumsForm {
    data: String,
    #[serde(default)]
    normalize: Option<String>,
}

// GET: Show empty form
pub async fn get(headers: HeaderMap) -> impl IntoResponse {
    let ctx = PageContext::from_headers(&headers);
    ChecksumsPage {
        title: "Online Text and File Hash Calculator - MD5, SHA1, SHA256, SHA512, WHIRLPOOL Hash Calculator - Defuse Security",
        input: String::new(),
        normalize: false,
        results: Vec::new(),
        supported_algorithms: SUPPORTED_ALGORITHMS,
        is_home: ctx.is_home,
        client_ip: ctx.client_ip,
        dnt_enabled: ctx.dnt_enabled,
        page_hits: ctx.page_hits,
        unique_hits: ctx.unique_hits,
    }
}

// POST: Calculate hashes and show results
pub async fn post(headers: HeaderMap, Form(form): Form<ChecksumsForm>) -> impl IntoResponse {
    let ctx = PageContext::from_headers(&headers);
    let normalize = form.normalize.as_deref() == Some("yes");

    let data = if normalize {
        form.data.replace("\r", "").replace("\n", "")
    } else {
        form.data.clone()
    };

    let results = compute_hashes(&data);

    ChecksumsPage {
        title: "Online Text and File Hash Calculator - MD5, SHA1, SHA256, SHA512, WHIRLPOOL Hash Calculator - Defuse Security",
        input: form.data,
        normalize,
        results,
        supported_algorithms: SUPPORTED_ALGORITHMS,
        is_home: ctx.is_home,
        client_ip: ctx.client_ip,
        dnt_enabled: ctx.dnt_enabled,
        page_hits: ctx.page_hits,
        unique_hits: ctx.unique_hits,
    }
}

fn compute_hashes(input: &str) -> Vec<HashResult> {
    let bytes = input.as_bytes();

    vec![
        HashResult {
            algorithm: "md5",
            hash: hex::encode(Md5::digest(bytes)),
        },
        HashResult {
            algorithm: "sha1",
            hash: hex::encode(Sha1::digest(bytes)),
        },
        HashResult {
            algorithm: "sha256",
            hash: hex::encode(Sha256::digest(bytes)),
        },
        HashResult {
            algorithm: "sha384",
            hash: hex::encode(Sha384::digest(bytes)),
        },
        HashResult {
            algorithm: "sha512",
            hash: hex::encode(Sha512::digest(bytes)),
        },
        HashResult {
            algorithm: "md5(md5())",
            hash: hex::encode(Md5::digest(hex::encode(Md5::digest(bytes)).as_bytes())),
        },
    ]
}

use askama::Template;
use askama_axum::IntoResponse;
use axum::Form;
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
    pub ctx: PageContext,
    // Page-specific fields
    pub input: String,
    pub normalize: bool,
    pub results: Vec<HashResult>,
    pub supported_algorithms: &'static [&'static str],
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
pub async fn get(ctx: PageContext) -> impl IntoResponse {
    ChecksumsPage {
        ctx,
        input: String::new(),
        normalize: false,
        results: Vec::new(),
        supported_algorithms: SUPPORTED_ALGORITHMS,
    }
}

// POST: Calculate hashes and show results
pub async fn post(ctx: PageContext, Form(form): Form<ChecksumsForm>) -> impl IntoResponse {
    let normalize = form.normalize.as_deref() == Some("yes");

    let data = if normalize {
        form.data.replace("\r", "").replace("\n", "")
    } else {
        form.data.clone()
    };

    let results = compute_hashes(&data);

    ChecksumsPage {
        ctx,
        input: form.data,
        normalize,
        results,
        supported_algorithms: SUPPORTED_ALGORITHMS,
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

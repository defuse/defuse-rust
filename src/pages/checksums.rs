use askama::Template;
use axum::response::IntoResponse;
use bytes::Bytes;
use serde::Deserialize;

use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};

use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler};
use crate::app_state::AppState;

const SUPPORTED_ALGORITHMS: &[&str] = &["md5", "sha1", "sha256", "sha384", "sha512", "md5(md5())"];

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move {
            ChecksumsPage {
                ctx,
                input: String::new(),
                normalize: false,
                results: Vec::new(),
                supported_algorithms: SUPPORTED_ALGORITHMS,
            }
            .into_response()
        })
    }

    fn post(&self, ctx: PageContext, _state: &AppState, body: Bytes) -> Option<BoxFuture> {
        Some(Box::pin(async move {
            let form: ChecksumsForm = serde_urlencoded::from_bytes(&body)
                .expect("Failed to parse checksums form");

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
            .into_response()
        }))
    }
}

#[derive(Template)]
#[template(path = "pages/checksums.html")]
struct ChecksumsPage {
    ctx: PageContext,
    input: String,
    normalize: bool,
    results: Vec<HashResult>,
    supported_algorithms: &'static [&'static str],
}

pub struct HashResult {
    pub algorithm: &'static str,
    pub hash: String,
}

#[derive(Deserialize, Default)]
struct ChecksumsForm {
    #[serde(default)]
    data: String,
    #[serde(default)]
    normalize: Option<String>,
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

//! Online x86/x64 Assembler page handler.
//!
//! A tool for assembling and disassembling x86/x64 machine code.

use askama::Template;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler, PostBody};
use crate::libs::html_escape;
use crate::libs::x86_assembler::{self, Arch, AssemblerError, AssemblyResult};

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move {
            OnlineX86AssemblerPage {
                ctx,
                instructions: String::new(),
                hexstring: String::new(),
                x86_checked: true,
                x64_checked: false,
                assembly_result: None,
                disassembly_result: None,
                error: None,
            }
            .into_response()
        })
    }

    fn post(&self, ctx: PageContext, _state: &AppState, body: PostBody) -> Option<BoxFuture> {
        Some(Box::pin(async move {
            match body {
                PostBody::UrlEncoded(bytes) => {
                    let form: AssemblerForm =
                        serde_urlencoded::from_bytes(&bytes).unwrap_or_default();

                    // Parse architecture (whitelist to x86 or x64)
                    let (arch, x86_checked, x64_checked) = match form.arch.as_str() {
                        "x64" => (Arch::X64, false, true),
                        _ => (Arch::X86, true, false), // Default to x86
                    };

                    let mut assembly_result = None;
                    let mut disassembly_result = None;
                    let mut error = None;

                    // Check which operation was requested
                    if form.submit.as_deref() == Some("Assemble") && !form.instructions.is_empty() {
                        match x86_assembler::assemble(&form.instructions, arch) {
                            Ok(result) => assembly_result = Some(result),
                            Err(e) => error = Some(format_error(&e)),
                        }
                    } else if form.submit.as_deref() == Some("Disassemble") && !form.hexstring.is_empty() {
                        match x86_assembler::disassemble(&form.hexstring, arch) {
                            Ok(result) => disassembly_result = Some(result),
                            Err(e) => error = Some(format_error(&e)),
                        }
                    }

                    OnlineX86AssemblerPage {
                        ctx,
                        instructions: form.instructions,
                        hexstring: form.hexstring,
                        x86_checked,
                        x64_checked,
                        assembly_result,
                        disassembly_result,
                        error,
                    }
                    .into_response()
                }
                PostBody::Multipart { .. } => {
                    // This page doesn't support multipart forms
                    OnlineX86AssemblerPage {
                        ctx,
                        instructions: String::new(),
                        hexstring: String::new(),
                        x86_checked: true,
                        x64_checked: false,
                        assembly_result: None,
                        disassembly_result: None,
                        error: None,
                    }
                    .into_response()
                }
            }
        }))
    }
}

/// Format an error for display using HtmlEscape.
fn format_error(error: &AssemblerError) -> String {
    html_escape::escape_text(&error.to_string(), true, 4)
}

#[derive(Template)]
#[template(path = "pages/services/online_x86_assembler.html")]
struct OnlineX86AssemblerPage {
    ctx: PageContext,
    instructions: String,
    hexstring: String,
    x86_checked: bool,
    x64_checked: bool,
    assembly_result: Option<AssemblyResult>,
    disassembly_result: Option<AssemblyResult>,
    error: Option<String>,
}

impl OnlineX86AssemblerPage {
    /// Format the disassembly text with HTML escaping.
    fn format_disassembly(&self, result: &AssemblyResult) -> String {
        html_escape::escape_text(&result.disassembly, true, 4)
    }
}

#[derive(Deserialize, Default)]
struct AssemblerForm {
    #[serde(default)]
    instructions: String,
    #[serde(default)]
    hexstring: String,
    #[serde(default)]
    arch: String,
    #[serde(default)]
    submit: Option<String>,
}

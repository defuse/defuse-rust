//! Big Number Calculator page handler.
//!
//! A tool for calculating with arbitrary-precision numbers using Ruby.

use askama::Template;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler, PostBody};
use crate::libs::big_number_calculator::{self, CalculatorOptions, CalculatorResult, OutputBase};

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move {
            BigNumberCalculatorPage {
                ctx,
                eqn: String::new(),
                base: OutputBase::Decimal,
                add_spaces: true, // Default to checked (matches PHP GET behavior)
                result: None,
            }
            .into_response()
        })
    }

    fn post(&self, ctx: PageContext, _state: &AppState, body: PostBody) -> Option<BoxFuture> {
        Some(Box::pin(async move {
            match body {
                PostBody::UrlEncoded(bytes) => {
                    let form: CalculatorForm =
                        serde_urlencoded::from_bytes(&bytes).unwrap_or_default();

                    let base = OutputBase::from_str(&form.base);
                    let add_spaces = form.addspaces.is_some();

                    // Only calculate if submit button was pressed
                    let result = if form.submit.is_some() && !form.eqn.is_empty() {
                        let options = CalculatorOptions { base, add_spaces };
                        Some(big_number_calculator::calculate(&form.eqn, &options).await)
                    } else {
                        None
                    };

                    BigNumberCalculatorPage {
                        ctx,
                        eqn: form.eqn,
                        base,
                        add_spaces,
                        result,
                    }
                    .into_response()
                }
                PostBody::Multipart { .. } => {
                    // This page doesn't support multipart forms
                    BigNumberCalculatorPage {
                        ctx,
                        eqn: String::new(),
                        base: OutputBase::Decimal,
                        add_spaces: true,
                        result: None,
                    }
                    .into_response()
                }
            }
        }))
    }
}

#[derive(Template)]
#[template(path = "pages/services/big_number_calculator.html")]
struct BigNumberCalculatorPage {
    ctx: PageContext,
    eqn: String,
    base: OutputBase,
    add_spaces: bool,
    result: Option<CalculatorResult>,
}

#[derive(Deserialize, Default)]
struct CalculatorForm {
    #[serde(default)]
    eqn: String,
    #[serde(default)]
    base: String,
    #[serde(default)]
    addspaces: Option<String>,
    #[serde(default)]
    submit: Option<String>,
}

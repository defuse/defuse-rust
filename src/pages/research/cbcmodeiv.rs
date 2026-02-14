use askama::Template;
use axum::response::IntoResponse;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, PageHandler};
use crate::libs::bibliography::{Bibliography, Reference};

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        let bib = Bibliography::new(&[
            Reference::Full {
                authors: "Phil Rogaway",
                date: "April 3, 1995",
                title: "Problems with Proposed IP Cryptography",
                url: "http://www.vpnc.org/ietf-ipsec/92.ipsec/msg01847.html",
            },
            Reference::Simple {
                title: "Twitter discussion between @DefuseSec and @tqbf",
                url: "https://twitter.com/tqbf/status/376873119199137792",
            },
        ]);
        Box::pin(async move { CbcModeIvPage { ctx, bib }.into_response() })
    }
}

#[derive(Template)]
#[template(path = "pages/research/cbcmodeiv.html")]
struct CbcModeIvPage {
    ctx: PageContext,
    bib: Bibliography,
}

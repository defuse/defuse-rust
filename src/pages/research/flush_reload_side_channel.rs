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
                authors: "Yuval Yarom, Katrina Falkner",
                date: "July 18, 2013",
                title: "FLUSH+RELOAD: a High Resolution, Low Noise, L3 Cache Side-Channel Attack",
                url: "http://eprint.iacr.org/2013/448.pdf",
            },
            Reference::Full {
                authors: "Dmitry Asonov, Rakesh Agrawal",
                date: "2004",
                title: "Keyboard Acoustic Emanations",
                url: "http://rakesh.agrawal-family.com/papers/ssp04kba.pdf",
            },
            Reference::Full {
                authors: "Li Zhuang, Feng Zhou, J. D. Tygar",
                date: "November 2005",
                title: "Keyboard Acoustic Emanations Revisited",
                url: "http://www.cs.berkeley.edu/~tygar/papers/Keyboard_Acoustic_Emanations_Revisited/TISSEC.pdf",
            },
            Reference::Full {
                authors: "C. E. Shannon",
                date: "October 1949",
                title: "Communication Theory of Secrecy Systems",
                url: "http://www3.alcatel-lucent.com/bstj/vol28-1949/articles/bstj28-4-656.pdf",
            },
        ]);
        Box::pin(async move { FlushReloadSideChannelPage { ctx, bib }.into_response() })
    }
}

#[derive(Template)]
#[template(path = "pages/research/flush_reload_side_channel.html")]
struct FlushReloadSideChannelPage {
    ctx: PageContext,
    bib: Bibliography,
}

//! `error card` — a turn that failed.
//!
//! A [`fold_card`](crate::components::fold_card) like every other evidence
//! row, so it collapses to one line and opens onto the whole message: a
//! stack trace or a provider's JSON is often long, and an error that cannot
//! be folded away pushes the conversation off the screen for the rest of the
//! session.
//!
//! It wears `is-err` rather than the agent's colour — the point is the
//! failure, and the turn it failed in may not even have an agent.

use rustex1_kit::h::{m, Node};

use crate::components::{fold_card, samples, session};
use crate::css;

pub const CSS: &str = r#"
.error-demo { width: min(560px, 100%); }
.error-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// What this kind contributes to a message block.
pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![fold_card::fold(
        st,
        v,
        "fold-error is-err",
        "warning-octagon",
        "Error",
        None,
        fold_card::pre(&format!("{v}.text")),
    )
    .iff(&format!("{v}.kind === 'error'"))]
}

/// standalone demo — the card in the row the transcript draws it in.
pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::row("error"));
    m("div").class("error-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(session::row(st, &v, parts(st, &v))),
    )
}

pub fn register() {
    css::inject("error-card", CSS);
}

//! `assistant card` — what an agent said, and its byline.
//!
//! The body is markdown the store rendered: safe for `x-html` because the
//! renderer escapes the source before emitting any tag, so every tag here is
//! one it wrote (`helpers::markdown_to_html`).

use rustex1_kit::h::{m, Node};

use crate::components::{name_tag, samples, session};
use crate::css;

pub const CSS: &str = r#"
.assistant-demo { width: min(560px, 100%); }
.assistant-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// What this kind contributes to a message block.
pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![m("div")
        .class("bubble-assistant md-body")
        .iff(&format!("{v}.kind === 'assistant'"))
        .kids(vec![
            m("div").inner_html(&format!("{v}.html || {v}.text")),
            // Below the answer, not above it. You read what was said, then
            // who said it — and in a group that attribution is the only
            // thing telling three agents' bubbles apart. Whenever there is a
            // handle, not only in a group: a single agent still has a name, a
            // colour and a time.
            name_tag::byline(
                st,
                &format!("{v}.handle"),
                &format!("{v}.ts"),
                &format!("{v}.handle"),
            ),
        ])]
}

/// standalone demo — the card in the row the transcript draws it in.
pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::row("assistant"));
    m("div").class("assistant-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(session::row(st, &v, parts(st, &v))),
    )
}

pub fn register() {
    css::inject("assistant-card", CSS);
}

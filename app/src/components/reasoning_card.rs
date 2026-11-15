//! `reasoning card` — the model thinking out loud, folded away.
//!
//! A [`fold_card`](crate::components::fold_card) with a brain on it. The
//! collapse behaviour, the click target and the agent-coloured tint all come
//! from that primitive; what this module decides is the glyph, the label and
//! that the body is rendered markdown.

use rustex1_kit::h::{m, Node};

use crate::components::{fold_card, samples, session};
use crate::css;

pub const CSS: &str = r#"
.reasoning-demo { width: min(560px, 100%); }
.reasoning-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// What this kind contributes to a message block.
///
/// Same pre-rendered markdown as the answer card, and safe for `x-html` for
/// the same reason: the renderer escapes before it emits any tag.
pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![fold_card::fold(
        st,
        v,
        "fold-reasoning",
        "brain",
        "Reasoning",
        None,
        m("div")
            .class("fold-body md-body")
            .inner_html(&format!("{v}.html || {v}.text")),
    )
    .iff(&format!("{v}.kind === 'reasoning'"))]
}

/// standalone demo — the card in the row the transcript draws it in.
pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::row("reasoning"));
    m("div").class("reasoning-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(session::row(st, &v, parts(st, &v))),
    )
}

pub fn register() {
    css::inject("reasoning-card", CSS);
}

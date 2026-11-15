//! `tool response card` — what the tool gave back, and whether it worked.
//!
//! A [`fold_card`](crate::components::fold_card) whose head is conditional in
//! three places at once — the glyph (check or prohibit), the note (`ok` /
//! `failed` / `denied`) and the card's own `is-err` modifier, which takes it
//! out of the agent's colour. All three go through the primitive's own
//! seams: it used to re-draw the fold markup to get them, which is one more
//! copy of a thing that should exist once.

use rustex1_kit::h::{m, Node};

use crate::components::{fold_card, samples, session};
use crate::css;

pub const CSS: &str = r#"
.tool-response-demo { width: min(560px, 100%); }
.tool-response-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// What this kind contributes to a message block.
pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![fold_card::fold_as(
        st,
        v,
        "fold-tool-result",
        // A denied or failed call must not wear the agent's colour.
        &format!("({v}.ok === false || {v}.denied ? 'is-err' : 'ok')"),
        fold_card::head_dyn(
            &format!("'ph ' + ({v}.ok === false || {v}.denied ? 'ph-prohibit' : 'ph-check-circle')"),
            &format!("{v}.name"),
            Some(&format!(
                "({v}.denied ? 'denied' : {v}.ok === false ? 'failed' : 'ok')"
            )),
        ),
        fold_card::pre(&format!("{v}.text || ''")),
    )
    .iff(&format!("{v}.kind === 'tool_result'"))]
}

/// standalone demo — the card in the row the transcript draws it in.
pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::row("tool_result"));
    m("div").class("tool-response-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(session::row(st, &v, parts(st, &v))),
    )
}

pub fn register() {
    css::inject("tool-response-card", CSS);
}

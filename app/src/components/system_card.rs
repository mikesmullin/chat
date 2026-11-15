//! `system card` — the app talking, not a participant.
//!
//! A slash command's answer, the server saying nobody is here to reply, a
//! turn that was stopped. Deliberately not a bubble: it is not something
//! anyone said.

use rustex1_kit::h::{m, Node};

use crate::components::{samples, session};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .bubble-system {
  display: flex; gap: 7px; align-items: flex-start;
  font-size: 12px; color: var(--gray-dim); line-height: 1.6;
  padding: 2px 0; white-space: pre-wrap;
}
:is(#sb, #home) .bubble-system i { line-height: 19px; }

.system-demo { width: min(560px, 100%); }
.system-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// What this kind contributes to a message block.
pub fn parts(_st: &str, v: &str) -> Vec<Node> {
    vec![m("div")
        .class("bubble-system")
        .iff(&format!("{v}.kind === 'system'"))
        .kids(vec![
            Node::icon("info").class("ph ph-info"),
            m("span").text(&format!("{v}.text")),
        ])]
}

/// standalone demo — the card in the row the transcript draws it in.
pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::row("system"));
    m("div").class("system-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(session::row(st, &v, parts(st, &v))),
    )
}

pub fn register() {
    css::inject("system-card", CSS);
}

//! `queued card` — the turn is waiting for its model.
//!
//! A local model serves one inference request at a time, so a turn can wait
//! for a free slot of its `provider:model` before anything streams. The
//! wait is a row on the rail, not a silent stall: the user must be able to
//! tell "slow model" from "still in line". The row's text names the model
//! it is waiting for; the badge, when the line is longer than one, says how
//! many turns are ahead of it.

use rustex1_kit::h::{m, Node};

use crate::components::{samples, session};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .bubble-queued {
  display: flex; gap: 7px; align-items: center;
  font-size: 12px; color: var(--gray-dim); line-height: 1.6;
  padding: 2px 0; white-space: pre-wrap;
}
:is(#sb, #home) .bubble-queued i { line-height: 19px; }
:is(#sb, #home) .queued-text { overflow-wrap: anywhere; min-width: 0; }
:is(#sb, #home) .queued-pos {
  flex: none; padding: 0 6px; border-radius: 99px;
  border: 1px solid var(--line); color: var(--gray-dim);
  font: 500 10px/16px var(--mono); letter-spacing: .04em;
}

.queued-demo { width: min(560px, 100%); }
.queued-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// What this kind contributes to a message block.
pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![m("div")
        .class("bubble-queued")
        .iff(&format!("{v}.kind === 'queued'"))
        .kids(vec![
            Node::icon("hourglass").class("ph ph-hourglass"),
            m("span").class("queued-text").text(&format!("{v}.text")),
            // 1 is "next in line" — nothing to say. Past that, the count.
            m("span")
                .class("queued-pos")
                .show(&format!("{v}.position > 1"))
                .text(&format!("{v}.position + ' in line'")),
            m("span")
                .class("name-tag-when")
                .show(&format!("{v}.ts"))
                .text(&format!("'· ' + {st}.relAgo({v}.ts)")),
        ])]
}

/// standalone demo — the card in the row the transcript draws it in.
pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::row("queued"));
    m("div").class("queued-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(session::row(st, &v, parts(st, &v))),
    )
}

pub fn register() {
    css::inject("queued-card", CSS);
}

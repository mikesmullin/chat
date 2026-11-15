//! `decision card` — why an agent spoke, or did not.
//!
//! An agent set to `relevant` engagement has its turn decided by a small
//! model before the big one is asked anything. This card is that verdict, in
//! the decider's own words — "only 33% sure (needs 60%)" answers "why was it
//! quiet", where a bare percentage leaves you to guess the bar.

use rustex1_kit::h::{m, Node};

use crate::components::{fold_card, samples, session};
use crate::css;

pub const CSS: &str = r#"
.decision-demo { width: min(560px, 100%); }
.decision-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// What this kind contributes to a message block.
pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![fold_card::fold(
        st,
        v,
        "fold-decision",
        "funnel",
        "Engagement",
        Some(&format!(
            "(({v}.speak ? 'will reply' : 'stays quiet') + ' · ' + ({v}.reason || ({v}.confidence ? Math.round({v}.confidence * 100) + '%' : '?')))"
        )),
        fold_card::pre(&format!("JSON.stringify({v}.verdict, null, 2)")),
    )
    .iff(&format!("{v}.kind === 'decision'"))]
}

/// standalone demo — the card in the row the transcript draws it in.
pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::row("decision"));
    m("div").class("decision-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(session::row(st, &v, parts(st, &v))),
    )
}

pub fn register() {
    css::inject("decision-card", CSS);
}

use rustex1_kit::h::{m, Node};

use crate::components::{fold_card, samples, session};
use crate::css;

pub const CSS: &str = r#"
.reasoning-demo { width: min(560px, 100%); }
.reasoning-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

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

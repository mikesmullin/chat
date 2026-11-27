use rustex1_kit::h::{m, Node};

use crate::components::{fold_card, samples, session};
use crate::css;

pub const CSS: &str = r#"
.error-demo { width: min(560px, 100%); }
.error-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

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

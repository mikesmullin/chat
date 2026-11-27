use rustex1_kit::h::{m, Node};

use crate::components::{name_tag, samples, session};
use crate::css;

pub const CSS: &str = r#"
.assistant-demo { width: min(560px, 100%); }
.assistant-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![m("div")
        .class("bubble-assistant md-body")
        .iff(&format!("{v}.kind === 'assistant'"))
        .kids(vec![
            m("div").inner_html(&format!("{v}.html || {v}.text")),
            name_tag::byline(
                st,
                &format!("{v}.handle"),
                &format!("{v}.ts"),
                &format!("{v}.handle"),
            ),
        ])]
}

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

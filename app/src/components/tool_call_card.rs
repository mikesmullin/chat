use rustex1_kit::h::{m, Node};

use crate::components::{fold_card, samples, session};
use crate::css;

pub const CSS: &str = r#"
.tool-call-demo { width: min(560px, 100%); }
.tool-call-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![fold_card::fold(
        st,
        v,
        "fold-tool-call",
        "wrench",
        "Tool call",
        Some(&format!("{v}.name")),
        fold_card::pre(&format!("{v}.args ? JSON.stringify({v}.args, null, 2) : ''")),
    )
    .iff(&format!("{v}.kind === 'tool_call'"))]
}

pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::row("tool_call"));
    m("div").class("tool-call-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(session::row(st, &v, parts(st, &v))),
    )
}

pub fn register() {
    css::inject("tool-call-card", CSS);
}

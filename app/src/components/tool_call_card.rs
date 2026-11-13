//! `tool call card` — a tool invocation: wrench header with the mono
//! tool name, a plain-language explanation, and the code/args block.
//! Verbatim app classes.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.tool-call-demo { width: min(560px, 100%); }
.tool-call-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub const NAME: &str = "read";
pub const EXPLANATION: &str = "Reading the current auth module to map its endpoints.";
pub const CODE: &str = "src/auth.py";

/// the card.
pub fn card(name: &str, explanation: &str, code: &str) -> Node {
    m("div").class("tool-card tool-call-card").kids(vec![
        m("div").class("tool-call-header").kids(vec![
            Node::icon("wrench"),
            m("span").class("tool-call-name").child(lit(name)),
        ]),
        m("div")
            .class("tool-call-explanation")
            .child(lit(explanation)),
        m("pre").class("tool-call-code").child(lit(code)),
    ])
}

/// standalone demo.
pub fn demo() -> Node {
    m("div").class("tool-call-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(card(NAME, EXPLANATION, CODE)),
    )
}

pub fn register() {
    css::inject("tool-call-card", CSS);
}

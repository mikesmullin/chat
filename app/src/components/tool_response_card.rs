//! `tool response card` — a tool result: check header with the tool
//! name plus a green (`ok`) or red (`err`) edge, then the result body.
//! Verbatim app classes.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.tool-response-demo { width: min(560px, 100%); }
.tool-response-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub const NAME: &str = "read";
pub const BODY: &str = "312 lines · exports: login, refresh, revoke";

/// the card. `ok` picks the green/red edge.
pub fn card(name: &str, body: &str, ok: bool) -> Node {
    m("div")
        .class(if ok {
            "tool-card tool-card-row ok"
        } else {
            "tool-card tool-card-row err"
        })
        .kids(vec![
            m("div").class("tool-result-header").kids(vec![
                Node::icon("check-circle"),
                m("span").child(lit(name)),
            ]),
            m("pre").class("tool-result-body").child(lit(body)),
        ])
}

/// standalone demo: one ok result.
pub fn demo() -> Node {
    m("div").class("tool-response-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(card(NAME, BODY, true)),
    )
}

pub fn register() {
    css::inject("tool-response-card", CSS);
}

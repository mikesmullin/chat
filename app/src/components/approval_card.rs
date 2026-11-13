//! `approval card` — a tool-approval prompt: amber plate, warning
//! title, the tool name, and Allow / Always / Deny. Static showcase (the
//! buttons are inert, like the tab demo's close buttons).

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.approval-demo { width: min(560px, 100%); }
.approval-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub const NAME: &str = "write src/auth.py";

/// the card, pending.
pub fn card(name: &str) -> Node {
    m("div").class("approval-card pending").kids(vec![
        m("div").class("font-semibold text-sm approval-title").kids(vec![
            Node::icon("warning"),
            lit(" Tool approval required"),
        ]),
        m("div").class("text-xs mt-1 font-mono").child(lit(name)),
        m("div").class("flex gap-2 mt-2").kids(vec![
            m("button")
                .class("btn-allow")
                .attr("type", "button")
                .child(lit("Allow")),
            m("button")
                .class("btn-always")
                .attr("type", "button")
                .child(lit("Always")),
            m("button")
                .class("btn-deny")
                .attr("type", "button")
                .child(lit("Deny")),
        ]),
    ])
}

/// standalone demo.
pub fn demo() -> Node {
    m("div").class("approval-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(card(NAME)),
    )
}

pub fn register() {
    css::inject("approval-card", CSS);
}

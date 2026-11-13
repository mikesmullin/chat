//! `reasoning card` — the model's thinking block: violet plate, brain
//! header, expanded static body. Verbatim app classes (the violet is the
//! app's contract, close to the guide's pastel-violet role).

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.reasoning-demo { width: min(560px, 100%); }
.reasoning-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub const TEXT: &str =
    "The old endpoints must keep working, so a shim beats a flag day. Map first, then migrate.";

/// the block, expanded.
pub fn card(text: &str) -> Node {
    m("div").class("bubble-reasoning").kids(vec![
        m("div").class("reasoning-header").kids(vec![
            Node::icon("brain"),
            m("span").class("reasoning-label").child(lit("Reasoning")),
        ]),
        m("div").class("reasoning-body").child(lit(text)),
    ])
}

/// standalone demo.
pub fn demo() -> Node {
    m("div").class("reasoning-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(card(TEXT)),
    )
}

pub fn register() {
    css::inject("reasoning-card", CSS);
}

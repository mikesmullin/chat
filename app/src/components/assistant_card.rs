//! `assistant card` — the assistant-response block: plain full-width
//! text (the app fills it with rendered markdown). Verbatim app classes.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.assistant-demo { width: min(560px, 100%); }
.assistant-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub const PARA_1: &str =
    "I'll start by mapping the current endpoints, then add a compat shim.";
pub const PARA_2: &str =
    "Found 3 endpoints: login, refresh, revoke. The shim comes first.";

/// the block: one paragraph per string.
pub fn card(paragraphs: &[&str]) -> Node {
    m("div").class("bubble-assistant md-body").kids(
        paragraphs
            .iter()
            .map(|p| m("p").child(lit(p)))
            .collect(),
    )
}

/// standalone demo: two paragraphs.
pub fn demo() -> Node {
    m("div").class("assistant-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(card(&[PARA_1, PARA_2])),
    )
}

pub fn register() {
    css::inject("assistant-card", CSS);
}

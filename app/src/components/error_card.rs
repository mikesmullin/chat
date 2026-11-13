//! `error card` — a transport/agent failure line in the error tone.
//! Verbatim app class.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.error-demo { width: min(560px, 100%); }
.error-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub const TEXT: &str = "Error: dial tcp 127.0.0.1:8763: connection refused";

/// the line.
pub fn card(text: &str) -> Node {
    m("div").class("bubble-error").child(lit(text))
}

/// standalone demo.
pub fn demo() -> Node {
    m("div").class("error-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(card(TEXT)),
    )
}

pub fn register() {
    css::inject("error-card", CSS);
}

//! `user card` — the user-prompt bubble: right-aligned blue bubble in
//! the app, charcoal-retinted here through `--mjs-*` inheritance. Verbatim
//! app class.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.user-demo { width: min(560px, 100%); }
.user-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub const TEXT: &str =
    "Migrate the auth service to the new schema. Keep the old endpoints working during rollout.";

/// the bubble.
pub fn card(text: &str) -> Node {
    m("div").class("bubble-user").child(lit(text))
}

/// standalone demo.
pub fn demo() -> Node {
    m("div").class("user-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(card(TEXT)),
    )
}

pub fn register() {
    css::inject("user-card", CSS);
}

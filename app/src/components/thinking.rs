//! `thinking` — the `Thinking •••` indicator shown at the end of the
//! transcript while the agent works.
//!
//! The dots are **three separate `<span>`s**, which is not cosmetic: the app
//! skin animates `.thinking span` with a staggered `mjs-chat-blink`, so a
//! single `"Thinking •••"` text node renders static dots and the animation
//! silently targets nothing.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
/* demo framing: the indicator alone, centred like every other component */
.thinking-demo { display: flex; }
.thinking-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// the indicator: the word, then one span per dot so each can blink on its
/// own delay (`span:nth-child(2)` / `(3)` in the skin).
pub fn thinking() -> Node {
    m("div").class("thinking").kids(vec![
        lit("Thinking "),
        m("span").child(lit("•")),
        m("span").child(lit("•")),
        m("span").child(lit("•")),
    ])
}

/// standalone demo.
pub fn demo() -> Node {
    m("div").class("thinking-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(thinking()),
    )
}

pub fn register() {
    css::inject("thinking", CSS);
}

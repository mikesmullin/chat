//! `thinking` — the `Thinking •••` indicator shown at the end of the
//! transcript while the agent works. Verbatim app class; the blink
//! animation comes free with it.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
/* demo framing: the indicator alone, centred like every other component */
.thinking-demo { display: flex; }
.thinking-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// the indicator.
pub fn thinking() -> Node {
    m("div").class("thinking").child(lit("Thinking •••"))
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

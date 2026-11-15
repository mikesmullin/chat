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
/* ---- the working indicator ----
   Out of `web/styles.css`, where these were the only rules dressing
   this component — a look kept in a global sheet outlives the markup
   it was written for. Copied unchanged, still scoped under
   `.mjs-chat`, so neither specificity nor the cascade moved with
   them. */
.mjs-chat .thinking {
  align-self: flex-start;
  color: var(--mjs-dim);
  font-size: 13px;
  padding: 4px 0;
}
.mjs-chat .thinking span { animation: mjs-chat-blink 1.2s infinite; }
.mjs-chat .thinking span:nth-child(2) { animation-delay: 0.2s; }
.mjs-chat .thinking span:nth-child(3) { animation-delay: 0.4s; }
@keyframes mjs-chat-blink { 50% { opacity: 0.2; } }

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

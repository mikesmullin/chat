//! `system card` — the system-prompt card. The app renders no system
//! kind today; this is the new member the guide asked for, drawn in the
//! family's shape (header + body) with guide tokens: charcoal plate, dim
//! ink, gear glyph. Token-only colors, no raw hexes.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.system-card {
  width: 100%;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--panel-2);
  color: var(--muted);
  font-size: 13px;
  line-height: 1.5;
  white-space: pre-wrap;
}
.system-card .system-header {
  display: flex; align-items: center; gap: 6px;
  margin-bottom: 6px;
  font-size: 12px; font-weight: 600; letter-spacing: .02em;
  text-transform: uppercase;
}
.system-demo { width: min(560px, 100%); }
.system-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub const TEXT: &str = "You are a build engineer working in /workspace/psicom/chat. Keep answers short.";

/// the card: gear header + prompt body.
pub fn card(text: &str) -> Node {
    m("div").class("system-card").kids(vec![
        m("div").class("system-header").kids(vec![
            Node::icon("gear"),
            m("span").child(lit("System")),
        ]),
        m("div").child(lit(text)),
    ])
}

/// standalone demo.
pub fn demo() -> Node {
    m("div").class("system-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(card(TEXT)),
    )
}

pub fn register() {
    css::inject("system-card", CSS);
}

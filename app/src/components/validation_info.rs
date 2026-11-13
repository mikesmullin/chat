//! `validation info` — the form's verdict line: `error` (pastel-red,
//! e.g. `Name is required`) or `ok` (pastel-green, e.g. `Queued …`).
//! Hidden while the message is empty.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.validation-info {
  font-family: var(--mono);
  font-size: 12px;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.validation-info.error { color: var(--pastel-red); }
.validation-info.ok { color: var(--pastel-green); }
.validation-demo { display: flex; flex-direction: column; gap: 12px; }
"#;

/// a static sample for the demo.
pub fn sample(tone: &str, message: &str) -> Node {
    m("span")
        .class(&format!("validation-info {tone}"))
        .child(lit(message))
}

/// the live line: tone + message are expressions over the form's store.
pub fn live(tone_expr: &str, message_expr: &str) -> Node {
    m("span")
        .class("validation-info")
        .bind("class", tone_expr)
        .show(message_expr)
        .text(message_expr)
}

/// standalone demo: the error sample and the ok sample.
pub fn demo() -> Node {
    m("div").class("validation-demo").kids(vec![
        sample("error", "Name is required"),
        sample("ok", "Queued nightly-docs · 0 9 * * *"),
    ])
}

pub fn register() {
    css::inject("validation-info", CSS);
}

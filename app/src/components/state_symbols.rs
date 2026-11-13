//! The per-row state symbols — the small shapes that mark a row's state.
//! Done = solid indigo dot, idle = hollow dim ring (both pure CSS, so they are
//! `x-ignore` nodes the browser renders); the working state is simply `uiPieRing`
//! (amber), hosted directly in the row.
//!
//! Owns their CSS; injected alongside the other component CSS at boot.

use rustex1_kit::h::{m, Node};
use crate::css;

pub const NAME: &str = "uiStateSymbols";

const CSS: &str = r#"
.state-idle {
  width: 10px; height: 10px; border-radius: 50%;
  border: 2px solid var(--gray-dim); background: transparent;
  flex: 0 0 auto;
}
.state-done {
  width: 8px; height: 8px; border-radius: 50%;
  background: var(--accent); border: 0;
  flex: 0 0 auto;
}
"#;

pub fn register() {
    css::inject(NAME, CSS);
}

/// the solid "done" dot.
pub fn done() -> Node {
    m("span").class("state-done").ignore()
}

/// the hollow "idle" ring.
pub fn idle() -> Node {
    m("span").class("state-idle").ignore()
}

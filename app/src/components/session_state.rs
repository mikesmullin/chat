//! `uiSessionState` — the state symbol as its own unit: the same small shapes
//! that mark agent-menu rows, featured alone at `/storybook/session-state`.
//! done = solid indigo dot, working = amber pie ring (the `uiPieRing` widget),
//! idle = hollow dim-gray ring (all pure CSS).
//!
//! Owns its CSS; `css::inject` puts it in `<head>` on register (updated on
//! every HMR swap). The shared builder `symbol()` is also called by
//! `item_group` rows, so a state symbol renders identically whether it
//! lives alone or inside a composite.

use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

pub const NAME: &str = "uiSessionState";

const CSS: &str = r#"
.state-done {
  width: 8px; height: 8px; border-radius: 50%;
  background: var(--accent); border: 0;
  flex: 0 0 auto;
}
.state-idle {
  width: 10px; height: 10px; border-radius: 50%;
  border: 2px solid var(--gray-dim); background: transparent;
  flex: 0 0 auto;
}
/* feature demo: the three states side by side (done / working / idle) */
.state-demo { display: flex; align-items: center; gap: 36px; }
"#;

/// One state symbol — the single source of truth for the per-row glyph.
pub fn symbol(state: &str) -> Node {
    match state {
        "done" => m("span").class("state-done").ignore(),
        "working" => Node::host("uiPieRing", &[("color", "var(--pastel-amber)")])
            .bind("fraction", "0.68"),
        _ => m("span").class("state-idle").ignore(),
    }
}

/// register the widget + its CSS.
pub fn register() {
    css::inject(NAME, CSS);
    mjs::component(NAME, &def().finish());
}

/// Props (host-provided, with defaults):
/// - `tone` — `done` / `working` / `idle` (default `done`)
fn def() -> Def {
    let mut d = Def::new().named(NAME);
    d.prop_str("tone", "done").tree(
        m("span")
            .kids(vec![
                m("span").class("state-done").ignore().iff("tone === 'done'"),
                Node::host("uiPieRing", &[("color", "var(--pastel-amber)")])
                    .bind("fraction", "0.68")
                    .iff("tone === 'working'"),
                m("span").class("state-idle").ignore().iff("tone === 'idle'"),
            ]),
    );
    d
}

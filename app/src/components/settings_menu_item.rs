//! `uiSettingsMenuItem` — gear glyph + label, borderless, indigo-only hover.
//! The "Settings Menu Item" component: a bare pill button — glyph on the left,
//! label on the right, transparent until hover.
//!
//! Owns its CSS in this file; `css::inject` puts it in the page `<head>` on
//! register (updated on every HMR swap, so edited CSS applies immediately).

use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

const NAME: &str = "uiSettingsMenuItem";

/// The settings button's styles (glyph + label, indigo-only hover).
const CSS: &str = r#"
.btn {
  display: inline-flex; align-items: center; justify-content: center; gap: 10px;
  font: inherit; font-size: 15px; font-weight: 500; color: var(--text);
  background: transparent; border: none; border-radius: 0; padding: 0;
  cursor: pointer; transition: color .13s, transform .13s;
}
.btn:hover { color: var(--accent-soft); }
.btn:active { transform: scale(0.98); }
.btn .ph { font-size: 16px; line-height: 1; color: var(--muted); transition: color .13s; }
.btn:hover .ph { color: var(--accent-soft); }
.btn:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
"#;

pub fn register() {
    css::inject(NAME, CSS);
    mjs::component(NAME, &def().finish());
}

fn def() -> Def {
    let mut d = Def::new().named(NAME);
    d.prop_str("label", "Settings")
        .prop_str("icon", "gear")
        .tree(
            m("button")
                .class("btn btn-settings")
                .attr("type", "button")
                .attr("aria-label", "label")
                .kids(vec![
                    Node::icon_dyn("icon"),
                    m("span").text("label"),
                ]),
        );
    d
}

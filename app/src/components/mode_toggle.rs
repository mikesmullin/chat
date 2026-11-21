//! Shared Simple / Advanced display mode and its toggle.
use rustex1_kit::h::{m, Node};
use rustex1_kit::{js, mjs};
use rustex1_kit::mjs::Def;
use wasm_bindgen::JsValue;

use crate::css;

pub const CSS: &str = r#"
.mode-toggle {
  flex: 0 0 auto; display: inline-flex; align-items: center; justify-content: center;
  width: 32px; height: 32px; padding: 0; border: 0; border-radius: 8px;
  background: transparent; color: var(--gray-dim); font-size: 15px; cursor: pointer;
}
.mode-toggle:hover { color: var(--fg); background: var(--panel-2); }
.mode-toggle.is-advanced { color: var(--accent); background: var(--accent-tint); }
.mode-toggle:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
"#;

pub fn button() -> Node {
    m("button").class("mode-toggle")
        .attr("type", "button")
        .bind("class", "($store.mode.advanced ? 'is-advanced' : '')")
        .bind("title", "($store.mode.advanced ? 'Advanced Mode — switch to Simple Mode' : 'Simple Mode — switch to Advanced Mode')")
        .bind("aria-label", "($store.mode.advanced ? 'Advanced Mode — switch to Simple Mode' : 'Simple Mode — switch to Advanced Mode')")
        .bind("aria-pressed", "($store.mode.advanced ? 'true' : 'false')")
        .on("click", "$store.mode.toggle()")
        .child(Node::icon_dyn("($store.mode.advanced ? 'tree-view' : 'tree')"))
}

fn toggle(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref("mode");
    js::set_bool(&s, "advanced", !js::get_bool(&s, "advanced"));
    mjs::redraw();
    JsValue::UNDEFINED
}

pub fn register() {
    css::inject("mode-toggle", CSS);
    let mut d = Def::new().named("store mode");
    d.bool("advanced", false).method("toggle", toggle);
    mjs::store("mode", &d.finish());
}

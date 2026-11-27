use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::{js, mjs};
use std::cell::RefCell;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsValue;

use crate::css;

const AUTOHIDE_MS: f64 = 60_000.0;

thread_local! {
    static ARMED: RefCell<std::collections::BTreeMap<(String, String), u32>> =
        RefCell::new(std::collections::BTreeMap::new());
}

pub fn note(store: &str, field: &str, text: &str) {
    let s = mjs::store_ref(store);
    js::set_str(&s, field, text);
    let key = (store.to_string(), field.to_string());
    let gen = ARMED.with(|m| {
        let mut m = m.borrow_mut();
        let g = m.entry(key.clone()).or_insert(0);
        *g += 1;
        *g
    });
    let (st, fld) = key;
    let later = Closure::once_into_js(move |_: JsValue| {
        let current =
            ARMED.with(|m| m.borrow().get(&(st.clone(), fld.clone())).copied().unwrap_or(0));
        if current == gen {
            let s = mjs::store_ref(&st);
            js::set_str(&s, &fld, "");
            mjs::redraw();
        }
    });
    js::call(
        &js::global(),
        "setTimeout",
        &[later, JsValue::from_f64(AUTOHIDE_MS)],
    );
}

pub const CSS: &str = r#"
.validation-info {
  display: flex; align-items: center; gap: 8px;
  width: 100%; box-sizing: border-box;
  padding: 8px 10px; border-radius: 8px;
  font-family: var(--mono); font-size: 12px; line-height: 1.5;
  overflow-wrap: anywhere;
}
.validation-info.ok { background: #dcfce7; color: #052e16; }
.validation-info.error { background: #fee2e2; color: #450a0a; }
.validation-info .vi-icon {
  display: inline-flex; align-items: center;
  font-size: 14px; line-height: 1;
}
.validation-demo { display: flex; flex-direction: column; gap: 12px; }
"#;

fn icon_for(tone: &str) -> Node {
    if tone.contains("error") {
        Node::icon("x-circle").class("ph ph-x-circle vi-icon")
    } else {
        Node::icon("check-circle").class("ph ph-check-circle vi-icon")
    }
}

pub fn sample(tone: &str, message: &str) -> Node {
    m("div")
        .class(&format!("validation-info {tone}"))
        .kids(vec![icon_for(tone), m("span").child(lit(message))])
}

pub fn live(tone_expr: &str, message_expr: &str) -> Node {
    m("div")
        .class("validation-info")
        .bind("class", tone_expr)
        .show(message_expr)
        .kids(vec![
            Node::icon_dyn(&format!(
                "(({tone_expr}).includes('error') ? 'x-circle' : 'check-circle')"
            ))
            .class("vi-icon"),
            m("span").text(message_expr),
        ])
}

pub fn demo() -> Node {
    m("div").class("validation-demo").kids(vec![
        sample("error", "Name is required"),
        sample("ok", "Queued nightly-docs · 0 9 * * *"),
    ])
}

pub fn register() {
    css::inject("validation-info", CSS);
}

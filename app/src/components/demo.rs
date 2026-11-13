//! `demo` — the shared demo store for the standalone input routes.
//!
//! The kit widgets only persist through a bound `model` object (an unbound
//! host drops every keystroke), so each input demo binds to `$store.demo`
//! under its own key. One store keeps the demos independent of the
//! automation form's `automationDraft` — typing in a demo never touches the
//! form, and vice versa.

use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};
use wasm_bindgen::JsValue;

pub const MODEL: &str = "$store.demo";

pub fn register() {
    let mut d = Def::new().named("store demo");
    d.str("text1", "")
        .str("text2", "")
        .str("toggle", "false")
        .str("range", "")
        .str("radio", "")
        .str("area", "")
        .str("tags", "")
        .str("field", "")
        .str("composerDraft", "")
        .bool("composerWaiting", false)
        .method("composerSend", send)
        .method("composerStop", stop);
    mjs::store("demo", &d.finish());
}

/// `composerSend`: flip to the waiting (Stop) state.
fn send(_this: JsValue, _args: js_sys::Array) -> JsValue {
    js::set_bool(&mjs::store_ref("demo"), "composerWaiting", true);
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `composerStop`: back to the idle (Send) state.
fn stop(_this: JsValue, _args: js_sys::Array) -> JsValue {
    js::set_bool(&mjs::store_ref("demo"), "composerWaiting", false);
    mjs::redraw();
    JsValue::UNDEFINED
}

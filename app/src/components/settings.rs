//! `settings` — the app's own settings, in the inspector.
//!
//! The third column shows one thing at a time: whatever was selected last.
//! Opening settings therefore closes the agent editor, and selecting an
//! agent closes settings — otherwise the column would stack two unrelated
//! panels and neither would look like the answer to what you clicked.
//!
//! Values are persisted to SQLite, not to browser storage: they describe the
//! project, so they belong with it and should be the same in any browser
//! that opens it.

use wasm_bindgen::JsValue;

use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

pub const STORE: &str = "settings";

/// `load()` — read the current values.
fn load(_this: JsValue, _args: js_sys::Array) -> JsValue {
    rustex1_kit::api::get("/api/settings", None, |v| {
        let s = mjs::store_ref(STORE);
        let values = js::get(&v, "settings");
        js::set(&s, "values", &values);
        // The draft is what the form edits; the values are what is stored.
        // Keeping them apart is what lets Save mean something.
        js::set(&s, "draft", &shallow_copy(&values));
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

/// A copy, so editing the draft does not silently edit the stored values.
fn shallow_copy(v: &JsValue) -> JsValue {
    let out = js::obj();
    let keys = js_sys::Object::keys(&js_sys::Object::from(v.clone()));
    for k in keys.iter() {
        let key = k.as_string().unwrap_or_default();
        js::set_str(&out, &key, &js::get_str(v, &key));
    }
    out.into()
}

/// `openPanel()` — the Settings item in the rail.
///
/// Not named `open`: the store already has an `open` **bool** that `x-show`
/// reads, and a method of the same name collides with it — the field and the
/// function overwrite each other, so `$store.settings.open` is sometimes a
/// boolean and sometimes a function and neither the binding nor the click
/// works reliably.
fn open_panel(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "open", true);
    js::set_str(&s, "saved", "");
    // One panel at a time: the inspector shows what you last selected.
    let agents = mjs::store_ref("agents");
    js::set_bool(&agents, "open", false);
    js::set_str(&agents, "selected", "");
    js::set_bool(&mjs::store_ref("sessionDraft"), "open", false);
    js::set_bool(&mjs::store_ref("ident"), "panelOpen", false);
    // Opening with the column collapsed would look like the click did
    // nothing.
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
    load(JsValue::UNDEFINED, js_sys::Array::new());
    mjs::redraw();
    JsValue::UNDEFINED
}

fn close(_this: JsValue, _args: js_sys::Array) -> JsValue {
    js::set_bool(&mjs::store_ref(STORE), "open", false);
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `save()` — persist the draft.
fn save(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let draft = js::get(&s, "draft");
    js::set_bool(&s, "saving", true);
    mjs::redraw();
    rustex1_kit::api::post_json("/api/settings", &draft, |v| {
        let s = mjs::store_ref(STORE);
        js::set_bool(&s, "saving", false);
        let values = js::get(&v, "settings");
        js::set(&s, "values", &values);
        js::set(&s, "draft", &shallow_copy(&values));
        crate::components::validation_info::note("settings", "saved", "Saved");
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

pub fn register() {
    let mut d = Def::new().named("store settings");
    d.val("values", &js::obj().into())
        .val("draft", &js::obj().into())
        .bool("open", false)
        .bool("saving", false)
        .str("saved", "")
        .method("load", load)
        .method("openPanel", open_panel)
        .method("close", close)
        .method("save", save);
    mjs::store(STORE, &d.finish());
}

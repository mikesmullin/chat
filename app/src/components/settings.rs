use wasm_bindgen::JsValue;

use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

pub const STORE: &str = "settings";

fn load(_this: JsValue, _args: js_sys::Array) -> JsValue {
    rustex1_kit::api::get("/api/settings", None, |v| {
        let s = mjs::store_ref(STORE);
        let values = js::get(&v, "settings");
        js::set(&s, "values", &values);
        js::set(&s, "draft", &shallow_copy(&values));
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

fn shallow_copy(v: &JsValue) -> JsValue {
    let out = js::obj();
    let keys = js_sys::Object::keys(&js_sys::Object::from(v.clone()));
    for k in keys.iter() {
        let key = k.as_string().unwrap_or_default();
        js::set_str(&out, &key, &js::get_str(v, &key));
    }
    out.into()
}

fn open_panel(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "open", true);
    js::set_str(&s, "saved", "");
    let agents = mjs::store_ref("agents");
    js::set_bool(&agents, "open", false);
    js::set_str(&agents, "selected", "");
    js::set_bool(&mjs::store_ref("sessionDraft"), "open", false);
    js::set_bool(&mjs::store_ref("groupDraft"), "open", false);
    js::set_bool(&mjs::store_ref("ident"), "panelOpen", false);
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

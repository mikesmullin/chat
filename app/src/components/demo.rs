use rustex1_kit::mjs::Def;
use rustex1_kit::mjs;
use rustex1_kit::js;
use wasm_bindgen::JsValue;

pub const MODEL: &str = "$store.demo";

fn toggle_bool(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref("demo");
    let field = js::arg_str(&args, 0);
    js::set_bool(&s, &field, !js::get_bool(&s, &field));
    mjs::redraw();
    JsValue::UNDEFINED
}

fn cron_description(this: &JsValue, _args: js_sys::Array) -> JsValue {
    let cron = js::get_str(this, "cron");
    JsValue::from_str(&crate::cron::description_of(&cron))
}

fn cron_next(this: &JsValue, _args: js_sys::Array) -> JsValue {
    let cron = js::get_str(this, "cron");
    JsValue::from_str(&crate::cron::next_line_of(&cron))
}

pub fn register() {
    let mut d = Def::new().named("store demo");
    d.str("text1", "")
        .str("text2", "")
        .str("search", "")
        .str("toggle", "false")
        .str("range", "")
        .str("radio", "")
        .str("area", "")
        .str("tags", "")
        .str("field", "")
        .str("cron", "5 4 * * *")
        .bool("switchOn", true)
        .bool("switchOff", false)
        .method("toggleBool", toggle_bool)
        .method("cronDescription", |this, args| cron_description(&this, args))
        .method("cronNext", |this, args| cron_next(&this, args));
    mjs::store("demo", &d.finish());
}

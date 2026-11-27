use wasm_bindgen::prelude::*;

use rustex1_kit::js;

pub fn inject(name: &str, css: &str) {
    let id = format!("css-{}", name);
    let doc = js::document();
    let head = js::get(&doc, "head");
    let found = js::call(&doc, "getElementById", &[JsValue::from_str(&id)]);
    if found.is_null() {
        let style = js::call(&doc, "createElement", &[JsValue::from_str("style")]);
        js::set_str(&style, "id", &id);
        js::set_str(&style, "textContent", css);
        js::call(&head, "append", &[style.clone()]);
    } else {
        js::set_str(&found, "textContent", css);
    }
}

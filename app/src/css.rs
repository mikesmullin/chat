//! Component-local CSS injection.
//!
//! Each storybook component defines a `CSS: &str` block that lives next to its
//! markup in the same `.rs` file. `inject` places it as a single
//! `<style id="css-<name>">` in `<head>`. The CSS is **not** part of any m-js
//! template, so it is immune to template escaping and redraw — it is appended
//! by the app boot itself, the same way `mjs::component` registers a widget.
//!
//! Idempotent across instances, and **updated** on HMR swap: a rebuild
//! re-runs `register()` with the new CSS text, which overwrites the existing
//! tag so the new rules replace the old ones instead of stacking.

use wasm_bindgen::prelude::*;

use rustex1_kit::js;

/// Append or update `<style id="css-<name>">` with `css`.
pub fn inject(name: &str, css: &str) {
    let id = format!("css-{}", name);
    let doc = js::document();
    let head = js::get(&doc, "head");
    let found = js::call(&doc, "getElementById", &[JsValue::from_str(&id)]);
    if found.is_null() {
        // create once per name, for the life of this boot
        let style = js::call(&doc, "createElement", &[JsValue::from_str("style")]);
        js::set_str(&style, "id", &id);
        js::set_str(&style, "textContent", css);
        js::call(&head, "append", &[style.clone()]);
    } else {
        // rebuild after a hot swap: overwrite, never append.
        js::set_str(&found, "textContent", css);
    }
}

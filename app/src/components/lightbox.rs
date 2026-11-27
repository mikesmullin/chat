use rustex1_kit::h::{m, Node};
use rustex1_kit::{js, mjs};
use wasm_bindgen::JsValue;

use crate::css;

pub const CSS: &str = r#"
.lightbox {
  position: fixed; inset: 0; z-index: 200;
  display: flex; align-items: center; justify-content: center;
  background: rgba(0, 0, 0, .84);
  cursor: zoom-out;
  overscroll-behavior: contain;
}
.lightbox img {
  max-width: 92vw; max-height: 92vh;
  object-fit: contain;
  border-radius: 6px;
  cursor: inherit;
  transition: transform .12s ease-out;
}
.lightbox-close {
  position: absolute; top: 14px; right: 14px;
  width: 32px; height: 32px;
  display: inline-flex; align-items: center; justify-content: center;
  border: 0; border-radius: 8px;
  background: rgba(255, 255, 255, .08); color: var(--text);
  cursor: pointer;
}
.lightbox-close:hover { background: rgba(255, 255, 255, .16); }
"#;

pub fn lightbox() -> Node {
    m("div")
        .class("lightbox")
        .show("$store.lightbox.src")
        .attr("tabindex", "-1")
        .on("click", "$store.lightbox.close()")
        .on("wheel", "$event.preventDefault(); $store.lightbox.zoom($event.deltaY)")
        .kids(vec![
            m("img")
                .bind("src", "$store.lightbox.src")
                .attr("alt", "attachment")
                .bind("style", "('transform: scale(' + $store.lightbox.scale + ')')"),
            m("button")
                .class("lightbox-close")
                .attr("type", "button")
                .attr("title", "Close")
                .attr("aria-label", "Close")
                .child(Node::icon("x")),
        ])
}

pub fn register() {
    css::inject("lightbox", CSS);
    let mut d = mjs::Def::new().named("store lightbox");
    d.str("src", "")
        .num("scale", 1.0)
        .method("open", open)
        .method("close", close)
        .method("zoom", zoom);
    mjs::store("lightbox", &d.finish());
    let token = rustex1_kit::lifecycle::token();
    let callback = wasm_bindgen::closure::Closure::wrap(Box::new(
        move |event: JsValue| {
            if !rustex1_kit::lifecycle::am_current_token(&token) {
                return;
            }
            let key = js::get_str(&event, "key");
            if key == "Escape" && !js::get_str(&mjs::store_ref("lightbox"), "src").is_empty() {
                close(JsValue::UNDEFINED, js_sys::Array::new());
            }
        },
    ) as Box<dyn FnMut(JsValue)>);
    let doc = js::get(&js::global(), "document");
    js::call(
        &doc,
        "addEventListener",
        &[JsValue::from_str("keydown"), callback.into_js_value()],
    );
}

fn open(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref("lightbox");
    let src = js::arg_str(&args, 0);
    if src.is_empty() {
        return JsValue::UNDEFINED;
    }
    js::set_str(&s, "src", &src);
    js::set_num(&s, "scale", 1.0);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn close(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref("lightbox");
    js::set_str(&s, "src", "");
    js::set_num(&s, "scale", 1.0);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn zoom(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref("lightbox");
    let delta = args.get(0).as_f64().unwrap_or(0.0);
    let scale = js::get_num(&s, "scale");
    let factor = if delta > 0.0 { 0.9 } else { 1.1 };
    js::set_num(&s, "scale", (scale * factor).clamp(0.5, 8.0));
    mjs::redraw();
    JsValue::UNDEFINED
}

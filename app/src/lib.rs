use rustex1_kit::{self as kit, App, Runtime};
use wasm_bindgen::prelude::*;

mod api;
mod events;
mod helpers;
mod pages;
mod shortcuts;
mod components;
mod css;

struct Chat;

impl App for Chat {
    fn register_components(&self, _rt: &mut Runtime) {
        components::register();
    }

    fn register_routes(&self, rt: &mut Runtime) {
        pages::register(rt);
    }

    fn boot_toast(&self) -> Option<(String, String)> {
        None
    }

    fn kit_dev_telemetry(&self) -> bool { false }

    fn started(&self, _rt: &mut Runtime) {
        shortcuts::start();
        components::tab::start();
        components::notification_toggle::start();
        components::live::load_models();
        events::start();
    }
}

#[wasm_bindgen]
pub fn boot(m: JsValue, router: JsValue, hooks: JsValue, opts: JsValue) -> Result<(), JsValue> {
    kit::boot(m, router, hooks, opts, &Chat)
}


#[wasm_bindgen(js_name = buildInfo)]
pub fn build_info() -> String {
    kit::build_info()
}

#[wasm_bindgen(js_name = debugState)]
pub fn debug_state() -> String {
    kit::debug_state()
}

#[wasm_bindgen(js_name = dumpTemplate)]
pub fn dump_template(name: &str) -> String {
    kit::template_of(name)
}

#[wasm_bindgen(js_name = lintReport)]
pub fn lint_report() -> String {
    kit::lint_report()
}

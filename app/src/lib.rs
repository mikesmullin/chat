//! chat frontend — the agentic chat UI in Rust, compiled to wasm.
//!
//! The whole client. No JavaScript in this repo except the two bridge files
//! the rustex1 kit provides (`app.mjs` boots the wasm, `hot.mjs` speaks m-js's
//! hot-reload protocol). Components, templates, state, routes and networking
//! are all Rust.
//!
//! What the UI is: the m-js-cmp-chat demo grid — fullscreen chat (tabs +
//! agent + transcript + composer) beside the sidebar rail (same
//! transcript + composer, no tabs/agent/goto/visibility) — backed by the one
//! shared `chat` store. The store keeps the `m-js-cmp-chat` field/method names
//! so the port stays mechanical; the wire is angela's HTTP API
//! (`POST /api/chat` NDJSON) via this server's `/api/*` forward, with an echo
//! fallback when angela is down.

use rustex1_kit::{self as kit, App, Runtime};
use wasm_bindgen::prelude::*;

mod api;
mod helpers;
mod pages;
mod components;
mod css;

/// How often the wall re-reads sessions. The list changes on its own (another
/// tab, a `/new`, a delete), so the poll is the reconciler.
const POLL_MS: f64 = 4000.0;

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

    fn started(&self, rt: &mut Runtime) {
        components::live::load_models();
        rt.every(POLL_MS, || {
            // light poll: the session list changes on its own — another tab,
            // a `/new`, a delete — so the poll is the reconciler.
            components::live::refresh();
        });
    }
}

/// Called by the kit's `app.mjs` — on first load and after every wasm rebuild.
#[wasm_bindgen]
pub fn boot(m: JsValue, router: JsValue, hooks: JsValue, opts: JsValue) -> Result<(), JsValue> {
    kit::boot(m, router, hooks, opts, &Chat)
}

// ── console surface ─────────────────────────────────────────────────────────

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

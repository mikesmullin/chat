use wasm_bindgen::prelude::*;

use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs::{self, Def};
use rustex1_kit::{js, lifecycle};

use crate::css;

pub const NAME: &str = "uiPerfBadge";

const ATTR: &str = "data-perf-read";

const TICK_MS: f64 = 500.0;

thread_local! {
    static TICK_FOR: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

pub const CSS: &str = r#"
.perf-badge {
  flex: 0 0 auto;
  display: inline-flex; align-items: center; gap: 6px;
  height: 30px; padding: 0 9px;
  background: transparent;
  color: var(--muted);
  font-family: var(--sans);
  font-size: 11px;
  letter-spacing: .02em;
  white-space: nowrap;
  cursor: default;
  user-select: none;
}
.perf-badge .ph { font-size: 13px; line-height: 1; }
.perf-badge .perf-read {
  font-variant-numeric: tabular-nums;
  color: var(--mjs-yellow);
}
"#;

pub fn badge() -> Node {
    Node::host(NAME, &[])
}

pub fn register() {
    css::inject("perf-badge", CSS);
    mjs::component(NAME, &def().finish());
}

fn def() -> Def {
    let mut d = Def::new().named(NAME);
    d.tree(
        m("span")
            .class("perf-badge")
            .attr("role", "status")
            .attr("aria-live", "off")
            .attr(
                "title",
                "draw calls",
            )
            .kids(vec![
                Node::icon("paint-bucket"),
                m("span").class("perf-read").attr(ATTR, "").ignore(),
            ]),
    );
    d.method("init", |_this, _args| {
        ensure_tick();
        write_all();
        JsValue::UNDEFINED
    });
    d
}

fn ensure_tick() {
    let token = lifecycle::token();
    if TICK_FOR.with(|t| *t.borrow() == Some(token.clone())) {
        return;
    }
    TICK_FOR.with(|t| *t.borrow_mut() = Some(token.clone()));
    let for_cb = token.clone();
    lifecycle::every(TICK_MS, move || {
        if !lifecycle::am_current_token(&for_cb) {
            return;
        }
        write_all();
    });
}

fn readout() -> String {
    let mj = mjs::m();
    let drawn = js::get_num(&mj, "renderCount");
    let dropped = js::get_num(&mj, "redrawsDropped");
    if !drawn.is_finite() {
        return "n/a".to_string();
    }

    let mut out = format!("{}", drawn as i64);
    if dropped.is_finite() && dropped > 0.0 {
        out.push_str(&format!(" · {} dropped", dropped as i64));
    }
    out
}

fn write_all() {
    let Some(nodes) = js::query_all(&format!("[{ATTR}]")) else {
        return;
    };
    let count = js_sys::Reflect::get(&nodes, &JsValue::from_str("length"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as usize;
    if count == 0 {
        return;
    }
    let text = readout();
    for i in 0..count {
        let node = js_sys::Reflect::get(&nodes, &JsValue::from(i as u32))
            .unwrap_or(JsValue::UNDEFINED);
        if node.is_null() || node.is_undefined() {
            continue;
        }
        if js::get_str(&node, "textContent") != text {
            js::set_text(&node, &text);
        }
    }
}

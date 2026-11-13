//! `dropdown` — a themed select: an icon (tinted on hover) + the current
//! value in mono + a
//! caret, opening a dark menu that matches the page (a native `<select>`
//! popup is OS chrome and cannot be themed, which is the whole reason this
//! exists).
//!
//! ## Why a builder, not an `x-component`
//! m-js does not fire `@click` on elements nested inside a sub-component
//! template (the lesson the tab-bar's close button taught). The menu items
//! *are* clickable, so this ships as a Rust builder — the markup is owned
//! by the host route's tree, m-js binds the handlers directly, and a
//! composite calling `dropdown(...)` gets literally the same markup the
//! standalone route shows.
//!
//! ## State
//! The `dropdown` store holds one key per instance plus a single `open`
//! key naming the dropdown currently showing its menu (so opening one
//! closes the other). `@click.outside` dismisses, via `closeIf(key)` —
//! which no-ops unless *this* dropdown is the open one, so clicking
//! straight from one dropdown to the other does not race: the button's own
//! `@click` fires at the element before the document-level outside
//! listener.

use wasm_bindgen::JsValue;

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::css;

const STORE: &str = "dropdown";

pub const CSS: &str = r#"
/* ---- trigger: chrome, not a boxed control ---------------------- */
.dropdown { position: relative; display: inline-flex; flex: 0 0 auto; }
.dropdown-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--text);
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.4;
  cursor: pointer;
  white-space: nowrap;
  transition: border-color .13s, background-color .13s;
}
.dropdown-btn:hover { background: var(--panel-2); border-color: var(--border); }
.dropdown-btn:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.dropdown.open .dropdown-btn { background: var(--panel-2); border-color: var(--border); }

/* the icon carries the role colour, and only while the dropdown is
   hovered — at rest it is muted, like the caret. */
.dropdown-icon {
  font-size: 14px;
  flex: 0 0 auto;
  color: var(--muted);
  transition: color .13s;
}
.dropdown:hover .dropdown-icon.tone-green { color: var(--pastel-green); }
/* the same indigo the send button and the context ring are painted with */
.dropdown:hover .dropdown-icon.tone-accent { color: var(--accent); }

.dropdown-value { overflow: hidden; text-overflow: ellipsis; }
.dropdown-caret {
  font-size: 11px;
  color: var(--gray-dim);
  flex: 0 0 auto;
  transition: transform .13s;
}
.dropdown.open .dropdown-caret { transform: rotate(180deg); }

/* ---- menu: the page's own dark surface -------------------------- */
.dropdown-menu {
  position: absolute;
  bottom: calc(100% + 6px);
  left: 0;
  z-index: 40;
  min-width: 100%;
  width: max-content;
  max-width: 260px;
  padding: 4px;
  display: flex;
  flex-direction: column;
  gap: 1px;
  background: var(--panel-2);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 10px 28px rgba(0, 0, 0, .55);
}
.dropdown-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 6px 8px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--muted);
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.4;
  text-align: left;
  cursor: pointer;
  white-space: nowrap;
}
.dropdown-item:hover { background: var(--accent-glow); color: var(--text); }
.dropdown-item:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
.dropdown-item.on { color: var(--accent-soft); background: var(--accent-tint); }
.dropdown-item-label { flex: 1 1 auto; }
.dropdown-item-check { font-size: 12px; color: var(--accent); flex: 0 0 auto; }

/* standalone demo: the pair from the design reference, on one line */
.dropdown-demo { display: flex; align-items: center; gap: 18px; }
"#;

/// one menu row: the value, plus a check when it is the current one.
fn item(key: &str, value: &str) -> Node {
    let is_on = format!("$store.{STORE}.{key} === '{value}'");
    m("button")
        .class("dropdown-item")
        .attr("type", "button")
        .bind("class", &format!("({is_on} ? 'on' : '')"))
        .on("click", &format!("$store.{STORE}.pick('{key}', '{value}')"))
        .kids(vec![
            m("span").class("dropdown-item-label").child(lit(value)),
            m("i").class("ph ph-check dropdown-item-check").show(&is_on),
        ])
}

/// a themed dropdown bound to `$store.dropdown.<key>`.
///
/// - `key`     — the store key it reads and writes; also its open-state id
/// - `icon`    — phosphor glyph name (verified present before use)
/// - `tone`    — `green` / `accent`; the icon tints on hover/open only
/// - `extra`   — extra class on the root, for host layout (e.g. `dd-model`)
/// - `options` — the menu, in order
pub fn dropdown(key: &str, icon: &str, tone: &str, extra: &str, options: &[&str]) -> Node {
    let is_open = format!("$store.{STORE}.open === '{key}'");
    m("div")
        .class(&format!("dropdown {extra}"))
        .bind("class", &format!("({is_open} ? 'open' : '')"))
        .on("click.outside", &format!("$store.{STORE}.closeIf('{key}')"))
        .kids(vec![
            m("button")
                .class("dropdown-btn")
                .attr("type", "button")
                .on("click", &format!("$store.{STORE}.toggle('{key}')"))
                .kids(vec![
                    m("i").class(&format!("ph ph-{icon} dropdown-icon tone-{tone}")),
                    m("span")
                        .class("dropdown-value")
                        .text(&format!("$store.{STORE}.{key}")),
                    m("i").class("ph ph-caret-down dropdown-caret"),
                ]),
            m("div")
                .class("dropdown-menu")
                .show(&is_open)
                .kids(options.iter().map(|o| item(key, o)).collect()),
        ])
}

pub const MODELS: &[&str] = &[
    "claude-opus-5-5",
    "claude-sonnet-5-5",
    "claude-haiku-5-5",
];
pub const LEVELS: &[&str] = &["low", "medium", "high", "max"];

/// the model dropdown — cpu glyph, pastel green.
pub fn model(extra: &str) -> Node {
    dropdown("model", "cpu", "green", extra, MODELS)
}

/// the reasoning-level dropdown — brain glyph, the accent indigo.
pub fn level(extra: &str) -> Node {
    dropdown("level", "brain", "accent", extra, LEVELS)
}

/// standalone demo: the pair, as the design reference shows them.
pub fn demo() -> Node {
    m("div")
        .class("dropdown-demo")
        .kids(vec![model(""), level("")])
}

/// `toggle(key)`: open this one, or close it if it is already open.
fn toggle(_this: JsValue, args: js_sys::Array) -> JsValue {
    let key = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    let now = js::get_str(&s, "open");
    js::set_str(&s, "open", if now == key { "" } else { &key });
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `pick(key, value)`: set the value and close the menu.
fn pick(_this: JsValue, args: js_sys::Array) -> JsValue {
    let key = js::arg_str(&args, 0);
    let value = js::arg_str(&args, 1);
    if key.is_empty() {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    js::set_str(&s, &key, &value);
    js::set_str(&s, "open", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `closeIf(key)`: dismiss, but only when *this* dropdown is the open one —
/// so a click that moves straight to the other dropdown cannot close it.
fn close_if(_this: JsValue, args: js_sys::Array) -> JsValue {
    let key = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    if js::get_str(&s, "open") == key {
        js::set_str(&s, "open", "");
        mjs::redraw();
    }
    JsValue::UNDEFINED
}

pub fn register() {
    css::inject("dropdown", CSS);
    let mut d = Def::new().named("store dropdown");
    d.str("open", "")
        .str("model", "claude-opus-5-5")
        .str("level", "high")
        .method("toggle", toggle)
        .method("pick", pick)
        .method("closeIf", close_if);
    mjs::store(STORE, &d.finish());
}

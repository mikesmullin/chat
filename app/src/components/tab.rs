//! `uiTab` — one tab of a tab bar. The tab's **state** lives in the `tabbook`
//! store (same shape as the chat app's own tabs: a list + an `activeId` +
//! `newTab`/`selectTab`/`closeTab` store methods).
//!
//! ## Structure
//! The display (number + state symbol) is the reusable `uiTab` unit. The
//! *actions* (pick / close) live on the `x-for` **row**, as direct children,
//! matching the chat app's tab row exactly — mjs reliably handles `@click`
//! handlers on row-level elements, but not on elements nested inside a
//! sub-component (that nesting is what made the close button inert).
//!
//! - `.tab` row: `@click` → `pickTab(tab.id)`
//! - `.tab-x` button (direct row child): `@click.stop` → `closeTab(tab.id)`
//! - `uiTab` (display host): number (shared counter) + state symbol (shared
//!   session-state unit)

use wasm_bindgen::JsValue;

use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

use rustex1_kit::js;

pub const NAME: &str = "uiTab";

const STORE: &str = "tabbook";

const CSS: &str = r#"
.tab {
  display: inline-flex; align-items: center; justify-content: center; gap: 6px;
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--panel-2);
  color: var(--text);
  font-size: 14px;
  cursor: pointer;
  white-space: nowrap;
  user-select: none;
  transition: border-color .13s, background-color .13s, color .13s;
}
.tab:hover:not(.tab.active) {
  background: var(--panel);
  border-color: var(--line-2);
}
.tab:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
/* active tab — accent border, indigo-tint fill, indigo-soft ink */
.tab.active {
  border-color: var(--accent);
  background: var(--accent-tint);
  color: var(--accent-soft);
}
/* the display unit (uiTab): number + state symbol, fills the tab */
.ui-tab { display: inline-flex; align-items: center; gap: 6px; }
/* the close button: bare glyph, inherits the tab's ink, indigo hover */
.tab-x {
  flex: 0 0 auto;
  border: 0; background: transparent;
  padding: 0 1px;
  border-radius: 4px;
  color: inherit; cursor: pointer;
  transition: color .13s, background-color .13s;
}
.tab-x:hover { color: var(--accent-soft); }
.tab-x:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.tab .ph { font-size: 12px; line-height: 1; flex: 0 0 auto; }
.tab .counter { font-size: 13px; }
/* the pie ring keeps the 14px size of the agents-menu symbol */
.tab .pie-ring { width: 14px; height: 14px; vertical-align: middle; }
/* the live bar (tab-bar): rows + plus button */
.tab-bar { display: inline-flex; align-items: center; gap: 8px; }
.tab-plus {
  height: 30px; padding: 0 8px;
  display: inline-flex; align-items: center;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--panel-2);
  color: var(--muted);
  cursor: pointer;
  transition: color .13s, border-color .13s, background-color .13s;
}
.tab-plus:hover {
  color: var(--accent-soft); border-color: var(--line-2);
  background: var(--panel);
}
.tab-plus:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.tab-plus .ph { font-size: 14px; line-height: 1; }
/* demo chrome: the static row of three on /storybook/tab */
.tab-demo { display: flex; gap: 8px; }
"#;

/// One tab DISPLAY host for the STATIC feature demo. String `state` is wrapped
/// in single quotes so m-js reads a JS string literal.
pub fn tab_display(n: i32, state: &str) -> Node {
    Node::host("uiTab", &[])
        .attr("class", "ui-tab")
        .bind("n", &format!("{}", n))
        .bind("state", &format!("'{}'", state))
}

/// Feature demo: three static tabs — one active (n=1), two not (n=2, n=3).
/// A design sample: the close button is inert (no `@click` handler bound).
pub fn demo() -> Node {
    m("div")
        .class("tab-demo")
        .kids(vec![
            m("span")
                .class("tab active")
                .attr("aria-pressed", "true")
                .child(tab_display(1, "done")),
            m("span")
                .class("tab")
                .attr("aria-pressed", "false")
                .child(tab_display(2, "working")),
            m("span")
                .class("tab")
                .attr("aria-pressed", "false")
                .child(tab_display(3, "idle")),
        ])
}

/// The live bar: `x-for` over the store's `tabs`. The row is a direct element
/// (no sub-component boundary) so its `@click` and the child close button's
/// `@click.stop` both reliably fire — the chat-app pattern.
pub fn tab_bar() -> Node {
    let st = format!("$store.{}", STORE); // e.g. `$store.tabbook`
    m("div")
        .class("tab-bar")
        .kids(vec![
            m("span")
                .for_each(&format!("(tab, i) in {}.tabs", st))
                .key("tab.id")
                .attr("class", "tab")
                .bind("class", &format!("{{ active: tab.id === {}.activeId }}", st))
                .on("click", &format!("{}.pickTab(tab.id)", st))
                .kids(vec![
                    Node::host("uiTab", &[])
                        .attr("class", "ui-tab")
                        .bind("n", "(i + 1)")
                        // m-js reads `tab.state` from the x-for row.
                        .bind("state", "tab.state"),
                    m("button")
                        .class("tab-x")
                        .attr("type", "button")
                        .attr("aria-label", &format!("close tab {}", st))
                        .on("click.stop", &format!("{}.closeTab(tab.id)", st))
                        .child(Node::icon("x")),
                ]),
            m("button")
                .class("tab-plus")
                .attr("type", "button")
                .attr("aria-label", "new tab")
                .on("click", &format!("{}.addTab()", st))
                .child(Node::icon("plus")),
        ])
}

/// register the widget + the `tabbook` store.
pub fn register() {
    css::inject(NAME, CSS);
    mjs::component(NAME, &def().finish());
    store_register();
}

/// Props (host-provided, with defaults):
/// - `n`   — 1-indexed number shown in the tab (default 1)
/// - `state` — `done` / `working` / `idle` (default `done`)
fn def() -> Def {
    let mut d = Def::new().named(NAME);
    d.prop_num("n", 1.0)
        .prop_str("state", "done")
        .tree(
            m("span")
                .class("ui-tab")
                .kids(vec![
                    // the number is the shared counter unit.
                    Node::host("uiCounter", &[]).bind("value", "n"),
                    // the state symbol is the shared session-state unit.
                    Node::host("uiSessionState", &[]).bind("tone", "state"),
                ]),
        );
    d
}

/// seed: 3 tabs (ids 1/2/3, done/working/idle), active = 1.
fn seed_tabs() -> JsValue {
    let a = js_sys::Array::new();
    for (id, state) in [(1.0, "done"), (2.0, "working"), (3.0, "idle")] {
        let t = js::obj();
        js::set_num(&t, "id", id);
        js::set_str(&t, "state", state);
        a.push(&t.into());
    }
    a.into()
}

/// `addTab`: append the next tab (id = length + 1), cycle its state,
/// make it active. Mirrors `store.newTab`.
fn add_tab_store(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let tabs = js_sys::Array::from(&js::get(&s, "tabs"));
    // New id = highest existing id + 1 (not length + 1), so closing a tab
    // never re-uses a stale id.
    let mut max_id = 0.0;
    for t in tabs.iter() {
        let id = js::get_num(&t, "id");
        if id > max_id {
            max_id = id;
        }
    }
    let n = max_id + 1.0;
    let states = ["done", "working", "idle"];
    let state = states[((n - 1.0) as usize) % 3];
    let t = js::obj();
    js::set_num(&t, "id", n);
    js::set_str(&t, "state", state);
    let pushed = tabs.clone();
    pushed.push(&t.into());
    js::set(&s, "tabs", &pushed.clone().into());
    js::set_num(&s, "activeId", n);
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `pickTab`: set the active id.
fn pick_tab_store(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg(&args, 0).as_f64().unwrap_or(-1.0);
    if id >= 0.0 {
        js::set_num(&mjs::store_ref(STORE), "activeId", id);
        mjs::redraw();
    }
    JsValue::UNDEFINED
}

/// `closeTab`: drop the tab with `id`; if it was active, the first survivor
/// becomes active.
fn close_tab_store(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg(&args, 0).as_f64().unwrap_or(-1.0);
    let s = mjs::store_ref(STORE);
    let tabs = js_sys::Array::from(&js::get(&s, "tabs"));
    let mut kept = js_sys::Array::new();
    for t in tabs.iter() {
        if js::get_num(&t, "id") != id {
            kept.push(&t);
        }
    }
    js::set(&s, "tabs", &kept.clone().into());
    if js::get_num(&s, "activeId") == id {
        if kept.length() > 0 {
            let first = kept.get(0);
            js::set_num(&s, "activeId", js::get_num(&first, "id"));
        }
    }
    mjs::redraw();
    JsValue::UNDEFINED
}

/// register the `tabbook` store.
fn store_register() {
    let mut d = Def::new().named("store tabbook");
    d.val("tabs", &seed_tabs())
        .num("activeId", 1.0)
        .method("addTab", add_tab_store)
        .method("pickTab", pick_tab_store)
        .method("closeTab", close_tab_store)
        .finish();
    mjs::store(STORE, &d.finish());
}

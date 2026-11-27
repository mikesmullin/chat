use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, lifecycle};
use wasm_bindgen::prelude::*;

use crate::css;

pub const NAME: &str = "uiTab";


const CSS: &str = r#"
.mjs-chat .tabs {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: 1;
  overflow-x: auto;
  min-width: 0;
}

.mjs-chat .tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border-radius: 8px;
  border: 1px solid var(--mjs-line);
  background: var(--mjs-panel-2);
  color: var(--mjs-ink);
  font-size: 13px;
  cursor: pointer;
  white-space: nowrap;
}
.mjs-chat .tab.active {
  border-color: var(--mjs-accent);
  background: color-mix(in srgb, var(--mjs-accent) 15%, transparent);
  color: var(--mjs-accent-2);
}
.mjs-chat .tab .x {
  opacity: 0.6;
  border: 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
  padding: 0 2px;
}

.tab {
  display: inline-flex; align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--panel-2);
  color: var(--text);
  font-size: 13px;
  line-height: 1.3;
  cursor: pointer;
  white-space: nowrap;
  user-select: none;
  min-width: fit-content;
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
.tab.active {
  border-color: var(--accent);
  background: var(--accent-tint);
  color: var(--accent-soft);
}
.ui-tab {
  display: flex; align-items: center;
  gap: 6px;
  width: 100%;
  min-height: 1.3em;
}
.ui-tab .ui-tab-title {
  flex: 1 1 0;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tab.is-preview .ui-tab-title { font-style: italic; }
.tab-x {
  flex: 0 0 auto;
  border: 0; background: transparent;
  padding: 0 2px;
  border-radius: 4px;
  color: inherit; cursor: pointer;
  line-height: 1.3;
  transition: color .13s, background-color .13s;
}
.tab-x:hover { color: var(--accent-soft); }
.tab-x:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.tab-bar { display: inline-flex; align-items: center; gap: 8px; }
.tab-plus {
  height: 30px; padding: 0 8px;
  display: inline-flex; align-items: center;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--panel-2);
  color: var(--muted);
  cursor: pointer;
  line-height: 1.3;
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
.tab-plus .ph { font-size: 14px; line-height: 1.3; }
.tab-demo { display: flex; gap: 8px; }
"#;

pub fn tab_display(icon: &str, label: &str) -> Node {
    Node::host_bound("uiTab", &[
        ("class", "ui-tab"),
        ("icon", &format!("'{}'", icon)),
        ("label", &format!("'{}'", label)),
    ])
}

pub fn demo() -> Node {
    let close = m("button")
        .class("tab-x")
        .attr("type", "button")
        .child(Node::icon("x"));
    m("div")
        .class("tab-demo")
        .kids(vec![
            m("span")
                .class("tab active")
                .attr("aria-pressed", "true")
                .child(tab_display("chat-circle", "1"))
                .child(close.clone()),
            m("span")
                .class("tab")
                .attr("aria-pressed", "false")
                .child(tab_display("chat-circle", "2"))
                .child(close.clone()),
            m("span")
                .class("tab")
                .attr("aria-pressed", "false")
                .child(tab_display("chat-circle", "Idle"))
                .child(close.clone()),
        ])
}

pub fn register() {
    css::inject(NAME, CSS);
    mjs::component(NAME, &def().finish());
}

pub fn start() {
    let global = js::global();
    let doc = js::get(&global, "document");
    let old = js::get(&global, "__chatTabFocusListener");
    if !js::is_missing(&old) { js::call(&doc, "removeEventListener", &[JsValue::from_str("focusin"), old]); }
    let token = lifecycle::token();
    let callback = Closure::wrap(Box::new(move |event: JsValue| {
        if !lifecycle::am_current_token(&token) { return; }
        let target = js::get(&event, "target");
        if !js::call(&target, "matches", &[JsValue::from_str(".tab[data-session-id]")]).as_bool().unwrap_or(false) { return; }
        let id = js::call_string(&target, "getAttribute", &[JsValue::from_str("data-session-id")]);
        let expr = js::call_string(&target, "getAttribute", &[JsValue::from_str("data-tab-store")]);
        if let Some(name) = expr.strip_prefix("$store.") {
            js::call(&mjs::store_ref(name), "markSessionRead", &[JsValue::from_str(&id)]);
        }
    }) as Box<dyn FnMut(JsValue)>).into_js_value();
    js::call(&doc, "addEventListener", &[JsValue::from_str("focusin"), callback.clone()]);
    js::set(&global, "__chatTabFocusListener", &callback);
}

fn def() -> Def {
    let mut d = Def::new().named(NAME);
    d.prop_str("icon", "")
        .prop_str("label", "")
        .prop_str("title", "")
        .tree(
            m("span")
                .class("ui-tab")
                .kids(vec![
                    Node::icon_dyn("(icon === '' ? 'chat-circle' : icon)")
                        .class("ph ptree-icon"),
                    m("span")
                        .class("ui-tab-title")
                        .text("label")
                        .bind("title", "title"),
                ]),
        );
    d
}

pub fn tab_bar(st: &str) -> Node {
    m("div").class("tab-bar").kids(vec![
        m("span")
            .for_each(&format!("s in {st}.tabs"))
            .key("s.id")
            .attr("class", "tab")
            .attr("tabindex", "0")
            .attr("role", "button")
            .bind("class", &format!("{{ active: s.id === {st}.activeId, 'is-preview': s.id === {st}.previewId }}"))
            .attr("data-tab-store", st)
            .bind("data-session-id", "s.id")
            .on("click", &format!("{st}.markSessionRead(s.id); {st}.openSession(s.id)"))
            .on("keydown.enter", &format!("{st}.markSessionRead(s.id); {st}.openSession(s.id)"))
            .on("dblclick", &format!("{st}.pinTab(s.id)"))
            .bind("title", "s.title || s.id")
            .kids(vec![
                Node::host_bound(
                    "uiTab",
                    &[
                        ("class", "ui-tab"),
                        ("icon", "'chat-circle'"),
                        ("label", "(s.ordinal || '?') + ''"),
                        ("title", "(s.title || s.id)"),
                    ],
                ),
                m("button")
                    .class("tab-x")
                    .attr("type", "button")
                    .attr("aria-label", "close tab")
                    .attr("title", "Close this tab (the session is kept)")
                    .on("click.stop", &format!("{st}.closeTab(s.id)"))
                    .child(Node::icon("x")),
            ]),
        m("button")
            .class("tab-plus")
            .attr("type", "button")
            .attr("aria-label", "new session")
            .show("$store.ident.can('session.create')")
            .on("click", &format!("{st}.newSession()"))
            .child(Node::icon("plus")),
    ])
}

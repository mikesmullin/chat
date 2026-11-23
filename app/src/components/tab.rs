//! `uiTab` — one tab of a tab bar. The tab's **state** lives in the store
//! store (same shape as the chat app's own tabs: a list + an `activeId` +
//! `addTab`/`pickTab`/`closeTab` store methods).
//!
//! ## Structure
//! The display (icon + title + state symbol) is the reusable `uiTab` unit:
//! a robot icon by default (the same glyph the agents-menu rows use), a
//! title that may be a number or text, and the shared `uiSessionState`
//! symbol. The *actions* (pick / close) live on the `x-for` **row**, as
//! direct children, matching the chat app's tab row exactly — mjs reliably
//! handles `@click` handlers on row-level elements, but not on elements
//! nested inside a sub-component (that nesting is what made the close
//! button inert).
//!
//! - `.tab` row: `@click` → `pickTab(tab.id)`
//! - `.tab-x` button (direct row child): `@click.stop` → `closeTab(tab.id)`
//! - `uiTab` (display host): icon + title + `uiSessionState` symbol


use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, lifecycle};
use wasm_bindgen::prelude::*;

use crate::components::session_state;
use crate::css;

pub const NAME: &str = "uiTab";


const CSS: &str = r#"
/* ---- the tab strip ----
   Out of `web/styles.css` with the demo page it used to share. */
.mjs-chat .tabs {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: 1;
  overflow-x: auto;
  min-width: 0;
}

/* ---- the tab chip ----
   Out of `web/styles.css`, where these were the only rules dressing
   this component — a look kept in a global sheet outlives the markup
   it was written for. Copied unchanged, still scoped under
   `.mjs-chat`, so neither specificity nor the cascade moved with
   them. */
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
/* active tab — accent border, indigo-tint fill, indigo-soft ink */
.tab.active {
  border-color: var(--accent);
  background: var(--accent-tint);
  color: var(--accent-soft);
}
/* the display unit: icon + title + state symbol. A true flex row — the
   title stretches, the icon and symbol are fixed-size, so the ring (14x14
   square, flex 0 0 auto via the global .pie-ring rules) never squishes in
   a tight row. */
.ui-tab {
  display: flex; align-items: center;
  gap: 6px;
  width: 100%;
  min-height: 1.3em;
}
/* the icon: the only <i> inside a tab, styled here (robot glyph by default,
   named phosphor icon via the `icon` prop) */
.ui-tab i {
  flex: 0 0 auto;
  font-size: 14px; line-height: 1.3;
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
/* the close button: bare glyph, inherits the tab's ink, indigo hover */
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
/* demo chrome: the static row of three on /storybook/tab */
.tab-demo { display: flex; gap: 8px; }
"#;

/// One tab DISPLAY host for the STATIC feature demo. Strings are wrapped in
/// single quotes so m-js reads a JS string literal.
pub fn tab_display(icon: &str, label: &str, tone: &str) -> Node {
    Node::host_bound("uiTab", &[
        ("class", "ui-tab"),
        ("icon", &format!("'{}'", icon)),
        ("label", &format!("'{}'", label)),
        ("tone", &format!("'{}'", tone)),
    ])
}

/// Feature demo: three static tabs — one active numeric, one numeric
/// working, one text-titled idle; each shows the full composition (icon +
/// title + symbol + close), the close button being inert in this static
/// sample (no store).
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
                .child(tab_display("robot", "1", "done"))
                .child(close.clone()),
            m("span")
                .class("tab")
                .attr("aria-pressed", "false")
                .child(tab_display("robot", "2", "working"))
                .child(close.clone()),
            m("span")
                .class("tab")
                .attr("aria-pressed", "false")
                .child(tab_display("robot", "Idle", "idle"))
                .child(close.clone()),
        ])
}

/// The live bar: `x-for` over the store's `tabs`. The row is a direct element
/// (no sub-component boundary) so its `@click` and the child close button's
/// `@click.stop` both reliably fire — the chat-app pattern.
/// register the widget.
pub fn register() {
    css::inject(NAME, CSS);
    mjs::component(NAME, &def().finish());
}

/// Focus events are delegated because m-js does not bind them on these rows.
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

/// Props (host-provided, with defaults). Prop names are HTML attributes,
/// so they are single lowercase words — a camelCase name like `titleHint`
/// is lowercased by the parser and silently never reaches the component.
/// - `icon`  — phosphor glyph name; empty string means the robot icon (default "")
/// - `label` — the tab's short text; a number or text (default "")
/// - `title` — the label's tooltip: the full session title behind the
///   short label (default "")
/// - `tone`  — `done` / `working` / `idle` (default `done`)
/// - `fraction` — passed to the state symbol: the ring's context-window
///   fill (default 0.68, the symbol's showcase fill)
/// - `hint`  — passed to the state symbol as its tooltip: the status and
///   context counts of `session_state::hint_of` (default "")
fn def() -> Def {
    let mut d = Def::new().named(NAME);
    d.prop_str("icon", "")
        .prop_str("label", "")
        .prop_str("title", "")
        .prop_str("tone", "done")
        .prop_num("fraction", 0.68)
        .prop_str("hint", "")
        .tree(
            m("span")
                .class("ui-tab")
                .kids(vec![
                    // icon: robot by default, otherwise a named phosphor glyph.
                    // Parens are required: without them the ternary binds looser
                    // than `+`/`===` and m-js evaluates it to just the glyph.
                    Node::icon_dyn("(icon === '' ? 'robot' : icon)")
                        .class("ui-tab-icon"),
                    // label: number or text, ellipsised when it overflows;
                    // its tooltip is the full title it cannot show.
                    m("span")
                        .class("ui-tab-title")
                        .text("label")
                        .bind("title", "title"),
                    // state symbol: the shared `uiSessionState` unit, so a tab's
                    // symbol is literally the same markup featured alone at
                    // /storybook/session-state.
                    Node::host("uiSessionState", &[])
                        .bind("tone", "tone")
                        .bind("fraction", "fraction")
                        .bind("title", "hint"),
                ]),
        );
    d
}

/// The tab strip, over whichever store holds the open tabs.
///
/// One definition: `/` passes the live store, the design guide passes the
/// samples, where opening a tab really does switch it.
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
                        ("icon", "'robot'"),
                        // The tab shows its session's stable 1-based number,
                        // not the prompt: a title is far too long for a tab,
                        // and the number is what the user counts by.
                        ("label", "(s.ordinal || '?') + ''"),
                        // …and its tooltip is the full session title the
                        // short label cannot show.
                        ("title", "(s.title || s.id)"),
                        // The conversation's state, by the one rule the
                        // project tree uses too.
                        ("tone", &session_state::tone_of("s")),
                        // The ring fills with the conversation's context
                        // window, by the one rule as well.
                        ("fraction", &session_state::fraction_of("s")),
                        // The symbol's tooltip is the status and context
                        // counts, by the one rule the tree uses.
                        ("hint", &session_state::hint_of("s")),
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
            // Creating sessions is a capability (PLAN44 Layer 1): no
            // `session.create`, no button.
            .show("$store.ident.can('session.create')")
            .on("click", &format!("{st}.newSession()"))
            .child(Node::icon("plus")),
    ])
}

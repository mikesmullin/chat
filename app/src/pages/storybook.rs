//! The storybook — a Design Guide at `/storybook`, rendered entirely in Rust
//! (the static `storybook.html` is retired). Two columns: a component rail on
//! the left, the current component centred in the content area on the right.
//!
//! Routes:
//!   /storybook            -> Settings Menu Item (home)
//!   /storybook/counter    -> Counter
//!   /storybook/nav-rail   -> Left Sidebar Navigation (standalone demo)
//!   /storybook/pie-ring   -> Pie Ring Graph
//!   /storybook/reltime    -> Relative Timestamp (live — the kit `uiRelTime`)
//!   /storybook/session-state -> Session State (done / working / idle symbols)
//!   /storybook/item-group -> Item Group
//!   /storybook/agents     -> Agents Menu
//!   /storybook/sandboxes  -> Sandboxes Menu
//!   /storybook/tab        -> Tab (single unit, three static samples)
//!   /storybook/tab-bar    -> Tab Bar (live: add / pick / close)
//!
//! The rail is rebuilt on every page (each route re-renders the whole `#app`
//! tree), highlighting whichever entry matches the current component.

use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs::{self, Def};
use rustex1_kit::{js, lifecycle, Runtime};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use crate::components::{agents_menu, item_group, nav_rail, relative_timestamp, sandboxes_menu, tab};

/// the two-column shell: rail (left) + centred content (right).
fn shell(active: &str, content: Node) -> Node {
    m("div")
        .attr("id", "sb")
        .class("sb")
        .kids(vec![
            nav_rail::page_rail(active),
            m("main")
                .class("sb-content")
                .child(m("section").class("feature").child(content)),
        ])
}

/// Build one storybook route's scope: the shell tree plus the `toggleRail`
/// method the rail's toggle button calls (`@click="toggleRail($event)"`).
/// The method lives on the route scope (not in the template) so the button's
/// expression is a trivial call — the same shape m-js itself uses for its
/// 404 template (`@click="goHome"` + a `goHome` method on the route object).
fn route_def(name: &str, active: &str, content: Node) -> JsValue {
    let mut d = Def::new().named(name);
    d.tree(shell(active, content));
    d.method("toggleRail", toggle_rail);
    d.finish()
}

/// `toggleRail(event)` — collapse/expand the sidebar. Reads
/// `event.currentTarget` (the button), so no `$el` magic is needed.
/// Direct DOM class toggles: no store, no redraw, nothing to re-render.
fn toggle_rail(_this: JsValue, args: js_sys::Array) -> JsValue {
    let ev = js::arg(&args, 0);
    let btn = js::get(&ev, "currentTarget");
    if js::is_missing(&btn) {
        return JsValue::UNDEFINED;
    }
    let rail = js::call(
        &btn,
        "closest",
        &[JsValue::from_str(".sb-rail,.nav-demo")],
    );
    if js::is_missing(&rail) {
        return JsValue::UNDEFINED;
    }
    let cls = js::get(&rail, "classList");
    let toggle = |el_cls: &JsValue| {
        js::call(el_cls, "toggle", &[JsValue::from_str("collapsed")]);
    };
    let is_collapsed = |el_cls: &JsValue| {
        js::call(el_cls, "contains", &[JsValue::from_str("collapsed")])
            .as_bool()
            .unwrap_or(false)
    };
    let set_expanded = |collapsed: bool| {
        js::call(
            &btn,
            "setAttribute",
            &[
                JsValue::from_str("aria-expanded"),
                JsValue::from_str(if collapsed { "false" } else { "true" }),
            ],
        );
    };
    let is_live = js::call(&cls, "contains", &[JsValue::from_str("sb-rail")])
        .as_bool()
        .unwrap_or(false);
    if is_live {
        // live rail: collapse `#sb`'s grid track and the rail together.
        if let Some(page) = js::query("#sb") {
            let page_cls = js::get(&page, "classList");
            toggle(&page_cls);
            set_expanded(is_collapsed(&page_cls));
        }
        toggle(&cls);
    } else {
        // standalone demo panel: collapse its own width.
        toggle(&cls);
        set_expanded(is_collapsed(&cls));
    }
    JsValue::UNDEFINED
}

/// register all storybook routes (called from `pages::register`).
/// Also installs the SPA link interception (once per boot): plain
/// left-clicks on `a[href^="/storybook"]` route client-side via
/// `Router.set` instead of full-reloading. Document-level delegation, so
/// re-renders can't drop it; scoped to `/storybook*` so the chat app's
/// own navigation is untouched. Modifier/middle clicks pass through to
/// the browser (new-tab, copy-link all keep working). Stale boots
/// self-disable via the lifecycle token (same pattern as the reltime tick).
pub fn register(rt: &mut Runtime) {
    register_spa_nav();
    // home: the settings menu item widget (gear + "Settings").
    rt.route("/storybook", "Settings Menu Item", |_| {
        route_def(
            "route /storybook",
            "Settings Menu Item",
            Node::host("uiSettingsMenuItem", &[]),
        )
    });

    // counter: four inline counter instances.
    rt.route("/storybook/counter", "Counter", |_| {
        let content = m("div")
            .class("counter-demo")
            .kids(vec![
                Node::host("uiCounter", &[]).bind("value", "1"),
                Node::host("uiCounter", &[]).bind("value", "3"),
                Node::host("uiCounter", &[]).bind("value", "24"),
                Node::host("uiCounter", &[]).bind("value", "128"),
            ]);
        route_def("route /storybook/counter", "Counter", content)
    });

    // standalone fixed-size rail.
    rt.route("/storybook/nav-rail", "Left Sidebar Navigation", |_| {
        route_def(
            "route /storybook/nav-rail",
            "Left Sidebar Navigation",
            nav_rail::nav_rail_demo(),
        )
    });

    // pie ring: a 68% accent ring.
    rt.route("/storybook/pie-ring", "Pie Ring Graph", |_| {
        let ring = m("div")
            .class("demo-ring")
            .child(
                Node::host("uiPieRing", &[("color", "var(--accent)")])
                    .bind("fraction", "0.68"),
            );
        route_def("route /storybook/pie-ring", "Pie Ring Graph", ring)
    });

    // relative timestamp: the kit's live uiRelTime, shown for a set of offsets.
    rt.route("/storybook/reltime", "Relative Timestamp", |_| {
        route_def(
            "route /storybook/reltime",
            "Relative Timestamp",
            relative_timestamp::demo(),
        )
    });

    // a single "Done" item group, two rows.
    rt.route("/storybook/item-group", "Item Group", |_| {
        let group = item_group::item_group(
            "done",
            "Done",
            "2",
            &[
                (
                    "Eve".into(),
                    "Finished the onboarding copy and tone guide".into(),
                    "1h".into(),
                ),
                ("Dan".into(), "Fixed the session timeout bug".into(), "45m".into()),
            ],
        );
        route_def("route /storybook/item-group", "Item Group", group)
    });

    // the full agents menu (done / working / idle groups).
    rt.route("/storybook/agents", "Agents Menu", |_| {
        route_def(
            "route /storybook/agents",
            "Agents Menu",
            agents_menu::agents_menu(),
        )
    });

    // the sandboxes menu (one group per host computer).
    rt.route("/storybook/sandboxes", "Sandboxes Menu", |_| {
        route_def(
            "route /storybook/sandboxes",
            "Sandboxes Menu",
            sandboxes_menu::sandboxes_menu(),
        )
    });

    // the state symbol unit — done / working / idle, featured alone.
    rt.route("/storybook/session-state", "Session State", |_| {
        route_def(
            "route /storybook/session-state",
            "Session State",
            m("div")
                .class("state-demo")
                .kids(vec![
                    // default `tone = "done"`.
                    Node::host("uiSessionState", &[]),
                    Node::host("uiSessionState", &[]).bind("tone", "'working'"),
                    Node::host("uiSessionState", &[]).bind("tone", "'idle'"),
                ]),
        )
    });
    // a single tab unit, featured alone: three static samples
    // (n=1 active, n=2 working, n=3 idle).
    rt.route("/storybook/tab", "Tab", |_| {
        route_def("route /storybook/tab", "Tab", tab::demo())
    });
    // the live tab bar: tabs close, the plus adds, click sets the active tab.
    rt.route("/storybook/tab-bar", "Tab Bar", |_| {
        route_def("route /storybook/tab-bar", "Tab Bar", tab::tab_bar())
    });
}

/// One document-level click listener per boot: upgrade storybook rail
/// links to client-side routing. The rail renders plain `<a href>` (so
/// no-JS, new-tab, and copy-link keep working); this listener only
/// upgrades unmodified left-clicks.
fn register_spa_nav() {
    let token = lifecycle::token();
    let cb = Closure::wrap(Box::new(move |ev: JsValue| {
        // a rebuilt (hot-swapped) boot registers its own listener;
        // this one belongs to a dead boot, stay out of the way.
        if !lifecycle::am_current_token(&token) {
            return;
        }
        // only unmodified left-clicks — the rest belong to the browser.
        if js::get_num(&ev, "button") != 0.0
            || js::get_bool(&ev, "ctrlKey")
            || js::get_bool(&ev, "metaKey")
            || js::get_bool(&ev, "shiftKey")
            || js::get_bool(&ev, "altKey")
        {
            return;
        }
        let target = js::get(&ev, "target");
        if js::is_missing(&target) {
            return;
        }
        let anchor = js::call(&target, "closest", &[JsValue::from_str("a[href]")]);
        if js::is_missing(&anchor) {
            return;
        }
        let href = js::call_string(&anchor, "getAttribute", &[JsValue::from_str("href")]);
        if !href.starts_with("/storybook") {
            return;
        }
        js::call(&ev, "preventDefault", &[]);
        mjs::router_set(&href);
    }) as Box<dyn FnMut(JsValue)>);
    js::call(
        &js::document(),
        "addEventListener",
        &[
            JsValue::from_str("click"),
            cb.as_ref().unchecked_ref::<JsValue>().clone(),
        ],
    );
    cb.forget();
}


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
//!   /storybook/automation -> New Automation (form of fields + validation)
//!   /storybook/field      -> Field (label + control + hint)
//!   /storybook/label      -> Label
//!   /storybook/hint       -> Hint Text
//!   /storybook/text-input -> Text Input (bare kit uiText)
//!   /storybook/toggle     -> Toggle Button (bare kit uiToggle)
//!   /storybook/range      -> Range Input (bare kit uiNumber stepper)
//!   /storybook/radio      -> Radio Input (bare kit uiChoice)
//!   /storybook/text-area  -> Text Area (bare kit uiArea)
//!   /storybook/tags       -> Tag Input (bare kit uiChips)
//!   /storybook/validation -> Validation Info (error / ok)
//!   /storybook/session-history -> Session History (past sessions)
//!   /storybook/session   -> Session (transcript of message cards)
//!   /storybook/session-panel -> Session Panel (transcript + composer)
//!   /storybook/composer  -> Composer (draft + row: dropdowns, ctx pie, send)
//!   /storybook/dropdown  -> Dropdown (themed select: icon + value + caret)
//!   /storybook/system-card -> System Card (system prompt)
//!   /storybook/user-card -> User Card (user prompt bubble)
//!   /storybook/assistant-card -> Assistant Card (response block)
//!   /storybook/reasoning-card -> Reasoning Card (thinking block)
//!   /storybook/fold-card -> Fold Card (the collapsible evidence primitive)
//!   /storybook/prompt-card -> Prompt Card (system + user prompt as sent)
//!   /storybook/decision-card -> Decision Card (why an agent spoke)
//!   /storybook/tool-call-card -> Tool Call Card
//!   /storybook/tool-response-card -> Tool Response Card
//!   /storybook/error-card -> Error Card
//!   /storybook/approval-card -> Approval Card (allow / always / deny)
//!   /storybook/thinking -> Thinking (working indicator)
//!   /storybook/task-list -> Task List (todo panel)
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
use crate::components::{
    agents_menu, approval_card, assistant_card, automation_form, composer, decision_card,
    samples,
    dropdown, error_card, fold_card, prompt_card,
    field, hint_text, item_group, label, nav_rail, radio_input, range_input,
    reasoning_card, relative_timestamp, sandboxes_menu, session, session_history,
    session_panel, system_card, tab, tag_input, task_list, text_area, text_input, thinking, toggle_button, tool_call_card, tool_response_card,
    user_card, validation_info,
};

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

    // the new-automation form (fields + validation, bound to the draft store).
    rt.route("/storybook/automation", "New Automation", |_| {
        route_def(
            "route /storybook/automation",
            "New Automation",
            automation_form::automation_form(),
        )
    });

    // one field row: label + control + hint.
    rt.route("/storybook/field", "Field", |_| {
        route_def("route /storybook/field", "Field", field::demo())
    });

    // the field's label.
    rt.route("/storybook/label", "Label", |_| {
        route_def("route /storybook/label", "Label", label::demo())
    });

    // the field's hint line.
    rt.route("/storybook/hint", "Hint Text", |_| {
        route_def("route /storybook/hint", "Hint Text", hint_text::demo())
    });

    // bare single-line input.
    rt.route("/storybook/text-input", "Text Input", |_| {
        route_def(
            "route /storybook/text-input",
            "Text Input",
            text_input::demo(),
        )
    });

    // bare boolean switch.
    rt.route("/storybook/toggle", "Toggle Button", |_| {
        route_def(
            "route /storybook/toggle",
            "Toggle Button",
            toggle_button::demo(),
        )
    });

    // bare numeric stepper.
    rt.route("/storybook/range", "Range Input", |_| {
        route_def("route /storybook/range", "Range Input", range_input::demo())
    });

    // bare single-choice segmented control.
    rt.route("/storybook/radio", "Radio Input", |_| {
        route_def("route /storybook/radio", "Radio Input", radio_input::demo())
    });

    // bare multiline input.
    rt.route("/storybook/text-area", "Text Area", |_| {
        route_def("route /storybook/text-area", "Text Area", text_area::demo())
    });

    // bare token box.
    rt.route("/storybook/tags", "Tag Input", |_| {
        route_def("route /storybook/tags", "Tag Input", tag_input::demo())
    });

    // the form's verdict line: error / ok samples.
    rt.route("/storybook/validation", "Validation Info", |_| {
        route_def(
            "route /storybook/validation",
            "Validation Info",
            validation_info::demo(),
        )
    });

    // the sidebar session list.
    rt.route("/storybook/session-history", "Session History", |_| {
        route_def(
            "route /storybook/session-history",
            "Session History",
            session_history::demo(),
        )
    });

    // the transcript column: message cards in order.
    rt.route("/storybook/session", "Session", |_| {
        route_def("route /storybook/session", "Session", session::demo())
    });
    // the whole middle column: transcript above, composer below.
    rt.route("/storybook/session-panel", "Session Panel", |_| {
        route_def(
            "route /storybook/session-panel",
            "Session Panel",
            session_panel::demo(),
        )
    });

    // the evidence cards: the fold primitive, the two prompts, the verdict.
    rt.route("/storybook/fold-card", "Fold Card", |_| {
        route_def("route /storybook/fold-card", "Fold Card", fold_card::demo())
    });
    rt.route("/storybook/prompt-card", "Prompt Card", |_| {
        route_def("route /storybook/prompt-card", "Prompt Card", prompt_card::demo())
    });
    rt.route("/storybook/decision-card", "Decision Card", |_| {
        route_def("route /storybook/decision-card", "Decision Card", decision_card::demo())
    });

    // the prompt bar: draft + dropdowns + ctx pie + send/stop.
    rt.route("/storybook/composer", "Composer", |_| {
        route_def("route /storybook/composer", "Composer", composer::demo())
    });

    // the themed select: icon + mono value + caret, dark menu.
    rt.route("/storybook/dropdown", "Dropdown", |_| {
        route_def("route /storybook/dropdown", "Dropdown", dropdown::demo())
    });

    // one route per message card, each alone on screen.
    rt.route("/storybook/system-card", "System Card", |_| {
        route_def("route /storybook/system-card", "System Card", system_card::demo())
    });
    rt.route("/storybook/user-card", "User Card", |_| {
        route_def("route /storybook/user-card", "User Card", user_card::demo())
    });
    rt.route("/storybook/assistant-card", "Assistant Card", |_| {
        route_def(
            "route /storybook/assistant-card",
            "Assistant Card",
            assistant_card::demo(),
        )
    });
    rt.route("/storybook/reasoning-card", "Reasoning Card", |_| {
        route_def(
            "route /storybook/reasoning-card",
            "Reasoning Card",
            reasoning_card::demo(),
        )
    });
    rt.route("/storybook/tool-call-card", "Tool Call Card", |_| {
        route_def(
            "route /storybook/tool-call-card",
            "Tool Call Card",
            tool_call_card::demo(),
        )
    });
    rt.route("/storybook/tool-response-card", "Tool Response Card", |_| {
        route_def(
            "route /storybook/tool-response-card",
            "Tool Response Card",
            tool_response_card::demo(),
        )
    });
    rt.route("/storybook/error-card", "Error Card", |_| {
        route_def("route /storybook/error-card", "Error Card", error_card::demo())
    });
    rt.route("/storybook/approval-card", "Approval Card", |_| {
        route_def(
            "route /storybook/approval-card",
            "Approval Card",
            approval_card::demo(),
        )
    });
    rt.route("/storybook/thinking", "Thinking", |_| {
        route_def("route /storybook/thinking", "Thinking", thinking::demo())
    });
    rt.route("/storybook/task-list", "Task List", |_| {
        route_def("route /storybook/task-list", "Task List", task_list::demo())
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
        route_def("route /storybook/tab-bar", "Tab Bar", tab::tab_bar(samples::EXPR))
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


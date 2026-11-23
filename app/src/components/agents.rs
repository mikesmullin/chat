//! `agents` — the store behind the Agents rail and the agent editor.
//!
//! Separate from the `live` store on purpose: an agent definition outlives
//! every conversation that uses it, and folding the two together would mean
//! a draft edit disappearing because somebody opened a different session.
//!
//! The editor is a plain draft object. Nothing is saved until Save is
//! pressed, because a definition is the thing that decides what an agent
//! *is* — an autosave on every keystroke would rewrite a live agent's soul
//! halfway through a word.

use wasm_bindgen::JsValue;

use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

pub const STORE: &str = "agents";

/// The app's accent, and what an agent without one is drawn in.
pub const DEFAULT_ACCENT: &str = "#a78bfa";

/// The Phosphor icon an agent without one of its own is drawn with.
pub const DEFAULT_ICON: &str = "robot";

fn pick_option(_this: JsValue, args: js_sys::Array) -> JsValue {
    let key = js::arg_str(&args, 0);
    if key == "model" || key == "reasoningEffort" {
        let s = mjs::store_ref(STORE);
        js::set_str(&js::get(&s, "draft"), &key, &js::arg_str(&args, 1));
        js::set_str(&s, "saved", "");
        js::set_str(&mjs::store_ref("dropdown"), "open", "");
        mjs::redraw();
    }
    JsValue::UNDEFINED
}

/// The fields the editor round-trips. Listed once so loading a definition
/// into the draft and sending the draft back cannot drift apart.
const TEXT_FIELDS: &[&str] = &[
    "name",
    "description",
    "soul",
    "goals",
    "memory",
    "engagement",
    "model",
    "reasoningEffort",
    "allowlist",
    "policyMode",
    "accentColor",
    // A bare Phosphor name (`robot`, `dog`, `terminal`). Blank means the
    // default, so clearing the box is how an agent goes back to it.
    "icon",
    // Sent as a string and parsed server-side: an empty box means "use the
    // default", which is a different thing from the number 0.
    "maxTurns",
    // The agent's workspace root: where file-io and the shell start. Blank
    // means the project root. Trimmed on save so a pasted path with trailing
    // spaces points where it was meant to.
    "cwd",
    // PLAN55 D1: the agent's Containerfile, an absolute host path. Blank
    // means no sandbox — the agent runs on the host.
    "containerFile",
];

fn blank_draft() -> JsValue {
    let d = js::obj();
    for f in TEXT_FIELDS {
        js::set_str(&d, f, "");
    }
    // What a new agent does by default: take part. `invoked` and `relevant`
    // are deliberate choices, not something to land in by accident.
    js::set_str(&d, "engagement", "engaged");
    js::set_str(&d, "policyMode", "ask");
    // A colour picker with no value shows black, which reads as a choice
    // nobody made. Start on the app's accent instead.
    js::set_str(&d, "accentColor", DEFAULT_ACCENT);
    // Shown rather than left blank, so a new agent's icon is something you
    // can see and change instead of a box whose empty state you have to
    // know the meaning of.
    js::set_str(&d, "icon", DEFAULT_ICON);
    // Newline-separated in the form; the server accepts either shape.
    js::set_str(&d, "mcp", "");
    d.into()
}

/// Shut the settings panel, which shares the inspector column.
fn close_settings() {
    js::set_bool(&mjs::store_ref(crate::components::settings::STORE), "open", false);
}

/* Shut the session editor, which shares the inspector column. */
fn close_session_editor() {
    js::set_bool(&mjs::store_ref("sessionDraft"), "open", false);
}

/// `loadAgents()` — the rail.
fn load_agents(_this: JsValue, _args: js_sys::Array) -> JsValue {
    rustex1_kit::api::get("/api/agents", None, |v| {
        let s = mjs::store_ref(STORE);
        js::set(&s, "agents", &js::get(&v, "agents"));
        js::set(&s, "engagements", &js::get(&v, "engagements"));
        js::set(&s, "mcpPresets", &js::get(&v, "mcpPresets"));
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

/// One draft field as text, whatever m-js chose to store there.
///
/// `x-model` on `input[type=number]` writes a **number** — or `null` when the
/// box is empty — rather than a string (m-js `build.js`, `applyModel`). So
/// `get_str` yields "" for a perfectly good value, and a save built from it
/// clears the very field it was meant to set. Reading both shapes here keeps
/// that off every caller.
fn draft_text(draft: &JsValue, field: &str) -> String {
    let v = js::get(draft, field);
    if let Some(t) = v.as_string() {
        return t;
    }
    match v.as_f64() {
        // Whole numbers without the `.0` a float would print.
        Some(n) if n.fract() == 0.0 => (n as i64).to_string(),
        Some(n) => n.to_string(),
        // `null` for an emptied number box: unset, which is what "" means
        // to the server.
        None => String::new(),
    }
}

/// Fill the draft from a definition.
fn to_draft(a: &JsValue) -> JsValue {
    let d = js::obj();
    for f in TEXT_FIELDS {
        js::set_str(&d, f, &js::get_str(a, f));
    }
    // An agent with no colour of its own edits from the default rather than
    // from an empty swatch.
    if js::get_str(a, "accentColor").is_empty() {
        js::set_str(&d, "accentColor", DEFAULT_ACCENT);
    }
    // Likewise the icon: an agent that has never been given one edits from
    // the default it is already being drawn with, not from an empty box.
    if js::get_str(a, "icon").is_empty() {
        js::set_str(&d, "icon", DEFAULT_ICON);
    }
    // `maxTurns` arrives as a JSON number, and `get_str` yields "" for
    // anything that is not a string — so without this the box would read
    // empty however it was configured. Unset stays "", which is what shows
    // the placeholder.
    js::set_str(
        &d,
        "maxTurns",
        &js::get(a, "maxTurns")
            .as_f64()
            .map(|v| (v as i64).to_string())
            .unwrap_or_default(),
    );
    // Arrays render as one-per-line, which is what a textarea is good at.
    let mcp = js_sys::Array::from(&js::get(a, "mcp"));
    let lines: Vec<String> = mcp.iter().filter_map(|x| x.as_string()).collect();
    js::set_str(&d, "mcp", &lines.join("\n"));
    d.into()
}

/// `select(name)` — open one agent in the inspector.
fn select(_this: JsValue, args: js_sys::Array) -> JsValue {
    let name = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    if name.is_empty() {
        return JsValue::UNDEFINED;
    }
    // Find it in the list we already have rather than re-fetching: the rail
    // just rendered from it, so a round trip would only add a flicker.
    let list = js_sys::Array::from(&js::get(&s, "agents"));
    let found = list.iter().find(|a| js::get_str(a, "name") == name);
    let Some(a) = found else {
        return JsValue::UNDEFINED;
    };
    js::set_str(&s, "selected", &name);
    // One panel at a time: the inspector shows what you last selected, and
    // two unrelated panels stacked in it is the answer to neither click.
    close_settings();
    close_session_editor();
    // The name as it was when the form opened: a rename has to tell the
    // server which row it is renaming.
    js::set_str(&s, "editing", &name);
    js::set(&s, "draft", &to_draft(&a));
    js::set_bool(&s, "open", true);
    js::set_bool(&s, "creating", false);
    js::set_str(&s, "error", "");
    js::set_str(&s, "saved", "");
    // The inspector is the third column, and it starts collapsed: picking an
    // agent with the column shut would look like the row did nothing. Same
    // line as `new_agent` below, for the same reason.
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `newAgent()` — the `+` beside the Agents title.
fn new_agent(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set(&s, "draft", &blank_draft());
    js::set_str(&s, "selected", "");
    js::set_str(&s, "editing", "");
    close_settings();
    close_session_editor();
    js::set_bool(&s, "open", true);
    js::set_bool(&s, "creating", true);
    js::set_str(&s, "error", "");
    js::set_str(&s, "saved", "");
    // The inspector is the third column; opening the editor with the column
    // collapsed would look like the button did nothing.
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
    // Re-read while we are here: the rail loads once on init, so an agent
    // defined from anywhere else would otherwise not be in the list.
    load_agents(JsValue::UNDEFINED, js_sys::Array::new());
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `close()` — leave the editor.
fn close(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "open", false);
    js::set_str(&s, "selected", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `save()` — create or update.
fn save(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let draft = js::get(&s, "draft");
    let name = js::get_str(&draft, "name").trim().to_string();
    if name.is_empty() {
        crate::components::validation_info::note("agents", "error", "an agent needs a name");
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    let payload = js::obj();
    for f in TEXT_FIELDS {
        js::set_str(&payload, f, draft_text(&draft, f).trim_end());
    }
    js::set_str(&payload, "name", &name);
    // One preset per line in the form; the server splits either shape.
    js::set_str(&payload, "mcp", &js::get_str(&draft, "mcp"));

    let editing = js::get_str(&s, "editing");
    let url = if editing.is_empty() {
        "/api/agents".to_string()
    } else {
        format!("/api/agents/{editing}")
    };
    js::set_bool(&s, "saving", true);
    mjs::redraw();

    rustex1_kit::api::post_json(&url, &payload.into(), move |v| {
        let s = mjs::store_ref(STORE);
        js::set_bool(&s, "saving", false);
        let err = js::get_str(&v, "error");
        if !err.is_empty() {
            crate::components::validation_info::note("agents", "error", &err);
            mjs::redraw();
            return;
        }
        let agent = js::get(&v, "agent");
        let saved = js::get_str(&agent, "name");
        js::set_str(&s, "error", "");
        crate::components::validation_info::note("agents", "saved", "Saved");
        js::set_bool(&s, "creating", false);
        // After a rename the row is under its new name, so the form has to
        // point at that one or the next save would 404.
        js::set_str(&s, "editing", &saved);
        js::set_str(&s, "selected", &saved);
        js::set(&s, "draft", &to_draft(&agent));
        load_agents(JsValue::UNDEFINED, js_sys::Array::new());
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

/// `remove()` — delete the agent being edited.
fn remove(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let name = js::get_str(&s, "editing");
    if name.is_empty() {
        return JsValue::UNDEFINED;
    }
    // Two presses rather than a dialog: the destructive press is a different
    // press from the one that reveals it, which is the property that
    // matters, and a modal over an inspector panel is its own problem.
    if !js::get_bool(&s, "confirmDelete") {
        js::set_bool(&s, "confirmDelete", true);
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    rustex1_kit::api::get(&format!("/api/agents/{name}"), Some("DELETE"), move |v| {
        let s = mjs::store_ref(STORE);
        js::set_bool(&s, "confirmDelete", false);
        let err = js::get_str(&v, "error");
        if !err.is_empty() {
            crate::components::validation_info::note("agents", "error", &err);
            mjs::redraw();
            return;
        }
        js::set_bool(&s, "open", false);
        js::set_str(&s, "selected", "");
        js::set_str(&s, "editing", "");
        load_agents(JsValue::UNDEFINED, js_sys::Array::new());
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

/// `setEngagement(value)` — the three-way switch in the editor.
fn set_engagement(_this: JsValue, args: js_sys::Array) -> JsValue {
    let v = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    js::set_str(&js::get(&s, "draft"), "engagement", &v);
    js::set_str(&s, "saved", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `engagementLabel(value)` — what the three modes mean, in words.
fn engagement_label(_this: JsValue, args: js_sys::Array) -> JsValue {
    let v = js::arg_str(&args, 0);
    JsValue::from_str(match v.as_str() {
        "engaged" => "replies to every message",
        "relevant" => "replies when a decision model says it is relevant",
        "invoked" => "replies only when @mentioned",
        _ => "",
    })
}

pub fn register() {
    let mut d = Def::new().named("store agents");
    d.val("agents", &js_sys::Array::new().into())
        .val("engagements", &js_sys::Array::new().into())
        .val("mcpPresets", &js_sys::Array::new().into())
        .val("draft", &blank_draft())
        // The agent highlighted in the rail.
        .str("selected", "")
        // The name the row is stored under, which a rename changes.
        .str("editing", "")
        .bool("open", false)
        .bool("creating", false)
        .bool("saving", false)
        .bool("confirmDelete", false)
        .str("error", "")
        .str("saved", "")
        .method("loadAgents", load_agents)
        .method("select", select)
        .method("newAgent", new_agent)
        .method("close", close)
        .method("save", save)
        .method("pickOption", pick_option)
        .method("remove", remove)
        .method("setEngagement", set_engagement)
        .method("engagementLabel", engagement_label);
    mjs::store(STORE, &d.finish());
}

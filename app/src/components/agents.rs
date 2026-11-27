use wasm_bindgen::JsValue;

use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

pub const STORE: &str = "agents";

pub const DEFAULT_ACCENT: &str = "#a78bfa";

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
    "icon",
    "maxTurns",
    "containerFile",
];

fn blank_draft() -> JsValue {
    let d = js::obj();
    for f in TEXT_FIELDS {
        js::set_str(&d, f, "");
    }
    js::set_str(&d, "engagement", "engaged");
    js::set_str(&d, "policyMode", "ask");
    js::set_str(&d, "accentColor", DEFAULT_ACCENT);
    js::set_str(&d, "icon", DEFAULT_ICON);
    js::set_str(&d, "mcp", "");
    d.into()
}

fn close_settings() {
    js::set_bool(&mjs::store_ref(crate::components::settings::STORE), "open", false);
}

fn close_session_editor() {
    js::set_bool(&mjs::store_ref("sessionDraft"), "open", false);
    js::set_bool(&mjs::store_ref("groupDraft"), "open", false);
}

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

fn draft_text(draft: &JsValue, field: &str) -> String {
    let v = js::get(draft, field);
    if let Some(t) = v.as_string() {
        return t;
    }
    match v.as_f64() {
        Some(n) if n.fract() == 0.0 => (n as i64).to_string(),
        Some(n) => n.to_string(),
        None => String::new(),
    }
}

fn to_draft(a: &JsValue) -> JsValue {
    let d = js::obj();
    for f in TEXT_FIELDS {
        js::set_str(&d, f, &js::get_str(a, f));
    }
    if js::get_str(a, "accentColor").is_empty() {
        js::set_str(&d, "accentColor", DEFAULT_ACCENT);
    }
    if js::get_str(a, "icon").is_empty() {
        js::set_str(&d, "icon", DEFAULT_ICON);
    }
    js::set_str(
        &d,
        "maxTurns",
        &js::get(a, "maxTurns")
            .as_f64()
            .map(|v| (v as i64).to_string())
            .unwrap_or_default(),
    );
    let mcp = js_sys::Array::from(&js::get(a, "mcp"));
    let lines: Vec<String> = mcp.iter().filter_map(|x| x.as_string()).collect();
    js::set_str(&d, "mcp", &lines.join("\n"));
    d.into()
}

fn select(_this: JsValue, args: js_sys::Array) -> JsValue {
    let name = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    if name.is_empty() {
        return JsValue::UNDEFINED;
    }
    let list = js_sys::Array::from(&js::get(&s, "agents"));
    let found = list.iter().find(|a| js::get_str(a, "name") == name);
    let Some(a) = found else {
        return JsValue::UNDEFINED;
    };
    js::set_str(&s, "selected", &name);
    close_settings();
    close_session_editor();
    js::set_str(&s, "editing", &name);
    js::set(&s, "draft", &to_draft(&a));
    js::set_bool(&s, "open", true);
    js::set_bool(&s, "creating", false);
    js::set_str(&s, "error", "");
    js::set_str(&s, "saved", "");
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
    mjs::redraw();
    JsValue::UNDEFINED
}

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
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
    load_agents(JsValue::UNDEFINED, js_sys::Array::new());
    mjs::redraw();
    JsValue::UNDEFINED
}

fn close(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "open", false);
    js::set_str(&s, "selected", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

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
        js::set_str(&s, "editing", &saved);
        js::set_str(&s, "selected", &saved);
        js::set(&s, "draft", &to_draft(&agent));
        load_agents(JsValue::UNDEFINED, js_sys::Array::new());
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

fn remove(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let name = js::get_str(&s, "editing");
    if name.is_empty() {
        return JsValue::UNDEFINED;
    }
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

fn set_engagement(_this: JsValue, args: js_sys::Array) -> JsValue {
    let v = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    js::set_str(&js::get(&s, "draft"), "engagement", &v);
    js::set_str(&s, "saved", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

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
        .str("selected", "")
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

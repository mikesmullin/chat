use wasm_bindgen::JsValue;
use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::{js, mjs};
use rustex1_kit::mjs::Def;
use crate::components::{session_editor, validation_info};

const ST: &str = "$store.groupDraft";

pub fn panel(st: &str) -> Node {
    let field = session_editor::field;
    m("div").class("agent-editor session-editor").show(&format!("{ST}.open")).kids(vec![
        m("div").class("agent-editor-head").kids(vec![Node::icon("pencil-simple"), m("span").class("agent-editor-title").text(&format!("('Edit group: ' + {ST}.name)"))]),
        validation_info::live(&format!("({ST}.messageTone ? ' ' + {ST}.messageTone : '')"), &format!("{ST}.message")),
        field("Purpose", "optional", &format!("{ST}.purpose"), "", 0),
        field("Notes", "optional", &format!("{ST}.notes"), "", 3),
        field("Soul appendage", "added to each agent's soul; blank inherits", &format!("{ST}.soulAppendage"), "", 4),
        field("Default agents", "one agent name per line; blank inherits", &format!("{ST}.defaultAgents"), "coder", 3),
        m("div").class("ae-field").kids(vec![m("label").class("ae-label").kids(vec![m("span").child(lit("Routing")), m("span").class("hint").child(lit("optional"))]), session_editor::routing_modes(ST)]),
        field("Memory budget", "optional", &format!("{ST}.memoryHigh"), "", 0),
        field("Workspace root", "optional absolute path", &format!("{ST}.workspaceRoot"), "", 0),
        m("div").class("se-worktree").kids(session_editor::worktree_config(st, ST)),
        m("div").class("ae-actions").child(m("button").class("ae-btn primary").attr("type", "button").on("click", &format!("{ST}.save()")).child(lit("Save"))),
    ])
}

pub fn demo() -> Node {
    m("div").init("$store.groupDraft.seed('psicom/chat', $store.samples); $store.groupDraft.open = true").child(panel(crate::components::samples::EXPR))
}

fn seed(_this: JsValue, args: js_sys::Array) -> JsValue {
    let name = js::arg_str(&args, 0);
    let source = args.get(1);
    let draft = mjs::store_ref("groupDraft");
    js::set_str(&draft, "name", &name);
    let from = args.get(2).as_string().unwrap_or_else(|| "samples".to_string());
    js::set_str(&draft, "from", &from);
    let mut props = js::obj().into();
    for row in js_sys::Array::from(&js::get(&source, "projects")).iter() {
        if js::get_str(&row, "name") == name { props = js::get(&row, "props"); break; }
    }
    for key in ["purpose", "notes", "routing", "workspaceRoot", "soulAppendage"] { js::set_str(&draft, key, &js::get_str(&props, key)); }
    let value = js::get(&props, "defaultAgents");
    let names: Vec<String> = if js_sys::Array::is_array(&value) {
        js_sys::Array::from(&value).iter().filter_map(|v| v.as_string()).collect()
    } else { Vec::new() };
    js::set_str(&draft, "defaultAgents", &names.join("\n"));
    let n = js::get(&props, "memoryHigh");
    js::set_str(&draft, "memoryHigh", &n.as_f64().map(|n| n.to_string()).unwrap_or_default());
    let wt = js::get(&props, "worktree");
    for (key, field) in [("repoId", "worktreeRepoId"), ("branchPattern", "worktreeBranchPattern"), ("pathPattern", "worktreePathPattern")] { js::set_str(&draft, field, &js::get_str(&wt, key)); }
    js::set_str(&draft, "message", "");
    JsValue::UNDEFINED
}

pub fn open_folder(_this: JsValue, args: js_sys::Array) -> JsValue {
    let kind = js::arg_str(&args, 0);
    let name = js::arg_str(&args, 1);
    if kind == "unfiled" {
        js::call(&mjs::store_ref("ptree"), "toggle", &[JsValue::from_str(&name)]);
        return JsValue::UNDEFINED;
    }
    let allowed = js::call(&mjs::store_ref("ident"), "can", &[JsValue::from_str("project.create")]);
    if allowed.as_bool() != Some(true) { return JsValue::UNDEFINED; }
    let forward = js_sys::Array::new();
    forward.push(&JsValue::from_str(&name));
    forward.push(&args.get(2));
    open(JsValue::UNDEFINED, forward)
}

pub fn open(_this: JsValue, args: js_sys::Array) -> JsValue {
    let name = js::arg_str(&args, 0);
    if name.is_empty() { return JsValue::UNDEFINED; }
    let source_name = args.get(1).as_string().unwrap_or_else(|| "live".into());
    let live = mjs::store_ref(&source_name);
    let draft = mjs::store_ref("groupDraft");
    let home = mjs::store_ref("home");
    if js::get_str(&live, "activeProject") == name && js::get_bool(&draft, "open") {
        crate::components::aside_panels::close();
        js::set_bool(&home, "asideOpen", false);
    } else {
        js::set_str(&live, "activeProject", &name);
        js::call(&draft, "seed", &[JsValue::from_str(&name), live, JsValue::from_str(&source_name)]);
        crate::components::aside_panels::close();
        js::set_bool(&draft, "open", true);
        js::set_bool(&home, "asideOpen", true);
    }
    mjs::redraw();
    JsValue::UNDEFINED
}

fn set_routing(_this: JsValue, args: js_sys::Array) -> JsValue {
    js::set_str(&mjs::store_ref("groupDraft"), "routing", &js::arg_str(&args, 0));
    mjs::redraw();
    JsValue::UNDEFINED
}

fn pick_repo(_this: JsValue, args: js_sys::Array) -> JsValue {
    let draft = mjs::store_ref("groupDraft");
    let id = js::arg_str(&args, 0);
    js::set_str(&draft, "worktreeRepoId", &id);
    if !id.is_empty() && js::get_str(&draft, "worktreeBranchPattern").is_empty() {
        js::set_str(&draft, "worktreeBranchPattern", "chat/{{hex}}");
    }
    js::call(&mjs::store_ref("dropdown"), "closeIf", &[JsValue::from_str("repo")]);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn save(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref("groupDraft");
    if js::get_str(&s, "from") == "samples" {
        js::set_str(&s, "messageTone", "ok");
        js::set_str(&s, "message", "Sample saved");
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    let payload = js::obj();
    js::set_str(&payload, "name", &js::get_str(&s, "name"));
    let patch = js::obj();
    for key in ["purpose", "notes", "routing", "workspaceRoot", "soulAppendage"] { js::set_str(&patch, key, &js::get_str(&s, key)); }
    let names_text = js::get_str(&s, "defaultAgents");
    let names = js_sys::Array::new();
    for name in names_text.lines().map(str::trim).map(|s| s.trim_start_matches('@')).filter(|s| !s.is_empty()) {
        names.push(&JsValue::from_str(name));
    }
    if names.length() == 0 { js::set(&patch, "defaultAgents", &JsValue::NULL); }
    else { js::set(&patch, "defaultAgents", &names.into()); }
    let memory = js::get_str(&s, "memoryHigh");
    if memory.trim().is_empty() { js::set(&patch, "memoryHigh", &JsValue::NULL); }
    else if let Ok(n) = memory.trim().parse::<u64>() { js::set(&patch, "memoryHigh", &JsValue::from_f64(n as f64)); }
    else { js::set_str(&s, "messageTone", "error"); js::set_str(&s, "message", "Memory budget must be a non-negative integer"); mjs::redraw(); return JsValue::UNDEFINED; }
    let repo = js::get_str(&s, "worktreeRepoId");
    if repo.trim().is_empty() { js::set(&patch, "worktree", &JsValue::NULL); } else {
        let wt = js::obj();
        for (key, field) in [("repoId", "worktreeRepoId"), ("branchPattern", "worktreeBranchPattern"), ("pathPattern", "worktreePathPattern")] { js::set_str(&wt, key, &js::get_str(&s, field)); }
        js::set(&patch, "worktree", &wt.into());
    }
    js::set(&payload, "patch", &patch.into());
    rustex1_kit::api::post_json("/api/projects/settings", &payload.into(), |v| {
        let draft = mjs::store_ref("groupDraft");
        let failed = js::get(&v, "ok").as_bool() == Some(false);
        js::set_str(&draft, "messageTone", if failed { "error" } else { "ok" });
        let message = if failed { js::get_str(&v, "error") } else { "Saved".to_string() };
        js::set_str(&draft, "message", &message);
        if !failed { js::call(&mjs::store_ref("live"), "loadProjects", &[]); }
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

pub fn register() {
    let mut d = Def::new().named("store groupDraft");
    d.bool("open", false).str("from", "samples").str("name", "").str("purpose", "").str("notes", "").str("soulAppendage", "").str("defaultAgents", "").str("routing", "").str("memoryHigh", "").str("workspaceRoot", "").str("worktreeRepoId", "").str("worktreeBranchPattern", "").str("worktreePathPattern", "").str("message", "").str("messageTone", "").method("seed", seed).method("save", save).method("pickRepo", pick_repo).method("setRouting", set_routing);
    mjs::store("groupDraft", &d.finish());
}

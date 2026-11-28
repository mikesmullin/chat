use wasm_bindgen::JsValue;

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::css;
use crate::components::{cron_field, field, name_tag, picker, validation_info};

const STORE: &str = "tform";
const ST: &str = "$store.tform";

pub const CSS: &str = r#"
:is(#sb, #home) .tform-note {
  margin: 0;
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--panel-2);
  color: var(--gray-dim);
  font-size: 11px;
  line-height: 1.6;
}
:is(#sb, #home) .tform-note.tform-warn {
  display: flex; align-items: flex-start; gap: 6px;
  color: var(--danger, #d9534f);
  border-color: color-mix(in srgb, var(--danger, #d9534f) 30%, transparent);
  background: color-mix(in srgb, var(--danger, #d9534f) 8%, transparent);
}
:is(#sb, #home) .tform-warn .ph {
  flex: 0 0 auto;
  font-size: 13px; line-height: 1.6;
  margin-top: 0.5px;
}
:is(#sb, #home) .tform-path {
  display: flex; align-items: center; gap: 4px; flex-wrap: wrap;
  font-size: 11px; color: var(--gray-dim);
}
:is(#sb, #home) .tform-path .seg {
  padding: 1px 6px;
  border-radius: 5px;
  background: var(--panel-2);
  color: var(--gray);
}
:is(#sb, #home) .tform-path .sep { opacity: .5; }

.tform-value {
  display: inline-flex; align-items: center;
  font-size: 12px;
  color: var(--fg);
}
.tform-value-icon {
  font-size: 13px; line-height: 1;
  color: var(--gray-dim);
  margin-right: 5px;
}
.tform-value > span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tform-status { display: inline-flex; align-items: center; gap: 6px; }
.tform-status-icon { display: inline-block; font-size: 14px; line-height: 1; }
.tform-status.is-busy .tform-status-icon { color: var(--mjs-yellow); animation: tform-status-spin 1s linear infinite; }
.tform-status.is-ready .tform-status-icon { color: var(--pastel-green, var(--ok, #22c55e)); }
.tform-status.is-error .tform-status-icon { color: var(--pastel-red, var(--danger, #d9534f)); }
.tform-status.is-warning .tform-status-icon { color: var(--pastel-amber, #f5d8a6); }
.tform-status.is-idle .tform-status-icon { color: var(--gray-dim); }
@keyframes tform-status-spin { to { transform: rotate(360deg); } }
@media (prefers-reduced-motion: reduce) {
  .tform-status.is-busy .tform-status-icon { animation: none; }
}

.tform-demo { width: 320px; }
"#;

pub fn panel() -> Node {
    m("div")
        .class("agent-editor target-form")
        .show(&format!("{ST}.open"))
        .kids(vec![
            m("div").class("agent-editor-head").kids(vec![
                Node::icon_dyn(&format!("{ST}.icon")),
                m("span")
                    .class("agent-editor-title")
                    .text(&format!("{ST}.heading")),
            ]),
            validation_info::live("'error'", &format!("{ST}.error")),
            field::field(
                "Name",
                false,
                "use / to nest, e.g. psicom/chat",
                m("input")
                    .attr("type", "text")
                    .attr("spellcheck", "false")
                    .attr("placeholder", "psicom/chat")
                    .model(&format!("{ST}.name"))
                    .on("input", &format!("{ST}.name = $event.target.value"))
                    .on("keydown.enter", &format!("{ST}.save()")),
                false,
            )
            .show(&format!("{ST}.kind === 'project'")),
            m("div").class("ae-field se-project")
                .show(&format!("{ST}.kind === 'project' && {ST}.mode === 'create'"))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![m("span").child(lit("Project group")), m("span").class("hint").child(lit("optional parent"))]),
                    picker::picker(&picker::Picker {
                        key: "project-parent", icon: "folder", extra: "dd-project",
                        search: "Search projects…", items: &format!("{ST}.projects()"),
                        value: &format!("{ST}.parentProject"), null: Some(("No parent", "Top level")),
                        null_icon: "folder", on_pick: &format!("{ST}.pickParent({{}})"),
                        on_edit: "", actions: &[],
                    }),
                ]),
            m("div")
                .class("tform-path")
                .show(&format!("{ST}.kind === 'project' && {ST}.segments().length > 1"))
                .kids(vec![m("span")
                    .class("seg")
                    .for_each(&format!("seg in {ST}.segments()"))
                    .key("seg")
                    .text("seg")]),
            m("div")
                .class("ae-field")
                .show(&format!(
                    "{ST}.mode === 'edit' && {ST}.kind !== 'project' && {ST}.kind !== 'sandbox'"
                ))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![
                        m("span").child(lit("Original")),
                    ]),
                    m("span").class("tform-value").kids(vec![
                        m("i").class("ph ph-key tform-value-icon"),
                        m("span").text(&format!("{ST}.original")),
                    ]),
                ]),
            m("div").class("ae-field").show(&format!("{ST}.kind === 'sandbox'")).kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Container")),
                ]),
                m("span").class("tform-value").kids(vec![
                    m("i").class("ph ph-key tform-value-icon"),
                    m("span").text(&format!("{ST}.original")),
                ]),
            ]),
            m("div").class("ae-field").show(&format!("{ST}.kind === 'sandbox'")).kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Image")),
                ]),
                m("span").class("tform-value").kids(vec![
                    m("i").class("ph ph-shipping-container tform-value-icon"),
                    m("span").text(&format!("{ST}.image")),
                ]),
            ]),
            m("div").class("ae-field").show(&format!("{ST}.kind === 'sandbox'")).kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Status")),
                ]),
                m("span").class("tform-status").attr("role", "status")
                    .bind("class", &format!("('tform-status ' + {ST}.statusClass({ST}.status))"))
                    .kids(vec![
                        Node::icon_dyn(&format!("{ST}.statusIcon({ST}.status)"))
                            .class("tform-status-icon").attr("aria-hidden", "true"),
                        m("span").text(&format!("{ST}.statusLabel({ST}.status)")),
                    ]),
            ]),
            m("div").class("ae-field").show(&format!("{ST}.kind === 'sandbox'")).kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Agent")),
                ]),
                name_tag::who_link(
                    &format!("({ST}.agentIcon || 'robot')"),
                    &format!("({ST}.agentAccent || '#a78bfa')"),
                    &format!("{ST}.agentName"),
                    &format!("$store.agents.select({ST}.agentName); {ST}.close()"),
                )
                .show(&format!("{ST}.agentName")),
                m("span")
                    .class("tform-value")
                    .show(&format!("!{ST}.agentName"))
                    .text("'unassigned'"),
            ]),
            m("div").class("ae-field").show(&format!("{ST}.kind === 'sandbox'")).kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Created")),
                ]),
                m("span").class("tform-value").kids(vec![
                    m("i").class("ph ph-calendar tform-value-icon"),
                    m("span").text(&format!("{ST}.createdAt")),
                ]),
            ]),
            m("div").class("ae-field").show(&format!("{ST}.kind === 'worktree'")).kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Branch")),
                ]),
                m("span").class("tform-value").kids(vec![
                    m("i").class("ph ph-git-branch tform-value-icon"),
                    m("span").text(&format!("{ST}.name")),
                ]),
            ]),
            m("div").class("ae-field").show(&format!("{ST}.kind === 'worktree'")).kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Repo")),
                ]),
                m("span").class("tform-value").kids(vec![
                    m("i").class("ph ph-github-logo tform-value-icon"),
                    m("span").text(&format!("{ST}.repoName")),
                ]),
            ]),
            m("div").class("ae-field").show(&format!("{ST}.kind === 'worktree'")).kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Path")),
                ]),
                m("span").class("tform-value").kids(vec![
                    m("i").class("ph ph-folder-open tform-value-icon"),
                    m("span").text(&format!("{ST}.path")),
                ]),
            ]),
            field::field(
                "Path",
                false,
                "absolute, on this machine",
                m("input")
                    .attr("type", "text")
                    .attr("spellcheck", "false")
                    .attr("placeholder", "/workspace/psicom/chat")
                    .model(&format!("{ST}.path"))
                    .on("keydown.enter", &format!("{ST}.save()")),
                false,
            )
            .show(&format!("{ST}.kind === 'repo' && {ST}.mode !== 'edit'")),
            m("div")
                .class("ae-field")
                .show(&format!("({ST}.kind === 'branch' || {ST}.kind === 'worktree') && {ST}.mode !== 'edit'"))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![
                        m("span").child(lit("Repo")),
                        m("span")
                            .class("hint")
                            .child(lit("register one first if this is empty")),
                    ]),
                    m("select")
                        .model(&format!("{ST}.repoId"))
                        .kids(vec![m("option")
                            .for_each(&format!("r in {ST}.repos()"))
                            .key("r.id")
                            .bind("value", "r.id")
                            .text("r.name")]),
                ]),
            m("div")
                .class("ae-field")
                .show(&format!("({ST}.kind === 'branch' || {ST}.kind === 'worktree') && {ST}.mode !== 'edit'"))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![
                        m("span").child(lit("Branch")),
                        m("span").class("hint").text(&format!(
                            "({ST}.kind === 'worktree' ? 'checked out in the new worktree' : 'the new branch')"
                        )),
                    ]),
                    m("input")
                        .attr("type", "text")
                        .attr("spellcheck", "false")
                        .attr("placeholder", "feature/login")
                        .model(&format!("{ST}.name"))
                        .on("keydown.enter", &format!("{ST}.save()")),
                ]),
            m("label")
                .class("ae-label")
                .show(&format!("{ST}.kind === 'worktree' && {ST}.mode !== 'edit'"))
                .kids(vec![
                    m("input")
                        .attr("type", "checkbox")
                        .model(&format!("{ST}.create")),
                    m("span").child(lit("create this branch too")),
                ]),
            field::field(
                "Name",
                false,
                "shown in the rail",
                m("input")
                    .attr("type", "text")
                    .attr("spellcheck", "false")
                    .attr("placeholder", "issue-triage")
                    .model(&format!("{ST}.name"))
                    .on("keydown.enter", &format!("{ST}.save()")),
                false,
            )
            .show(&format!("{ST}.kind === 'automation'")),
            crate::components::switch::switch(
                &format!("{ST}.enabled"),
                &format!("{ST}.toggleBool('enabled')"),
                "'active'",
                None,
            )
            .show(&format!("{ST}.kind === 'automation'")),
            crate::components::switch::switch(
                &format!("{ST}.catchUp"),
                &format!("{ST}.toggleBool('catchUp')"),
                "'catch up missed runs at startup'",
                None,
            )
            .show(&format!("{ST}.kind === 'automation'")),
            cron_field::field("Schedule", ST, false)
                .show(&format!("{ST}.kind === 'automation'"))
                .on("keydown.enter", &format!("{ST}.save()")),
            m("div")
                .class("ae-field")
                .show(&format!("{ST}.kind === 'automation'"))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![
                        m("span").child(lit("Agent")),
                        m("span")
                            .class("hint")
                            .child(lit("seated before the prompt runs")),
                    ]),
                    picker::picker(&picker::Picker {
                        key: "agent",
                        icon: "robot",
                        extra: "dd-agent",
                        search: "Search agents…",
                        items: &format!("{ST}.agentItems()"),
                        value: &format!("{ST}.agent"),
                        null: Some(("Default", "no agent")),
                        null_icon: "robot",
                        on_pick: &format!("{ST}.pickAgent({{}})"),
                        on_edit: "",
                        actions: &[],
                    }),
                ]),
            m("div")
                .class("ae-field")
                .show(&format!("{ST}.kind === 'automation'"))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![
                        m("span").child(lit("Model")),
                        m("span").class("hint").child(lit("blank is the agent default")),
                    ]),
                    m("input")
                        .attr("type", "text")
                        .attr("spellcheck", "false")
                        .attr("placeholder", "agent default")
                        .model(&format!("{ST}.model"))
                        .on("keydown.enter", &format!("{ST}.save()")),
                ]),
            m("div")
                .class("ae-field")
                .show(&format!("{ST}.kind === 'automation'"))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![
                        m("span").child(lit("Prompt")),
                        m("span")
                            .class("hint")
                            .child(lit("submitted on every run")),
                    ]),
                    m("textarea")
                        .attr("placeholder", "What should the agent do each time?")
                        .model(&format!("{ST}.prompt")),
                ]),
            m("div")
                .class("ae-field")
                .show(&format!("{ST}.kind === 'automation'"))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![
                        m("span").child(lit("Timeout (minutes)")),
                        m("span").class("hint").child(lit("the run is stopped after this")),
                    ]),
                    m("input")
                        .attr("type", "text")
                        .attr("spellcheck", "false")
                        .attr("placeholder", "60")
                        .model(&format!("{ST}.timeout"))
                        .on("keydown.enter", &format!("{ST}.save()")),
                ]),
            m("p")
                .class("tform-note tform-warn")
                .show(&format!("{ST}.note"))
                .kids(vec![
                    Node::icon("warning").class("ph ph-warning tform-warn-icon"),
                    m("span").text(&format!("{ST}.note")),
                ]),
            m("div").class("ae-actions").kids(vec![
                m("button")
                    .class("ae-btn primary")
                    .attr("type", "button")
                    .show(&format!("({ST}.mode !== 'edit' || {ST}.kind === 'project' || {ST}.kind === 'automation')"))
                    .bind("disabled", &format!("({ST}.busy || !{ST}.ready())"))
                    .on("click", &format!("{ST}.save()"))
                    .text(&format!(
                        "({ST}.busy ? 'Working…' : ({ST}.mode === 'edit' ? ({ST}.kind === 'automation' ? 'Save' : 'Rename') : 'Create'))"
                    )),
                m("button")
                    .class("ae-btn")
                    .attr("type", "button")
                    .show(&format!("({ST}.mode === 'edit' && ['project', 'automation'].includes({ST}.kind))"))
                    .on("click", "$store.share.open($store.tform.kind, $store.tform.name)")
                    .kids(vec![
                        Node::icon("users-three").class("ph ph-users-three"),
                        m("span").child(lit("Share…")),
                    ]),
                m("span").class("spacer"),
                m("button")
                    .class("ae-btn danger")
                    .attr("type", "button")
                    .show(&format!("{ST}.mode === 'edit' && {ST}.deletable()"))
                    .bind("class", &format!("({ST}.confirming ? 'is-confirm' : '')"))
                    .on("click", &format!("{ST}.remove()"))
                    .text(&format!(
                        "({ST}.confirming ? 'Really delete?' : 'Delete')"
                    )),
            ]),
            m("p")
                .class("tform-note")
                .show(&format!("{ST}.confirming"))
                .text(&format!("{ST}.dangerNote")),
        ])
}

pub fn demo() -> Node { demo_for("project") }

pub fn demo_automation() -> Node { demo_for("automation") }

fn demo_for(kind: &str) -> Node {
    m("div").class("tform-demo")
        .init(&format!("{ST}.openCreate('{kind}', 'samples')"))
        .child(panel())
}


fn kind_info(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "project" => ("Project", "folder-plus"),
        "sandbox" => ("Sandbox", "cube"),
        "automation" => ("Automation", "timer"),
        "worktree" => ("Worktree", "git-fork"),
        "branch" => ("Branch", "git-branch"),
        "repo" => ("Repo", "folder-open"),
        _ => ("", "question"),
    }
}

fn danger_note(kind: &str) -> &'static str {
    match kind {
        "project" => "Deleting a folder unfiles its conversations — it never deletes them.",
        "repo" => {
            "Forgetting a repo removes it from this list only. Nothing on disk is touched."
        }
        "worktree" => {
            "Removing a worktree deletes its checkout directory, including anything \
             uncommitted in it. The branch, and everything committed to it, stays."
        }
        "sandbox" => {
            "Destroying a sandbox removes the container. Everything written inside \
             it — transcripts, downloads, scratch files — is gone with it."
        }
        "automation" => {
            "Deleting an automation stops its schedule. Sessions it already ran stay."
        }
        _ => "",
    }
}

fn open_with(kind: &str, mode: &str, store: &str, name: &str) {
    let (label, icon) = kind_info(kind);
    let s = mjs::store_ref(STORE);
    js::set_num(&s, "requestId", js::get_num(&s, "requestId") + 1.0);
    crate::components::aside_panels::close();
    js::set_bool(&s, "open", true);
    js::set_str(&s, "kind", kind);
    js::set_str(&s, "mode", mode);
    js::set_str(&s, "store", store);
    js::set_str(&s, "name", name);
    js::set_str(&s, "original", name);
    let parent = if kind == "project" && mode == "create" && mjs::has_store(store) {
        js::get_str(&js::get(&mjs::store_ref(store), "session"), "project")
    } else { String::new() };
    js::set_str(&s, "parentProject", &parent);
    js::set_str(&s, "icon", icon);
    js::set_str(&s, "dangerNote", danger_note(kind));
    js::set_bool(&s, "busy", false);
    js::set_str(&s, "path", "");
    js::set_str(&s, "repoName", "");
    js::set_str(&s, "image", "");
    js::set_str(&s, "status", "");
    js::set_str(&s, "agentName", "");
    js::set_str(&s, "agentIcon", "");
    js::set_str(&s, "agentAccent", "");
    js::set_str(&s, "createdAt", "");
    js::set_bool(&s, "create", true);
    js::set_str(&s, "cron", "");
    js::set_str(&s, "prompt", "");
    js::set_str(&s, "agent", "");
    js::set_str(&s, "model", "");
    js::set_str(&s, "timeout", "60");
    js::set_bool(&s, "enabled", true);
    js::set_bool(&s, "catchUp", false);
    seed_from_host(&s, kind, mode, name);
    js::set_str(
        &s,
        "heading",
        &if mode == "edit" {
            format!("Edit {}", label.to_lowercase())
        } else {
            format!("New {}", label.to_lowercase())
        },
    );
    js::set_str(&s, "error", "");
    js::set_bool(&s, "confirming", false);
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
    mjs::redraw();
}

fn seed_from_host(s: &JsValue, kind: &str, mode: &str, name: &str) {
    let host_name = js::get_str(s, "store");
    if host_name.is_empty() || !mjs::has_store(&host_name) {
        return;
    }
    let host = mjs::store_ref(&host_name);
    js::set_str(s, "note", "");

    if kind == "worktree" && mode == "edit" {
        let mut branch = String::new();
        let mut repo_id = String::new();
        let mut repo_name = String::new();
        let mut wt_path = String::new();
        let mut created = String::new();
        if mjs::has_store("vcs") {
            let vcs = mjs::store_ref("vcs");
            for item in js_sys::Array::from(&js::get(&vcs, "worktrees")).iter() {
                if js::get_str(&item, "id") == name {
                    branch = js::get_str(&item, "branch");
                    repo_id = js::get_str(&item, "repoId");
                    wt_path = js::get_str(&item, "path");
                    created = js::get_str(&item, "createdAt");
                    for r in js_sys::Array::from(&js::get(&vcs, "repos")).iter() {
                        if js::get_str(&r, "id") == repo_id {
                            repo_name = js::get_str(&r, "name");
                            break;
                        }
                    }
                    break;
                }
            }
        }
        if branch.is_empty() {
            for item in js_sys::Array::from(&js::get(&host, "worktrees")).iter() {
                if js::get_str(&item, "id") == name {
                    branch = js::get_str(&item, "name");
                    repo_name = js::get_str(&item, "hint");
                    wt_path = js::get_str(&item, "path");
                    break;
                }
            }
        }
        js::set_str(s, "repoId", &repo_id);
        js::set_str(s, "name", &branch);
        js::set_str(s, "path", &wt_path);
        js::set_str(s, "createdAt", &created);
        js::set_str(s, "repoName", &repo_name);
        return;
    }

    if kind == "branch" || kind == "worktree" {
        let repos = js_sys::Array::from(&js::get(&host, "repos"));
        if repos.length() == 0 {
            js::set_str(
                s,
                "note",
                "No repos registered. Use \u{201c}Open from folder\u{201d} in the branch menu first.",
            );
        } else {
            js::set_str(s, "repoId", &js::get_str(&repos.get(0), "id"));
        }
        if mode == "edit" {
            if let Some((repo, branch)) = name.split_once(':') {
                js::set_str(s, "repoId", repo);
                js::set_str(s, "name", branch);
            }
        }
    }

    if kind == "automation" {
        if mode == "edit" {
            let id = name;
            let rows = js_sys::Array::from(&js::get(&host, "automations"));
            for item in rows.iter() {
                if js::get_str(&item, "id") == id {
                    for key in ["name", "cron", "prompt", "agent", "model"] {
                        let v = js::get_str(&item, key);
                        if !v.is_empty() {
                            js::set_str(s, key, &v);
                        }
                    }
                    let timeout = js::get(&item, "timeoutMin");
                    if !timeout.is_undefined() && !timeout.is_null() {
                        if let Some(n) = timeout.as_f64() {
                            js::set_str(s, "timeout", &format!("{n}"));
                        }
                    }
                    js::set_bool(s, "enabled", js::get(&item, "enabled").as_bool().unwrap_or(true));
                    js::set_bool(s, "catchUp", js::get(&item, "catchUp").as_bool().unwrap_or(false));
                    break;
                }
            }
        }
        return;
    }

    if kind == "sandbox" {
        for item in js_sys::Array::from(&js::get(&host, "sandboxes")).iter() {
            if js::get_str(&item, "id") == name {
                js::set_str(s, "image", &js::get_str(&item, "image"));
                js::set_str(s, "status", &js::get_str(&item, "state"));
                js::set_str(s, "createdAt", &js::get_str(&item, "createdAt"));
                let agent = js::get_str(&item, "agent");
                js::set_str(s, "agentName", &agent);
                let mut icon = String::new();
                let mut accent = String::new();
                if !agent.is_empty() && mjs::has_store("agents") {
                    for a in js_sys::Array::from(&js::get(&mjs::store_ref("agents"), "agents")).iter() {
                        if js::get_str(&a, "name") == agent {
                            icon = js::get_str(&a, "icon");
                            accent = js::get_str(&a, "accentColor");
                            break;
                        }
                    }
                }
                js::set_str(s, "agentIcon", &icon);
                js::set_str(s, "agentAccent", &accent);
                break;
            }
        }
        return;
    }
}

fn repos(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let host_name = js::get_str(&s, "store");
    if host_name.is_empty() || !mjs::has_store(&host_name) {
        return js_sys::Array::new().into();
    }
    js::get(&mjs::store_ref(&host_name), "repos")
}

fn projects(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let form = mjs::store_ref(STORE);
    let source = js::get_str(&form, "store");
    if source.is_empty() || !mjs::has_store(&source) { return js_sys::Array::new().into(); }
    js::get(&mjs::store_ref(&source), "projects")
}

fn pick_parent(_this: JsValue, args: js_sys::Array) -> JsValue {
    js::set_str(&mjs::store_ref(STORE), "parentProject", &js::arg_str(&args, 0));
    js::call(&mjs::store_ref("dropdown"), "closeIf", &[JsValue::from_str("project-parent")]);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn agent_items(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let out = js_sys::Array::new();
    if !mjs::has_store("agents") {
        return out.into();
    }
    let list = js_sys::Array::from(&js::get(&mjs::store_ref("agents"), "agents"));
    for a in list.iter() {
        let name = js::get_str(&a, "name");
        if name.is_empty() {
            continue;
        }
        let icon = js::get_str(&a, "icon");
        let o = js::obj();
        js::set_str(&o, "id", &name);
        js::set_str(&o, "name", &name);
        js::set_str(&o, "icon", &if icon.is_empty() { "robot".to_string() } else { icon });
        js::set_str(&o, "hint", &js::get_str(&a, "description"));
        out.push(&o);
    }
    out.into()
}

fn pick_agent(_this: JsValue, args: js_sys::Array) -> JsValue {
    js::set_str(&mjs::store_ref(STORE), "agent", &js::arg_str(&args, 0));
    js::call(&mjs::store_ref("dropdown"), "closeIf", &[JsValue::from_str("agent")]);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn ready(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let kind = js::get_str(&s, "kind");
    let ok = match kind.as_str() {
        "project" => !js::get_str(&s, "name").trim().is_empty(),
        "repo" => !js::get_str(&s, "path").trim().is_empty(),
        "branch" | "worktree" => {
            !js::get_str(&s, "name").trim().is_empty()
                && !js::get_str(&s, "repoId").trim().is_empty()
        }
        "automation" => {
            !js::get_str(&s, "name").trim().is_empty()
                && !js::get_str(&s, "prompt").trim().is_empty()
                && cron_shape_ok(&js::get_str(&s, "cron"))
        }
        _ => false,
    };
    JsValue::from_bool(ok)
}

fn cron_shape_ok(cron: &str) -> bool {
    let c = cron.trim();
    if c.is_empty() {
        return false;
    }
    if let Some(word) = c.strip_prefix('@') {
        return !word.is_empty() && !word.contains(char::is_whitespace);
    }
    let n = c.split_whitespace().count();
    n == 5 || n == 6 || n == 7
}

fn deletable(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let kind = js::get_str(&mjs::store_ref(STORE), "kind");
    JsValue::from_bool(matches!(
        kind.as_str(),
        "project" | "repo" | "worktree" | "sandbox" | "automation"
    ))
}

fn open_create(_this: JsValue, args: js_sys::Array) -> JsValue {
    open_with(&js::arg_str(&args, 0), "create", &js::arg_str(&args, 1), "");
    JsValue::UNDEFINED
}

fn open_edit(_this: JsValue, args: js_sys::Array) -> JsValue {
    open_with(
        &js::arg_str(&args, 0),
        "edit",
        &js::arg_str(&args, 2),
        &js::arg_str(&args, 1),
    );
    JsValue::UNDEFINED
}

fn close(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_num(&s, "requestId", js::get_num(&s, "requestId") + 1.0);
    js::set_bool(&s, "open", false);
    js::set_bool(&s, "confirming", false);
    js::set_str(&s, "error", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

fn segments(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let out = js_sys::Array::new();
    let form = mjs::store_ref(STORE);
    let name = js::get_str(&form, "name");
    let parent = if js::get_str(&form, "mode") == "create" { js::get_str(&form, "parentProject") } else { String::new() };
    let path = if parent.is_empty() { name } else { format!("{parent}/{name}") };
    for seg in path.split('/').map(str::trim).filter(|s| !s.is_empty()) {
        out.push(&JsValue::from_str(seg));
    }
    out.into()
}

fn invalid(name: &str) -> Option<String> {
    let parts: Vec<&str> = name
        .split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return Some("a project needs a name".into());
    }
    if parts.iter().any(|p| *p == "." || *p == "..") {
        return Some("a project name cannot contain . or .. segments".into());
    }
    if parts.len() > 8 {
        return Some("a project may be at most 8 levels deep".into());
    }
    None
}

fn send(s: &JsValue, action: &str, fields: &[(&str, JsValue)]) {
    let payload = js::obj();
    for (k, v) in fields {
        js::set(&payload, k, v);
    }
    js::call(
        &mjs::store_ref(&js::get_str(s, "store")),
        "saveTarget",
        &[JsValue::from_str(action), payload.into()],
    );
}

fn save(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    if js::get_bool(&s, "busy") || !ready(JsValue::UNDEFINED, js_sys::Array::new()).is_truthy() { return JsValue::UNDEFINED; }
    let kind = js::get_str(&s, "kind");
    js::set_str(&s, "error", "");
    if kind != "project" {
        let name = js::get_str(&s, "name").trim().to_string();
        let repo = js::get_str(&s, "repoId");
        match kind.as_str() {
            "repo" => send(
                &s,
                "repo-add",
                &[("path", JsValue::from_str(js::get_str(&s, "path").trim()))],
            ),
            "branch" => send(
                &s,
                "branch-add",
                &[
                    ("repoId", JsValue::from_str(&repo)),
                    ("name", JsValue::from_str(&name)),
                    ("from", JsValue::from_str("")),
                ],
            ),
            "worktree" => send(
                &s,
                "worktree-add",
                &[
                    ("repoId", JsValue::from_str(&repo)),
                    ("branch", JsValue::from_str(&name)),
                    ("create", JsValue::from_bool(js::get_bool(&s, "create"))),
                ],
            ),
            "automation" => {
                if !cron_shape_ok(&js::get_str(&s, "cron")) {
                    crate::components::validation_info::note("tform", "error", "that cron does not parse — five fields, like 0 9 * * 1-5");
                    mjs::redraw();
                    return JsValue::UNDEFINED;
                }
                let mut fields = vec![
                    ("name", JsValue::from_str(&name)),
                    ("cron", JsValue::from_str(js::get_str(&s, "cron").trim())),
                    ("prompt", JsValue::from_str(js::get_str(&s, "prompt").trim())),
                    ("agent", JsValue::from_str(js::get_str(&s, "agent").trim())),
                    ("model", JsValue::from_str(js::get_str(&s, "model").trim())),
                    ("timeoutMin", JsValue::from_f64(js::get_str(&s, "timeout").trim().parse().unwrap_or(60.0))),
                    ("enabled", JsValue::from_bool(js::get_bool(&s, "enabled"))),
                    ("catchUp", JsValue::from_bool(js::get_bool(&s, "catchUp"))),
                ];
                if js::get_str(&s, "mode") == "edit" {
                    let original = js::get_str(&s, "original");
                    fields.push(("id", JsValue::from_str(&original)));
                }
                send(&s, "automation-save", &fields)
            }
            _ => {}
        }
        return JsValue::UNDEFINED;
    }
    let editing = js::get_str(&s, "mode") == "edit";
    let entered = js::get_str(&s, "name");
    let parent = js::get_str(&s, "parentProject");
    if let Some(msg) = invalid(&entered) {
        crate::components::validation_info::note("tform", "error", &msg);
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    let name = if !editing && !parent.is_empty() { format!("{parent}/{}", entered.trim()) } else { entered };
    if let Some(msg) = invalid(&name) {
        crate::components::validation_info::note("tform", "error", &msg);
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    let host = mjs::store_ref(&js::get_str(&s, "store"));
    let original = js::get_str(&s, "original");
    let args = if editing {
        vec![
            JsValue::from_str("rename"),
            JsValue::from_str(&original),
            JsValue::from_str(&name),
        ]
    } else {
        vec![JsValue::from_str("create"), JsValue::from_str(&name)]
    };
    js::call(&host, "saveProject", &args);
    close(JsValue::UNDEFINED, js_sys::Array::new());
    JsValue::UNDEFINED
}

fn remove(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    if !js::get_bool(&s, "confirming") {
        js::set_bool(&s, "confirming", true);
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    let kind = js::get_str(&s, "kind");
    let original = js::get_str(&s, "original");
    if kind == "project" {
        js::call(
            &mjs::store_ref(&js::get_str(&s, "store")),
            "saveProject",
            &[JsValue::from_str("delete"), JsValue::from_str(&original)],
        );
        close(JsValue::UNDEFINED, js_sys::Array::new());
        return JsValue::UNDEFINED;
    }
    let action = match kind.as_str() {
        "repo" => "repo-remove",
        "worktree" => "worktree-remove",
        "sandbox" => "sandbox-remove",
        "automation" => "automation-remove",
        _ => return JsValue::UNDEFINED,
    };
    send(&s, action, &[("id", JsValue::from_str(&original))]);
    JsValue::UNDEFINED
}

fn toggle_bool(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let field = js::arg_str(&args, 0);
    if field != "enabled" && field != "catchUp" { return JsValue::UNDEFINED; }
    js::set_bool(&s, &field, !js::get_bool(&s, &field));
    mjs::redraw();
    JsValue::UNDEFINED
}

fn status_appearance(status: &str) -> (&'static str, &'static str) {
    match status.trim().to_ascii_lowercase().as_str() {
        "creating" | "cloning" | "starting" => ("circle-notch", "is-busy"),
        "ready" => ("check-circle", "is-ready"),
        "stopped" => ("stop-circle", "is-idle"),
        "error" | "failed" => ("x-circle", "is-error"),
        "service unavailable" | "not found" => ("warning-circle", "is-warning"),
        _ => ("question", "is-idle"),
    }
}

fn status_icon(_this: JsValue, args: js_sys::Array) -> JsValue {
    JsValue::from_str(status_appearance(&js::arg_str(&args, 0)).0)
}

fn status_label(_this: JsValue, args: js_sys::Array) -> JsValue {
    let status = js::arg_str(&args, 0);
    JsValue::from_str(if status == "ready" { "running" } else { &status })
}

fn status_class(_this: JsValue, args: js_sys::Array) -> JsValue {
    JsValue::from_str(status_appearance(&js::arg_str(&args, 0)).1)
}

pub(crate) fn sync_sandbox_status(host: &JsValue) {
    let form = mjs::store_ref(STORE);
    if !js::get_bool(&form, "open") || js::get_str(&form, "kind") != "sandbox"
        || js::get_str(&form, "store") != "live" { return; }
    let warning = if js::get_bool(host, "sboxUp") { "" } else { "The sandbox service is not answering." };
    if js::get_str(&form, "note") != warning { js::set_str(&form, "note", warning); }
    if js::get_str(&form, "mode") != "edit" { return; }
    let id = js::get_str(&form, "original");
    let rows = js_sys::Array::from(&js::get(host, "sandboxes"));
    let status = if !js::get_bool(host, "sboxUp") { "Service unavailable".to_string() }
        else { rows.iter().find(|row| js::get_str(row, "id") == id)
            .map(|row| js::get_str(&row, "state"))
            .filter(|state| !state.is_empty()).unwrap_or_else(|| "Not found".to_string()) };
    if js::get_str(&form, "status") != status { js::set_str(&form, "status", &status); }
}

pub fn register() {
    css::inject("target-form", CSS);
    let mut d = Def::new().named("store tform");
    d.bool("open", false)
        .str("kind", "project")
        .str("mode", "create")
        .str("store", "live")
        .str("name", "")
        .str("original", "")
        .str("parentProject", "")
        .str("heading", "")
        .str("icon", "folder-plus")
        .str("note", "")
        .str("dangerNote", "")
        .str("error", "")
        .str("path", "")
        .str("repoId", "")
        .str("repoName", "")
        .str("image", "")
        .str("status", "")
        .str("agentName", "")
        .str("agentIcon", "")
        .str("agentAccent", "")
        .str("createdAt", "")
        .str("cron", "")
        .str("prompt", "")
        .str("agent", "")
        .str("model", "")
        .str("timeout", "60")
        .bool("enabled", true)
        .bool("catchUp", false)
        .num("requestId", 0.0)
        .bool("create", true)
        .bool("busy", false)
        .bool("confirming", false)
        .method("statusIcon", status_icon)
        .method("statusClass", status_class)
        .method("statusLabel", status_label)
        .method("toggleBool", toggle_bool)
        .method("cronDescription", |this, _args| JsValue::from_str(&crate::cron::description_of(&js::get_str(&this, "cron"))))
        .method("cronNext", |this, _args| JsValue::from_str(&crate::cron::next_line_of(&js::get_str(&this, "cron"))))
        .method("openCreate", open_create)
        .method("openEdit", open_edit)
        .method("close", close)
        .method("save", save)
        .method("remove", remove)
        .method("segments", segments)
        .method("repos", repos)
        .method("projects", projects)
        .method("pickParent", pick_parent)
        .method("agentItems", agent_items)
        .method("pickAgent", pick_agent)
        .method("ready", ready)
        .method("deletable", deletable);
    mjs::store(STORE, &d.finish());
}

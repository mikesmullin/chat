use wasm_bindgen::JsValue;

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::components::{picker, presence, validation_info};
use crate::css;

const STORE: &str = "sessionDraft";
const ST: &str = "$store.sessionDraft";

const KNOWN: &[(&str, &str, &str)] = &[
    ("researcher", "flask", "#0091ff"),
    ("joker", "mask-happy", "#be5cff"),
    ("fish", "fish", "#3af89f"),
    ("homie", "house", "#de4a91"),
    ("phony", "phone", "#2bc5c2"),
    ("bowser", "browser", "#96949e"),
    ("dummy", "lego-smiley", "#eeff00"),
];

pub const CSS: &str = r#"
:is(#sb, #home) .se-participants { display: flex; flex-direction: column; gap: 8px; }
:is(#sb, #home) .se-invite { display: flex; align-items: center; gap: 8px; }
:is(#sb, #home) .se-invite input {
  flex: 1 1 auto; min-width: 0; box-sizing: border-box;
  background: var(--panel-2); color: var(--text);
  border: 1px solid transparent; border-radius: 8px;
  padding: 7px 9px; font: inherit; line-height: 1.5;
}
:is(#sb, #home) .se-invite input:focus { outline: none; border-color: var(--gray-dim); }
:is(#sb, #home) .se-invite input::placeholder { color: var(--gray-dim); }
:is(#sb, #home) .se-invite-field { position: relative; flex: 1 1 auto; min-width: 0; }
:is(#sb, #home) .se-invite-field > i {
  position: absolute; left: 9px; top: 50%; transform: translateY(-50%);
  font-size: 13px; color: var(--gray-dim); pointer-events: none;
}
:is(#sb, #home) .se-invite-field input { padding-left: 30px; }

:is(#sb, #home) .se-worktree { display: flex; flex-direction: column; gap: 8px; }
:is(#sb, #home) .se-worktree .picker { display: flex; width: 100%; }
:is(#sb, #home) .se-worktree .dropdown-btn {
  flex: 1 1 auto; box-sizing: border-box; width: 100%;
  background: var(--panel-2); color: var(--fg);
  border: 1px solid transparent; border-radius: 8px;
  padding: 7px 9px; font-size: 13px; line-height: 1.5;
}
:is(#sb, #home) .se-worktree .dropdown-btn:hover { background: var(--panel-2); border-color: transparent; }
:is(#sb, #home) .se-worktree .dropdown-value { flex: 1 1 auto; color: var(--fg); }
:is(#sb, #home) .se-worktree .picker .picker-menu,
:is(#sb, #home) .se-project .picker .picker-menu { left: auto; right: 0; }
:is(#sb, #home) .se-project .dropdown { display: flex; width: 100%; }
:is(#sb, #home) .se-project .dropdown-btn {
  flex: 1 1 auto; box-sizing: border-box; width: 100%;
  background: var(--panel-2); color: var(--fg);
  border: 1px solid transparent; border-radius: 8px;
  padding: 7px 9px; font-size: 13px; line-height: 1.5;
}
:is(#sb, #home) .se-project .dropdown-btn:hover { background: var(--panel-2); border-color: transparent; }
:is(#sb, #home) .se-project .dropdown-value { flex: 1 1 auto; color: var(--fg); }
:is(#sb, #home) .se-worktree-path {
  margin: 0; font-family: var(--mono); font-size: 11px; line-height: 1.6;
  color: var(--gray-dim); overflow-wrap: anywhere;
}
:is(#sb, #home) .se-worktree-actions { display: flex; }

.se-editor-demo { width: 340px; }
"#;

pub(crate) fn field(label: &str, hint: &str, model: &str, placeholder: &str, rows: usize) -> Node {
    let input = if rows == 0 {
        m("input")
            .attr("type", "text")
            .attr("placeholder", placeholder)
            .model(model)
    } else {
        m("textarea")
            .attr("rows", &rows.to_string())
            .attr("placeholder", placeholder)
            .model(model)
    };
    let input = if let Some(key) = model.strip_prefix("$store.sessionDraft.") {
        if ["purpose", "notes", "workspaceRoot", "soulAppendage"].contains(&key) {
            input.bind("placeholder", &format!("({model} ? '{}' : ($store.sessionDraft.inherited.{key} ? '-' : '{}'))", placeholder, placeholder))
        } else { input }
    } else { input };
    m("div").class("ae-field").kids(vec![
        m("label").class("ae-label").kids(vec![
            m("span").child(lit(label)),
            m("span").class("hint").child(lit(hint)),
        ]),
        input,
    ])
}

fn mode(draft: &str, value: &str, label: &str, icon: &str) -> Node {
    m("button")
        .class("ae-mode")
        .attr("type", "button")
        .bind("class", &format!("({draft}.routing === '{value}' ? 'is-on' : '')"))
        .on("click", &format!("{draft}.setRouting('{value}')"))
        .kids(vec![m("i").class(&format!("ph {icon}")), m("span").child(lit(label))])
}

pub(crate) fn routing_modes(draft: &str) -> Node {
    m("div").class("ae-modes").kids(vec![
        mode(draft, "", "Inherit", "ph-arrow-up"),
        mode(draft, "everyone", "Everyone", "ph-chats-circle"),
        mode(draft, "lead", "Lead", "ph-megaphone"),
    ])
}

fn participants() -> Node {
    let seat = presence::seat(
        "s.handle",
        "s.icon",
        "s.accentColor",
        "!s.active",
        "(100 - i)",
    )
    .for_each(&format!("(s, i) in {ST}.seats"))
    .key("s.id");

    m("div").class("se-participants").kids(vec![
        m("label").class("ae-label").kids(vec![
            m("span").child(lit("Participants")),
            m("span").class("hint").child(lit("seated, then unseated — they keep their messages")),
        ]),
        presence::seats(vec![seat]),
        m("div").class("se-invite").kids(vec![
            m("div").class("se-invite-field").kids(vec![
                Node::icon("magnifying-glass"),
                m("input")
                    .attr("type", "text")
                    .attr("placeholder", "@name")
                    .attr("spellcheck", "false")
                    .model(&format!("{ST}.inviteHandle"))
                    .on("keydown.enter", &format!("{ST}.invite()")),
            ]),
            m("button")
                .class("ae-btn primary")
                .attr("type", "button")
                .on("click", &format!("{ST}.invite()"))
                .kids(vec![Node::icon("user-plus"), m("span").child(lit("Invite"))]),
        ]),
    ])
}

fn project_block(st: &str) -> Node {
    m("div").class("ae-field se-project").kids(vec![
        m("label").class("ae-label").kids(vec![
            m("span").child(lit("Project")),
            m("span").class("hint").child(lit("the folder this conversation is filed under")),
        ]),
        picker::project(st),
    ])
}

pub(crate) fn worktree_config(st: &str, draft: &str) -> Vec<Node> {
    vec![
        m("label").class("ae-label").kids(vec![
            m("span").child(lit("Worktree")),
            m("span").class("hint").child(lit("optional — pick a repo, or none")),
        ]),
        picker::repo(st, draft),
        field("Branch pattern", "the branch to check out; {{hex}} draws a fresh suffix", &format!("{draft}.worktreeBranchPattern"), "chat/{{hex}}", 0).show(&format!("{draft}.worktreeRepoId")),
        field("Path pattern", "where the checkout lands", &format!("{draft}.worktreePathPattern"), "{{parent}}/{{repo}}/{{prefix}}-{{hash}}", 0).show(&format!("{draft}.worktreeRepoId")),
    ]
}

fn worktree_section(st: &str) -> Node {
    let readout = format!("({ST}.worktreeRepoId ? ({st}.session.worktree && {st}.session.worktree.provisioned ? {st}.session.worktree.path : 'will be provisioned on save') : ({ST}.inherited.worktree ? 'Inherited worktree' : 'no worktree — this session does not manage git'))");
    let mut children = worktree_config(st, ST);
    children.push(m("p").class("se-worktree-path").text(&readout));
    children.push(m("div").class("se-worktree-actions").kids(vec![
        m("button").class("ae-btn").attr("type", "button")
            .show(&format!("({st}.session.worktree && {st}.session.worktree.provisioned)"))
            .on("click", &format!("{ST}.removeWorktree()"))
            .kids(vec![Node::icon("trash"), m("span").child(lit("Remove worktree"))]),
    ]));
    m("div").class("se-worktree").kids(children)
}

pub fn panel(st: &str) -> Node {
    m("div")
        .class("agent-editor session-editor")
        .show(&format!("{ST}.open"))
        .kids(vec![
        m("div").class("agent-editor-head").kids(vec![
            Node::icon("pencil-simple"),
            m("span").class("agent-editor-title").child(lit("Edit session")),
        ]),
        validation_info::live(
            &format!("({ST}.messageTone ? ' ' + {ST}.messageTone : '')"),
            &format!("{ST}.message"),
        ),
        field(
            "Title",
            "the tab's tooltip; the tab itself shows the number",
            &format!("{ST}.title"),
            "iran war status",
            0,
        ),
        field(
            "Purpose",
            "one line: what this conversation is for",
            &format!("{ST}.purpose"),
            "track the ceasefire talks",
            0,
        ),
        field("Notes", "free text", &format!("{ST}.notes"), "", 3),
        field("Soul appendage", "added to each agent's soul; blank inherits", &format!("{ST}.soulAppendage"), "", 4),
        m("div").class("ae-field").kids(vec![
            m("label").class("ae-label").kids(vec![
                m("span").child(lit("Routing")),
                m("span")
                    .class("hint")
                    .child(lit("who takes a turn in a group round")),
            ]),
            routing_modes(ST),
            m("p").class("ae-mode-note").text(&format!("{ST}.routingNote()")),
        ]),
        m("div").class("ae-field").kids(vec![
            m("label").class("ae-label").kids(vec![
                m("span").child(lit("Memory budget")),
                m("span").class("hint").child(lit("bytes before a thread is summarised")),
            ]),
            m("input")
                .class("ae-num")
                .attr("type", "number")
                .bind("placeholder", &format!("({ST}.memoryHigh ? '512' : ({ST}.inherited.memoryHigh ? '-' : '512'))"))
                .model(&format!("{ST}.memoryHigh")),
        ]),
        project_block(st),
        field(
            "Workspace root",
            "optional — absolute path; blank falls back to the project root",
            &format!("{ST}.workspaceRoot"),
            "/workspace/vibe-arcade",
            0,
        ),
        worktree_section(st),
        participants(),
        m("div").class("ae-actions").kids(vec![
            m("span")
                .class("ae-btn primary")
                .attr("title", "Save")
                .on("click", &format!("{ST}.save()"))
                .child(m("span").child(lit("Save"))),
            m("button")
                .class("ae-btn")
                .attr("type", "button")
                .on("click", "$store.share.openForActive()")
                .kids(vec![
                    Node::icon("users-three").class("ph ph-users-three"),
                    m("span").child(lit("Share…")),
                ]),
            m("span").class("spacer"),
            m("button")
                .class("ae-btn danger")
                .attr("type", "button")
                .bind("class", &format!("({ST}.confirmDelete ? 'is-confirm' : '')"))
                .on("click", &format!("{ST}.delete()"))
                .kids(vec![
                    Node::icon("trash").class("ph ph-trash"),
                    m("span").text(&format!(
                        "({ST}.confirmDelete ? 'Really delete?' : 'Delete')"
                    )),
                ]),
        ]),
    ])
}

pub fn demo() -> Node {
    m("div")
        .class("se-editor-demo")
        .init("$store.sessionDraft.open = true; $store.sessionDraft.seed($store.samples.session, 'samples'); $store.sessionDraft.workspaceRoot = ''; $store.sessionDraft.inherited = { workspaceRoot: '/workspace/example' }")
        .child(panel(crate::components::samples::EXPR))
}

fn say(s: &JsValue, tone: &str, message: &str) {
    js::set_str(s, "messageTone", tone);
    crate::components::validation_info::note(STORE, "message", message);
}

fn invite(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let raw = js::get_str(&s, "inviteHandle").trim().to_string();
    let handle = raw.trim_start_matches('@').trim().to_lowercase();
    if handle.is_empty() {
        say(&s, "error", "Name is required");
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    let Some((_, icon, accent)) = KNOWN.iter().find(|(n, _, _)| *n == handle) else {
        say(&s, "error", &format!("no agent named {handle}"));
        mjs::redraw();
        return JsValue::UNDEFINED;
    };

    let seats = js_sys::Array::from(&js::get(&s, "seats"));
    let mut max_id = 0.0;
    for i in 0..seats.length() {
        let p = seats.get(i);
        let id = js::get(&p, "id").as_f64().unwrap_or(0.0);
        if id > max_id {
            max_id = id;
        }
        if js::get_str(&p, "handle") == handle {
            js::set_bool(&p, "active", true);
            js::set_str(&s, "inviteHandle", "");
            say(&s, "ok", &format!("{handle} is already here"));
            mjs::redraw();
            return JsValue::UNDEFINED;
        }
    }

    let p = js::obj();
    js::set_num(&p, "id", max_id + 1.0);
    js::set_str(&p, "handle", &handle);
    js::set_str(&p, "icon", icon);
    js::set_str(&p, "accentColor", accent);
    js::set_bool(&p, "active", true);
    seats.push(&p);
    js::set(&s, "seats", &seats.into());
    js::set_str(&s, "inviteHandle", "");
    say(&s, "ok", &format!("{handle} joined"));
    mjs::redraw();
    JsValue::UNDEFINED
}

fn set_routing(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_str(&s, "routing", &js::arg_str(&args, 0));
    mjs::redraw();
    JsValue::UNDEFINED
}

fn routing_note(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let note = if js::get_str(&s, "routing") == "lead" {
        "one agent decides who speaks each round"
    } else {
        "every seated agent takes a turn"
    };
    JsValue::from_str(note)
}

fn save(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let title = js::get_str(&s, "title").trim().to_string();
    if title.is_empty() {
        say(&s, "error", "Title is required");
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    let live = mjs::store_ref("live");
    let id = js::get_str(&live, "activeId");
    if id.is_empty() {
        say(&s, "error", "No active session");
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    let payload = js::obj();
    js::set_str(&payload, "title", &title);
    js::set_str(&payload, "purpose", &js::get_str(&s, "purpose").trim().to_string());
    js::set_str(&payload, "notes", &js::get_str(&s, "notes").trim().to_string());
    js::set_str(&payload, "soulAppendage", &js::get_str(&s, "soulAppendage"));
    js::set_str(&payload, "routing", &js::get_str(&s, "routing"));
    let mh = js::get(&s, "memoryHigh");
    if mh.is_null() || mh.is_undefined() || mh.as_f64().is_none() {
        js::set(&payload, "memoryHigh", &JsValue::NULL);
    } else {
        js::set(&payload, "memoryHigh", &mh);
    }
    js::set_str(&payload, "workspaceRoot", &js::get_str(&s, "workspaceRoot").trim().to_string());
    let repo = js::get_str(&s, "worktreeRepoId").trim().to_string();
    if repo.is_empty() {
        js::set(&payload, "worktree", &JsValue::NULL);
    } else {
        let wt = js::obj();
        js::set_str(&wt, "repoId", &repo);
        js::set_str(&wt, "branchPattern", &js::get_str(&s, "worktreeBranchPattern").trim().to_string());
        js::set_str(&wt, "pathPattern", &js::get_str(&s, "worktreePathPattern").trim().to_string());
        js::set(&payload, "worktree", &wt);
    }
    rustex1_kit::api::post_json(
        &format!("/api/sessions/{id}/settings"),
        &payload.into(),
        move |v| {
            let live = mjs::store_ref("live");
            let s = mjs::store_ref(STORE);
            if js::get(&v, "ok").as_bool() == Some(false) {
                say(&s, "error", &js::get_str(&v, "error"));
                mjs::redraw();
                return;
            }
            if js::get_str(&live, "activeId") == id {
                js::set(&live, "session", &js::get(&v, "session"));
            }
            js::call(&mjs::store_ref("live"), "loadSessions", &[]);
            say(&s, "ok", "Saved");
            mjs::redraw();
        },
    );
    JsValue::UNDEFINED
}

fn close(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "open", false);
    js::set_bool(&s, "confirmDelete", false);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn delete(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let live = mjs::store_ref("live");
    let id = js::get_str(&live, "activeId");
    if id.is_empty() {
        say(&s, "error", "No active session");
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    if !js::get_bool(&s, "confirmDelete") {
        js::set_bool(&s, "confirmDelete", true);
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    js::set_bool(&s, "confirmDelete", false);
    js::call(&live, "deleteSession", &[JsValue::from_str(&id)]);
    js::set_bool(&s, "open", false);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn seed(_this: JsValue, args: js_sys::Array) -> JsValue {
    let meta = args.get(0);
    let from = args.get(1).as_string().unwrap_or_else(|| "live".to_string());
    let s = mjs::store_ref(STORE);
    let mut title = js::get_str(&meta, "title");
    if title.is_empty() {
        let row_id = js::get_str(&meta, "id");
        for r in js_sys::Array::from(&js::get(&mjs::store_ref(&from), "sessions")).iter() {
            if js::get_str(&r, "id") == row_id {
                title = js::get_str(&r, "title");
                break;
            }
        }
        if title.is_empty() { title = row_id; }
    }
    js::set_str(&s, "title", &title);
    js::set_str(&s, "purpose", &js::get_str(&meta, "purpose"));
    js::set_str(&s, "notes", &js::get_str(&meta, "notes"));
    js::set_str(&s, "soulAppendage", &js::get_str(&meta, "soulAppendage"));
    let routing = js::get_str(&meta, "routing");
    js::set_str(&s, "routing", &routing);
    let mh = js::get(&meta, "memoryHigh");
    if mh.is_null() || mh.is_undefined() || mh.as_f64().is_none() {
        js::set(&s, "memoryHigh", &JsValue::NULL);
    } else {
        js::set(&s, "memoryHigh", &mh);
    }
    js::set_str(&s, "workspaceRoot", &js::get_str(&meta, "workspaceRoot"));
    let wt = js::get(&meta, "worktree");
    js::set_str(&s, "worktreeRepoId", &js::get_str(&wt, "repoId"));
    let bp = js::get_str(&wt, "branchPattern");
    js::set_str(&s, "worktreeBranchPattern", if bp.trim().is_empty() { "chat/{{hex}}" } else { &bp });
    let pp = js::get_str(&wt, "pathPattern");
    js::set_str(&s, "worktreePathPattern", if pp.trim().is_empty() { "{{parent}}/{{repo}}/{{prefix}}-{{hash}}" } else { &pp });
    js::set_str(&s, "message", "");
    js::set_str(&s, "messageTone", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

fn pick_repo(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let id = args.get(0).as_string().unwrap_or_default();
    js::set_str(&s, "worktreeRepoId", &id);
    js::call(&mjs::store_ref("dropdown"), "closeIf", &[JsValue::from_str("repo")]);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn remove_worktree(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let live = mjs::store_ref("live");
    let id = js::get_str(&live, "activeId");
    if id.is_empty() {
        say(&s, "error", "No active session");
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    let payload = js::obj();
    js::set(&payload, "worktree", &JsValue::NULL);
    rustex1_kit::api::post_json(&format!("/api/sessions/{id}/settings"), &payload.into(), move |v| {
        let live = mjs::store_ref("live");
        let s = mjs::store_ref(STORE);
        if js::get(&v, "ok").as_bool() == Some(false) {
            say(&s, "error", &js::get_str(&v, "error"));
        } else {
            if js::get_str(&live, "activeId") == id {
                js::set(&live, "session", &js::get(&v, "session"));
            }
            say(&s, "ok", "Worktree removed");
        }
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

fn store_register() {
    let seats = js_sys::Array::new();
    for (i, (name, icon, accent)) in KNOWN.iter().enumerate() {
        let p = js::obj();
        js::set_num(&p, "id", (i + 1) as f64);
        js::set_str(&p, "handle", name);
        js::set_str(&p, "icon", icon);
        js::set_str(&p, "accentColor", accent);
        js::set_bool(&p, "active", i + 1 < KNOWN.len());
        seats.push(&p);
    }
    let mut d = Def::new().named("store sessionDraft");
    d.bool("open", false)
        .bool("confirmDelete", false)
        .val("inherited", &js::obj().into())
        .str("title", "iran war status")
        .str("purpose", "")
        .str("notes", "")
        .str("soulAppendage", "")
        .str("routing", "everyone")
        .str("memoryHigh", "")
        .str("workspaceRoot", "")
        .str("worktreeRepoId", "")
        .str("worktreeBranchPattern", "chat/{{hex}}")
        .str("worktreePathPattern", "{{parent}}/{{repo}}/{{prefix}}-{{hash}}")
        .str("inviteHandle", "")
        .str("message", "")
        .str("messageTone", "")
        .val("seats", &seats.into())
        .method("invite", invite)
        .method("seed", seed)
        .method("pickRepo", pick_repo)
        .method("removeWorktree", remove_worktree)
        .method("setRouting", set_routing)
        .method("routingNote", routing_note)
        .method("save", save)
        .method("delete", delete)
        .method("close", close);
    mjs::store(STORE, &d.finish());
}

pub fn register() {
    css::inject("session-editor", CSS);
    store_register();
}

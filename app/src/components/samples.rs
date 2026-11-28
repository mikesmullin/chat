use wasm_bindgen::JsValue;

use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

pub const STORE: &str = "samples";
pub const EXPR: &str = "$store.samples";

pub fn row(kind: &str) -> String {
    format!("{EXPR}.row('{kind}')")
}

struct Row {
    id: &'static str,
    kind: &'static str,
    text: &'static str,
    handle: &'static str,
    turn: bool,
    first: bool,
    last: bool,
    open: bool,
    name: &'static str,
    from: &'static str,
}

const BLANK: Row = Row {
    id: "",
    kind: "",
    text: "",
    handle: "",
    turn: false,
    first: false,
    last: false,
    open: false,
    name: "",
    from: "",
};

fn rows() -> Vec<Row> {
    vec![
        Row {
            id: "u1",
            kind: "user",
            text: "Read https://example.com/config and tell me which model it points at.",
            ..BLANK
        },
        Row {
            id: "u2",
            kind: "user",
            from: "Alice",
            text: "While you are in there — what is the context window set to?",
            ..BLANK
        },
        Row {
            id: "sp1",
            kind: "system_prompt",
            handle: "fish",
            turn: true,
            first: true,
            text: "Your name is fish.\n\nYou answer about this project's own \
                   configuration from the files, not from memory.",
            ..BLANK
        },
        Row {
            id: "agent-prompt-1",
            kind: "agent_prompt",
            handle: "fish",
            turn: true,
            text: "--- Sun, Nov 15 ---\n09:12 [Bob] @fish Read the config and \
                   tell me which model it points at.",
            ..BLANK
        },
        Row {
            id: "d1",
            kind: "decision",
            handle: "fish",
            turn: true,
            text: "",
            ..BLANK
        },
        Row {
            id: "r1",
            kind: "reasoning",
            handle: "fish",
            turn: true,
            open: true,
            text: "They want the default, not the session override — so the \
                   config file, not the composer. One read should settle it.",
            ..BLANK
        },
        Row {
            id: "c1",
            kind: "tool_call",
            handle: "fish",
            turn: true,
            name: "file_io__read_file",
            ..BLANK
        },
        Row {
            id: "t1",
            kind: "tool_result",
            handle: "fish",
            turn: true,
            open: true,
            name: "file_io__read_file",
            text: "default_model: muse:opus\ncontext_windows:\n  muse:\n    opus: 200000",
            ..BLANK
        },
        Row {
            id: "a1",
            kind: "assistant",
            handle: "fish",
            turn: true,
            last: true,
            text: "It points at `muse:opus`, with a **200k** context window.\n\n\
                   Nothing in the session overrides it, so that is what the \
                   next turn will use. See https://example.com/models for details.\n\n\
                   | Setting | Value |\n|---|---|\n| Model | `muse:opus` |\n| Context | **200k** |",
            ..BLANK
        },
        Row {
            id: "ap1",
            kind: "approval",
            handle: "fish",
            turn: true,
            open: true,
            name: "term_local__bash",
            text: "rm -rf build/",
            ..BLANK
        },
        Row {
            id: "ap-done",
            kind: "approval",
            handle: "fish",
            turn: true,
            name: "file_io__write_file",
            text: "{\"path\": \"src/auth.py\"}",
            ..BLANK
        },
        Row {
            id: "e1",
            kind: "error",
            text: "Error: dial tcp 127.0.0.1:1237: connection refused",
            ..BLANK
        },
        Row {
            id: "s1",
            kind: "system",
            text: "Invited @joker",
            ..BLANK
        },
    ]
}

fn queue_sample() -> JsValue {
    let items = js_sys::Array::new();
    for (id, text) in [
        ("q-sample-1", "re-record the spike trap demo with narration"),
        ("q-sample-2", "and bump the tomb's ember light"),
    ] {
        let item = js::obj();
        js::set_str(&item, "id", id);
        js::set_str(&item, "text", text);
        js::set_str(&item, "reason", "waiting for the session's workspace root and worktree");
        items.push(&item);
    }
    let q = js::obj();
    js::set(&q, "items", &items.into());
    js::set_str(
        &q,
        "reason",
        "waiting for the session's workspace root and worktree",
    );
    q.into()
}

fn dequeue_sample(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    let queue = js::get(&s, "queue");
    let items = js_sys::Array::new();
    let mut text = String::new();
    for row in js_sys::Array::from(&js::get(&queue, "items")).iter() {
        if js::get_str(&row, "id") == id {
            text = js::get_str(&row, "text");
        } else {
            items.push(&row);
        }
    }
    let next = js::obj();
    js::set(&next, "items", &items.into());
    js::set_str(&next, "reason", &js::get_str(&queue, "reason"));
    js::set(&s, "queue", &next.into());
    if !text.is_empty() {
        js::set_str(&s, "draft", &text);
    }
    mjs::redraw();
    JsValue::UNDEFINED
}

fn messages() -> JsValue {
    let out = js_sys::Array::new();
    let now = js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default();
    for r in rows() {
        let o = js::obj();
        js::set_str(&o, "id", r.id);
        js::set_str(&o, "eventId", r.id);
        js::set_str(&o, "kind", r.kind);
        if r.kind == "system" { js::set_bool(&o, "commandNotice", true); }
        js::set_str(&o, "text", r.text);
        js::set_str(&o, "handle", r.handle);
        js::set_str(&o, "ts", &now);
        js::set_bool(&o, "turn", r.turn);
        js::set_bool(&o, "turnFirst", r.first);
        js::set_bool(&o, "turnLast", r.last);
        js::set_bool(&o, "expanded", r.open);
        js::set_bool(&o, "ok", true);
        js::set_str(&o, "from", r.from);
        if !r.name.is_empty() {
            js::set_str(&o, "name", r.name);
        }
        if r.kind == "tool_call" {
            let args = js::obj();
            js::set_str(&args, "path", "~/.config/agl/config.yaml");
            js::set(&o, "args", &args);
        }
        if r.kind == "approval" {
            js::set_str(&o, "command", r.text);
            js::set_str(&o, "status", if r.id == "ap-done" { "always" } else { "pending" });
        }
        if r.kind == "agent_prompt" {
            js::set_num(&o, "historyTurns", 2.0);
        }
        if r.kind == "decision" {
            let verdict = js::obj();
            js::set_bool(&verdict, "speak", true);
            js::set_num(&verdict, "confidence", 0.92);
            js::set_str(&verdict, "why", "addressed by name");
            js::set(&o, "verdict", &verdict);
            js::set_bool(&o, "speak", true);
            js::set_str(&o, "reason", "addressed by name");
            js::set_num(&o, "confidence", 0.92);
        }
        if matches!(r.kind, "user" | "assistant" | "reasoning") {
            js::set_str(&o, "html", &crate::helpers::markdown_to_html(r.text));
        }
        out.push(&o);
    }
    out.into()
}

const SESSIONS: &[(f64, &str, &str, &str, &str, f64, f64, f64, f64)] = &[
    (
        1.0,
        "samples",
        "Which model does the config point at?",
        "ready",
        "psicom/chat",
        6.0,
        12.0,
        200000.0,
        1975.0,
    ),
    (
        2.0,
        "s-2",
        "Migrate the auth service",
        "running",
        "psicom/chat",
        3.0,
        4.0,
        200000.0,
        82400.0,
    ),
    (
        3.0,
        "s-3",
        "Retry logic refactor",
        "ready",
        "psicom/sbox",
        9.0,
        45.0,
        200000.0,
        168000.0,
    ),
    (
        4.0,
        "s-4",
        "Scratch: regex for the log parser",
        "ready",
        "",
        0.0,
        130.0,
        200000.0,
        0.0,
    ),
];

fn models() -> JsValue {
    let a = js_sys::Array::new();
    for m in ["muse:opus", "muse:sonnet", "llama-server:default"] {
        a.push(&JsValue::from_str(m));
    }
    a.into()
}

fn sessions() -> JsValue {
    let out = js_sys::Array::new();
    let now = js_sys::Date::new_0();
    let iso = |d: &js_sys::Date| d.to_iso_string().as_string().unwrap_or_default();
    for (ordinal, id, title, status, project, turns, minutes, context_window, total_tokens) in SESSIONS {
        let o = js::obj();
        js::set_num(&o, "ordinal", *ordinal);
        js::set_str(&o, "id", id);
        js::set_str(&o, "title", title);
        js::set_str(&o, "status", status);
        js::set_str(&o, "project", project);
        js::set_str(&o, "agent", "fish");
        js::set_str(&o, "model", "muse:opus");
        js::set_num(&o, "turns", *turns);
        js::set_num(&o, "contextWindow", *context_window);
        js::set_num(&o, "totalTokens", *total_tokens);
        let created = js_sys::Date::new_0();
        created.set_time(now.get_time() - minutes * 60_000.0);
        js::set_str(&o, "createdAt", &iso(&created));
        js::set_str(&o, "updatedAt", &iso(&created));
        js::set_str(&o, "lastEventAt", &iso(&created));
        out.push(&o);
    }
    out.into()
}

fn targets(rows: &[(&str, &str, &str, &str)]) -> JsValue {
    let out = js_sys::Array::new();
    for (id, name, icon, hint) in rows {
        let o = js::obj();
        js::set_str(&o, "id", id);
        js::set_str(&o, "name", name);
        js::set_str(&o, "icon", icon);
        js::set_str(&o, "hint", hint);
        if *id == "psicom/chat" {
            let props = js::obj();
            js::set_str(&props, "purpose", "Build the chat experience");
            js::set_str(&props, "workspaceRoot", "/workspace/example");
            js::set(&o, "props", &props.into());
        }
        out.push(&o);
    }
    out.into()
}

fn sandbox_rows() -> JsValue {
    const ROWS: &[(&str, &str, &str, &str, &str)] = &[
        ("sandbox2-4f1e88a1", "sbox-video:latest", "ready", "s-2", "video"),
        ("sandbox2-9c02d7b4", "sbox-video:latest", "starting", "s-1", "video"),
        ("sandbox2-b71ae305", "sbox-chewy:latest", "stopped", "", ""),
    ];
    let out = js_sys::Array::new();
    for (id, image, state, session, agent) in ROWS {
        let o = js::obj();
        js::set_str(&o, "id", id);
        js::set_str(&o, "image", image);
        js::set_str(&o, "state", state);
        js::set_str(&o, "session", session);
        js::set_str(&o, "agent", agent);
        out.push(&o);
    }
    out.into()
}

const AUTOMATIONS: &[(&str, &str, &str, &str, &str, &str, f64, bool)] = &[
    ("a-triage", "issue-triage", "0 9 * * 1-5", "Triage new issues", "scout", "", 60.0, true),
    ("a-sweep", "nightly-sweep", "0 2 * * *", "Sweep stale branches", "", "", 30.0, false),
];

fn sample_automations() -> JsValue {
    let out = js_sys::Array::new();
    for (id, name, cron, prompt, agent, model, timeout, enabled) in AUTOMATIONS {
        let o = js::obj();
        js::set_str(&o, "id", id);
        js::set_str(&o, "name", name);
        js::set_str(&o, "cron", cron);
        js::set_str(&o, "prompt", prompt);
        js::set_str(&o, "agent", agent);
        js::set_str(&o, "model", model);
        js::set(&o, "timeoutMin", &JsValue::from_f64(*timeout));
        js::set(&o, "enabled", &JsValue::from_bool(*enabled));
        js::set(&o, "nextRun", &JsValue::from_f64(0.0));
        out.push(&o);
    }
    out.into()
}

const PROJECTS: &[(&str, &str, &str, &str)] = &[
    ("psicom/chat", "psicom/chat", "brackets-angle", ""),
    ("psicom/sbox", "psicom/sbox", "brackets-angle", ""),
    ("scratch", "scratch", "brackets-angle", ""),
];

const REPOS: &[(&str, &str, &str, &str)] = &[
    ("r-chat", "chat", "folder-open", "main"),
    ("r-angela", "angela-rs", "folder-open", "rust"),
];

const WORKTREES: &[(&str, &str, &str, &str)] = &[
    ("wt-onboarding", "onboarding-flow", "tree-structure", "psicom/chat"),
    ("wt-billing", "billing-replay", "tree-structure", "psicom/chat"),
];

const BRANCHES: &[(&str, &str, &str, &str)] = &[
    ("br-chat-main", "main", "git-branch", "psicom/chat"),
    ("br-chat-sbox", "sbox-a", "git-branch", "psicom/chat"),
    ("br-angela-main", "main", "git-branch", "angela-rs"),
    ("br-angela-rust", "rust", "git-branch", "angela-rs"),
    ("br-agl-rust", "rust", "git-branch", "agl-rs"),
];

fn create_target(_this: JsValue, args: js_sys::Array) -> JsValue {
    js::call(
        &mjs::store_ref("tform"),
        "openCreate",
        &[
            JsValue::from_str(&js::arg_str(&args, 0)),
            JsValue::from_str(STORE),
        ],
    );
    JsValue::UNDEFINED
}

fn edit_target(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_str(&s, "sboxConfirmId", "");
    js::set_str(&s, "sboxError", "");
    js::call(
        &mjs::store_ref("tform"),
        "openEdit",
        &[
            JsValue::from_str(&js::arg_str(&args, 0)),
            JsValue::from_str(&js::arg_str(&args, 1)),
            JsValue::from_str(STORE),
        ],
    );
    JsValue::UNDEFINED
}

fn drop_session(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    let project = js::arg_str(&args, 1);
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let sessions = js_sys::Array::from(&js::get(&mjs::store_ref(STORE), "sessions"));
    for row in sessions.iter() {
        if js::get_str(&row, "id") == id {
            js::set_str(&row, "project", &project);
        }
    }
    mjs::redraw();
    JsValue::UNDEFINED
}

fn pick_target(_this: JsValue, args: js_sys::Array) -> JsValue {
    let field = match js::arg_str(&args, 0).as_str() {
        "project" => "projectId",
        "worktree" => "worktreeId",
        "branch" => "branchId",
        _ => return JsValue::UNDEFINED,
    };
    let s = mjs::store_ref(STORE);
    js::set_str(&s, field, &js::arg_str(&args, 1));
    js::call(&mjs::store_ref("picker"), "close", &[]);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn pick_browser_sandbox(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_str(&s, "browserSandboxId", &js::arg_str(&args, 0));
    js::call(&mjs::store_ref("picker"), "close", &[]);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn browser_rows() -> JsValue {
    let rows = js_sys::Array::new();
    for id in ["sbox-sample-1", "sbox-sample-2"] {
        let r = js::obj();
        js::set_str(&r, "id", id);
        js::set_str(&r, "name", id);
        js::set_str(&r, "icon", "desktop");
        js::set_str(&r, "hint", "ready");
        rows.push(&r);
    }
    rows.into()
}

fn save_target_sample(_this: JsValue, args: js_sys::Array) -> JsValue {
    let action = js::arg_str(&args, 0);
    let payload = args.get(1);
    let host = mjs::store_ref(STORE);
    if action == "sandbox-remove" {
        let id = js::get_str(&payload, "id");
        let rows = js_sys::Array::new();
        for row in js_sys::Array::from(&js::get(&host, "sandboxes")).iter() {
            if js::get_str(&row, "id") != id { rows.push(&row); }
        }
        let rows_js: JsValue = rows.into();
        js::set(&host, "sandboxes", &rows_js);
        js::set(
            &host,
            "sandboxGroups",
            &crate::components::sandboxes_menu::group_rows(&rows_js),
        );
        js::set_bool(&mjs::store_ref("tform"), "open", false);
    } else if action == "automation-save" {
        let id = js::get_str(&payload, "id");
        let id = if id.trim().is_empty() { "a-sample-created" } else { &id };
        let rows = js_sys::Array::from(&js::get(&host, "automations"));
        let mut replaced = false;
        for row in rows.iter() {
            if js::get_str(&row, "id") == id {
                for key in ["name", "cron", "prompt", "agent", "model", "timeoutMin"] {
                    let v = js::get(&payload, key);
                    if !v.is_undefined() { js::set(&row, key, &v); }
                }
                let enabled = js::get(&payload, "enabled");
                if !enabled.is_undefined() { js::set_bool(&row, "enabled", enabled.as_bool().unwrap_or(true)); }
                replaced = true;
            }
        }
        if !replaced {
            let o = js::obj();
            js::set_str(&o, "id", id);
            for key in ["name", "cron", "prompt", "agent", "model"] {
                js::set_str(&o, key, &js::get_str(&payload, key));
            }
            let timeout = js::get(&payload, "timeoutMin");
            js::set(&o, "timeoutMin", &if timeout.is_undefined() { JsValue::from_f64(60.0) } else { timeout });
            js::set_bool(&o, "enabled", js::get(&payload, "enabled").as_bool().unwrap_or(true));
            rows.push(&o.into());
        }
        js::set(&host, "automations", &rows.into());
        js::set_bool(&mjs::store_ref("tform"), "open", false);
    } else if action == "automation-remove" {
        let id = js::get_str(&payload, "id");
        let rows = js_sys::Array::new();
        for row in js_sys::Array::from(&js::get(&host, "automations")).iter() {
            if js::get_str(&row, "id") != id { rows.push(&row); }
        }
        js::set(&host, "automations", &rows.into());
        js::set_bool(&mjs::store_ref("tform"), "open", false);
    } else if action == "automation-toggle" {
        let id = js::get_str(&payload, "id");
        let enabled = js::get(&payload, "enabled").as_bool().unwrap_or(true);
        for row in js_sys::Array::from(&js::get(&host, "automations")).iter() {
            if js::get_str(&row, "id") == id { js::set_bool(&row, "enabled", enabled); }
        }
    }
    JsValue::UNDEFINED
}

fn open_session(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let id = js::arg_str(&args, 0);
    if js::get_str(&s, "activeId") != id { crate::components::composer::reset_history(&s); }
    let preview = js::get_str(&s, "previewId");
    if !preview.is_empty() && preview != id && js::get_str(&s, "activeId") == preview {
        let tabs = js_sys::Array::new();
        for row in js_sys::Array::from(&js::get(&s, "tabs")).iter() {
            if js::get_str(&row, "id") != preview { tabs.push(&row); }
        }
        js::set(&s, "tabs", &tabs.into());
        js::set_str(&s, "previewId", "");
    }
    js::set_str(&s, "activeId", &id);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn history_move(_this: JsValue, args: js_sys::Array) -> JsValue {
    crate::components::composer::history_move(&mjs::store_ref(STORE), args)
}

fn sample_tree() -> JsValue {
    let nodes = js_sys::Array::new();
    for (id, parent, role, content) in [
        ("zero", "", "", "Session started"),
        ("one", "zero", "user", "Hi"),
        ("two", "one", "assistant", "How can I help?"),
        ("three", "two", "user", "What is the status?"),
        ("four", "three", "assistant", "Here is the current status."),
        ("five", "two", "user", "Show me another approach."),
        ("six", "five", "assistant", "Here is another approach."),
    ] {
        let n = js::obj();
        js::set_str(&n, "id", id); js::set_str(&n, "parentId", parent);
        js::set_str(&n, "kind", if role.is_empty() { "system" } else { "context_window" });
        js::set_str(&n, "role", role); js::set_str(&n, "content", content);
        nodes.push(&n);
    }
    crate::components::history_tree::flatten(&nodes, "four")
}

fn tree_close(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "treeOpen", false); js::set_num(&s, "treeEscapeAt", 0.0);
    mjs::redraw(); crate::components::history_tree::focus_composer(); JsValue::UNDEFINED
}

fn tree_escape(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let e = js::arg(&args, 0);
    if js::get_bool(&e, "repeat") { return JsValue::UNDEFINED; }
    if js::get_bool(&s, "treeOpen") { return tree_close(JsValue::UNDEFINED, js_sys::Array::new()); }
    let now = js_sys::Date::now();
    let previous = js::get(&s, "treeEscapeAt").as_f64().unwrap_or(0.0);
    js::set_num(&s, "treeEscapeAt", now);
    if previous > 0.0 && now - previous <= 750.0 {
        js::call(&e, "preventDefault", &[]);
        js::set(&s, "treeRows", &sample_tree());
        js::set_num(&s, "treeIndex", 4.0);
        js::set_bool(&s, "treeOpen", true);
        js::set_num(&s, "treeEscapeAt", 0.0);
        mjs::redraw();
    }
    JsValue::UNDEFINED
}

fn tree_reset_escape(_this: JsValue, _args: js_sys::Array) -> JsValue {
    js::set_num(&mjs::store_ref(STORE), "treeEscapeAt", 0.0);
    JsValue::UNDEFINED
}

fn tree_select(_this: JsValue, args: js_sys::Array) -> JsValue {
    crate::components::history_tree::select(&mjs::store_ref(STORE), js::arg_num(&args, 0, 0.0) as i64);
    JsValue::UNDEFINED
}
fn tree_move(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let i = js::get(&s, "treeIndex").as_f64().unwrap_or(0.0) as i64;
    crate::components::history_tree::select(&s, i + js::arg_num(&args, 0, 0.0) as i64);
    JsValue::UNDEFINED
}
fn tree_choose(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_str(&s, "cursor", &js::arg_str(&args, 0));
    tree_close(JsValue::UNDEFINED, js_sys::Array::new())
}
fn tree_choose_selected(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let rows = js_sys::Array::from(&js::get(&s, "treeRows"));
    let i = js::get(&s, "treeIndex").as_f64().unwrap_or(0.0) as u32;
    let args = js_sys::Array::new(); args.push(&JsValue::from_str(&js::get_str(&rows.get(i), "id")));
    tree_choose(JsValue::UNDEFINED, args)
}

fn draft_input(_this: JsValue, args: js_sys::Array) -> JsValue {
    crate::components::composer::draft_input(&mjs::store_ref(STORE), args)
}

fn send_sample(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let text = js::get_str(&s, "draft");
    if !text.trim().is_empty() {
        let rows = js_sys::Array::from(&js::get(&s, "messages"));
        let row = js::obj();
        js::set_str(&row, "kind", "user");
        js::set_str(&row, "text", text.trim());
        js::set_str(&row, "id", &format!("sample-user-{}", rows.length()));
        rows.push(&row);
        js::set(&s, "messages", &rows.into());
        crate::components::composer::reset_history(&s);
        js::set_str(&s, "draft", "");
        mjs::redraw();
    }
    JsValue::UNDEFINED
}

fn preview_session(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let id = js::arg_str(&args, 0);
    let tabs = js_sys::Array::from(&js::get(&s, "tabs"));
    if !tabs.iter().any(|row| js::get_str(&row, "id") == id) {
        let preview = js::get_str(&s, "previewId");
        let next = js_sys::Array::new();
        let session = js_sys::Array::from(&js::get(&s, "sessions"))
            .iter().find(|row| js::get_str(row, "id") == id);
        if let Some(session) = session {
            let mut replaced = false;
            for row in tabs.iter() {
                if !preview.is_empty() && js::get_str(&row, "id") == preview {
                    next.push(&session);
                    replaced = true;
                } else { next.push(&row); }
            }
            if !replaced { next.push(&session); }
            js::set(&s, "tabs", &next.into());
            js::set_str(&s, "previewId", &id);
        }
    }
    open_session(JsValue::UNDEFINED, args)
}

fn pin_tab(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    if js::get_str(&s, "previewId") == js::arg_str(&args, 0) {
        js::set_str(&s, "previewId", "");
        mjs::redraw();
    }
    JsValue::UNDEFINED
}

fn close_tab(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let id = js::arg_str(&args, 0);
    let tabs = js_sys::Array::new();
    for row in js_sys::Array::from(&js::get(&s, "tabs")).iter() {
        if js::get_str(&row, "id") != id { tabs.push(&row); }
    }
    if js::get_str(&s, "previewId") == id { js::set_str(&s, "previewId", ""); }
    if js::get_str(&s, "activeId") == id {
        js::set_str(&s, "activeId", &js::get_str(&tabs.get(0), "id"));
    }
    js::set(&s, "tabs", &tabs.into());
    mjs::redraw();
    JsValue::UNDEFINED
}

fn row_of(_this: JsValue, args: js_sys::Array) -> JsValue {
    let want = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    let rows = js_sys::Array::from(&js::get(&s, "messages"));
    for i in 0..rows.length() {
        if js::get_str(&rows.get(i), "id") == want {
            return rows.get(i);
        }
    }
    for i in 0..rows.length() {
        if js::get_str(&rows.get(i), "kind") == want {
            return rows.get(i);
        }
    }
    js::obj().into()
}

fn decide_approval(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    let verdict = js::arg_str(&args, 1);
    let rows = js_sys::Array::from(&js::get(&mjs::store_ref(STORE), "messages"));
    for i in 0..rows.length() {
        let r = rows.get(i);
        if js::get_str(&r, "id") == id {
            js::set_str(&r, "status", &verdict);
            js::set_bool(&r, "expanded", false);
        }
    }
    mjs::redraw();
    JsValue::UNDEFINED
}

fn toggle_card(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    let rows = js_sys::Array::from(&js::get(&s, "messages"));
    for i in 0..rows.length() {
        let r = rows.get(i);
        if js::get_str(&r, "id") == id {
            let open = js::get_bool(&r, "expanded");
            js::set_bool(&r, "expanded", !open);
        }
    }
    mjs::redraw();
    JsValue::UNDEFINED
}

fn accent_of(_this: JsValue, args: js_sys::Array) -> JsValue {
    let found = js::get_str(
        &js::get(&mjs::store_ref(STORE), "accents"),
        &js::arg_str(&args, 0),
    );
    JsValue::from_str(if found.is_empty() {
        crate::components::agents::DEFAULT_ACCENT
    } else {
        &found
    })
}

fn icon_of(_this: JsValue, args: js_sys::Array) -> JsValue {
    let found = js::get_str(
        &js::get(&mjs::store_ref(STORE), "icons"),
        &js::arg_str(&args, 0),
    );
    JsValue::from_str(if found.is_empty() {
        crate::components::agents::DEFAULT_ICON
    } else {
        &found
    })
}

fn toggle_notifications(this: JsValue, _args: js_sys::Array) -> JsValue {
    js::set_bool(&this, "notifyEnabled", !js::get_bool(&this, "notifyEnabled"));
    JsValue::UNDEFINED
}

fn mark_session_read(this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    let old = js_sys::Array::from(&js::get(&this, "unreadSessions"));
    let next = js_sys::Array::new();
    for row in old.iter() { if row.as_string().as_deref() != Some(&id) { next.push(&row); } }
    if old.length() != next.length() { js::set(&this, "unreadSessions", &next.into()); }
    JsValue::UNDEFINED
}

fn inert(_this: JsValue, _args: js_sys::Array) -> JsValue {
    JsValue::UNDEFINED
}

fn refresh_stream(this: JsValue, args: js_sys::Array) -> JsValue {
    let answer = crate::helpers::markdown_to_html(&js::get_str(&this, "streamText"));
    let reasoning = crate::helpers::markdown_to_html(&js::get_str(&this, "streamReasoning"));
    crate::components::session::refresh_stream_nodes(&js::arg(&args, 0), &answer, &reasoning);
    JsValue::UNDEFINED
}

pub fn register() {
    let accents = js::obj();
    js::set_str(&accents, "fish", "#38bdf8");
    js::set_str(&accents, "joker", "#be5cff");
    let icons = js::obj();
    js::set_str(&icons, "fish", "fish");
    js::set_str(&icons, "joker", "mask-happy");

    let others = js_sys::Array::new();
    for (i, (handle, icon, accent)) in
        [("fish", "fish", "#38bdf8"), ("joker", "mask-happy", "#be5cff")]
            .iter()
            .enumerate()
    {
        let seat = js::obj();
        js::set_num(&seat, "id", (i + 1) as f64);
        js::set_str(&seat, "handle", handle);
        js::set_str(&seat, "icon", icon);
        js::set_str(&seat, "accentColor", accent);
        js::set_bool(&seat, "active", true);
        others.push(&seat);
    }

    let metrics = crate::helpers::metrics_html(&crate::helpers::SessionMetrics {
        tok_per_sec: 42.0,
        duration_ms: 3100.0,
        input_tokens: 1761.0,
        output_tokens: 214.0,
        bytes: 9400.0,
    });

    let session = js::obj();
    js::set_num(&session, "contextWindow", 200000.0);
    js::set_str(&session, "id", "s-1");
    js::set_str(&session, "title", "darkmarch run");
    js::set_str(&session, "purpose", "two features, two worktrees, no stepping on toes");
    js::set_str(&session, "notes", "the guide's sample; inert");
    js::set_str(&session, "routing", "everyone");
    js::set_num(&session, "memoryHigh", 67108864.0);
    js::set_str(&session, "workspaceRoot", "/home/Bob/src/vibe-arcade");
    let worktree = js::obj();
    js::set_str(&worktree, "repoId", "r-sample");
    js::set_str(&worktree, "repoName", "vibe-arcade");
    js::set_str(&worktree, "branch", "chat/1a2b3c4d");
    js::set_str(&worktree, "path", "/home/Bob/.angela/worktrees/vibe-arcade/chat-1a2b3c4d-0b82d9328e39ec99");
    js::set_str(&worktree, "branchPattern", "chat/{{hex}}");
    js::set_str(&worktree, "pathPattern", "{{parent}}/{{repo}}/{{prefix}}-{{hash}}");
    js::set_bool(&worktree, "provisioned", true);
    js::set(&session, "worktree", &worktree);
    let sample_sessions = sessions();
    let initial_tabs = js_sys::Array::new();
    let all = js_sys::Array::from(&sample_sessions);
    initial_tabs.push(&all.get(0));
    initial_tabs.push(&all.get(1));

    let mut d = Def::new().named("store samples");
    d
        .val("messages", &messages())
        .val("queue", &queue_sample())
        .val("threads", &js_sys::Array::new().into())
        .val("approvals", &js_sys::Array::new().into())
        .val("session", &session.into())
        .val("accents", &accents.into())
        .val("icons", &icons.into())
        .val("sandboxes", &sandbox_rows())
        .val("browserSandboxes", &browser_rows())
        .str("browserSandboxId", "sbox-sample-1")
        .str("browserViewerUrl", "")
        .bool("viewerFetching", false)
        .val("sandboxGroups", &crate::components::sandboxes_menu::group_rows(&sandbox_rows()))
        .val("sandboxOpen", &js::obj().into())
        .str("sboxConfirmId", "")
        .str("sboxError", "")
        .val("automations", &sample_automations())
        .str("activeProject", "")
        .val("projects", &targets(PROJECTS))
        .val("worktrees", &targets(WORKTREES))
        .val("repos", &targets(REPOS))
        .bool("sboxUp", true)
        .val("repoBranches", &targets(BRANCHES))
        .str("projectId", "psicom/chat")
        .str("worktreeId", "")
        .str("branchId", "br-chat-main")
        .str("activeId", "samples")
        .bool("notifyEnabled", false)
        .val("unreadSessions", &js_sys::Array::new().into())
        .str("model", "muse:opus")
        .val("models", &models())
        .val("sessions", &sample_sessions)
        .val("tabs", &initial_tabs.into())
        .str("previewId", "s-2")
        .bool("compacted", false)
        .bool("treeOpen", false)
        .bool("treeLoading", false)
        .num("treeEscapeAt", 0.0)
        .num("treeIndex", 4.0)
        .val("treeRows", &sample_tree())
        .str("cursor", "four")
        .val("others", &others.clone().into())
        .val("activeOthers", &others.into())
        .str("userName", "Bob")
        .num("agentCount", 2.0)
        .bool("isGroup", true)
        .bool("compacting", false)
        .str("draft", "")
        .num("historyPosition", -1.0)
        .str("historyDraft", "")
        .bool("waiting", false)
        .bool("stopping", false)
        .bool("stopUnconfirmed", false)
        .str("allowlist", "")
        .bool("allowlistOpen", false)
        .bool("allowlistOverridden", false)
        .bool("unrestricted", false)
        .str("lastMetrics", &metrics)
        .num("promptTokens", 1761.0)
        .num("totalTokens", 1975.0)
        .str("streamText", "")
        .str("streamReasoning", "")
        .str("streamHandle", "")
        .bool("streamReasoningOpen", false)
        .val("prSample", &pr_sample())
        .val("repoSample", &repo_sample())
        .val("jobSample", &job_sample())
        .val("diffFiles", &crate::components::diff_view::parse_json(SAMPLE_DIFF))
        .val("journal", &journal_sample())
        .val("runSample", &run_sample())
        .method("sample", sample_of)
        .method("row", row_of)
        .method("decideApproval", decide_approval)
        .method("openSession", open_session)
        .method("toggleNotifications", toggle_notifications)
        .method("markSessionRead", mark_session_read)
        .method("previewSession", preview_session)
        .method("pinTab", pin_tab)
        .method("deleteSession", inert)
        .method("closeTab", close_tab)
        .method("newSession", inert)
        .method("toggleCard", toggle_card)
        .method("accentOf", accent_of)
        .method("iconOf", icon_of)
        .method("statusParts", crate::components::system_card::status_parts)
        .method("turnEdges", crate::components::session::turn_edges)
        .method("streamEdges", crate::components::session::stream_edges)
        .method("refreshStream", refresh_stream)
        .method("relAgo", crate::components::live::rel_ago)
        .method("keepPinned", inert)
        .method("onScroll", inert)
        .method("forkPrompt", inert)
        .method("send", send_sample)
        .method("draftInput", draft_input)
        .method("historyMove", history_move)
        .method("treeEscape", tree_escape)
        .method("treeResetEscape", tree_reset_escape)
        .method("treeClose", tree_close)
        .method("treeSelect", tree_select)
        .method("treeMove", tree_move)
        .method("treeChoose", tree_choose)
        .method("treeChooseSelected", tree_choose_selected)
        .method("stop", inert)
        .method("compact", inert)
        .method("toggleThread", inert)
        .method("removeAgent", inert)
        .method("saveAllowlist", inert)
        .method("revertAllowlist", inert)
        .method("setModel", inert)
        .method("setPolicy", inert)
        .method("setReasoning", inert)
        .method("openGroupEditor", crate::components::group_editor::open)
        .method("openTreeFolder", crate::components::group_editor::open_folder)
        .method("dropSession", drop_session)
        .method("refreshModels", inert)
        .method("loadProjects", inert)
        .method("loadTargets", inert)
        .method("saveTarget", save_target_sample)
        .method("saveProject", inert)
        .method("renameTreeRow", rename_tree_row)
        .method("pickTarget", pick_target)
        .method("dequeue", dequeue_sample)
        .method("pickBrowserSandbox", pick_browser_sandbox)
        .method("loadViewerUrl", inert)
        .method("editTarget", edit_target)
        .method("createTarget", create_target)
        .method("persistComposerSettings", inert)
        .method("noop", inert)
        .method("toggleSandboxGroup", crate::components::sandboxes_menu::toggle_group)
        .method("sboxRemove", sbox_remove_sample);
    mjs::store(STORE, &d.finish());
}

fn sbox_remove_sample(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    if js::get_str(&s, "sboxConfirmId") != id {
        crate::components::sandboxes_menu::arm_row(STORE, &id);
        return JsValue::UNDEFINED;
    }
    js::set_str(&s, "sboxConfirmId", "");
    js::set_str(&s, "sboxError", "");
    let rows = js_sys::Array::new();
    for row in js_sys::Array::from(&js::get(&s, "sandboxes")).iter() {
        if js::get_str(&row, "id") != id { rows.push(&row); }
    }
    let rows_js: JsValue = rows.into();
    js::set(&s, "sandboxes", &rows_js);
    js::set(
        &s,
        "sandboxGroups",
        &crate::components::sandboxes_menu::group_rows(&rows_js),
    );
    mjs::redraw();
    JsValue::UNDEFINED
}

fn rename_tree_row(_this: JsValue, args: js_sys::Array) -> JsValue {
    let kind = js::arg_str(&args, 0);
    let old = js::arg_str(&args, 1);
    let id = js::arg_str(&args, 2);
    let name = js::arg_str(&args, 3).trim().to_string();
    if name.is_empty() { return JsValue::UNDEFINED; }
    let s = mjs::store_ref(STORE);
    if kind == "session" {
        for row in js_sys::Array::from(&js::get(&s, "sessions")).iter() {
            if js::get_str(&row, "id") == id { js::set_str(&row, "title", &name); }
        }
    } else if kind == "project" && !name.contains('/') && name != "." && name != ".." {
        let new = match old.rsplit_once('/') {
            Some((parent, _)) => format!("{parent}/{name}"),
            None => name,
        };
        for (list, field) in [("projects", "name"), ("sessions", "project")] {
            for row in js_sys::Array::from(&js::get(&s, list)).iter() {
                let value = js::get_str(&row, field);
                if value == old || value.starts_with(&format!("{old}/")) {
                    js::set_str(&row, field, &format!("{new}{}", &value[old.len()..]));
                }
            }
        }
    }
    mjs::redraw();
    JsValue::UNDEFINED
}

pub fn sample(kind: &str) -> String {
    format!("{EXPR}.sample('{kind}')")
}

fn sample_of(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    match js::arg_str(&args, 0).as_str() {
        "pr" => js::get(&s, "prSample"),
        "repo" => js::get(&s, "repoSample"),
        "job" => js::get(&s, "jobSample"),
        _ => js::obj().into(),
    }
}

const SAMPLE_DIFF: &str = "diff --git a/darkmarch.html b/darkmarch.html
index 111612e..87b90b7 100644
--- a/darkmarch.html
+++ b/darkmarch.html
@@ -18,6 +18,8 @@
 #wave-label{margin-top:8px;font-size:15px;letter-spacing:3px}
+#ember-label{font-size:12px;letter-spacing:1.5px;margin-top:5px}
+#ember-label.hidden{display:none}
 #gold-label{position:absolute;top:16px;right:20px}
diff --git a/README.md b/README.md
index 4dd30d1..9c22ee4 100644
--- a/README.md
+++ b/README.md
@@ -1,3 +1,7 @@
 # darkmarch
+
+## Ember rings
+
+Rare elite drops stack a damage multiplier.
 The tomb fights back.";
 
fn pr_sample() -> JsValue {
    let description =
        "Adds the **ember ring** pickup: rare elite drops that stack a `+15%` damage multiplier.\n\n- HUD chip shows the stack count\n- demo video attached";
    let pr = js::obj();
    js::set_num(&pr, "id", 1.0);
    js::set_num(&pr, "number", 7.0);
    js::set_str(&pr, "repo", "vibe-arcade");
    js::set_str(&pr, "sourceBranch", "chat/ember-f701ab9a");
    js::set_str(&pr, "targetBranch", "main");
    js::set_str(&pr, "title", "Add ember ring pickups");
    js::set_str(&pr, "description", description);
    js::set_str(&pr, "descriptionHtml", &crate::helpers::markdown_to_html(description));
    js::set_str(&pr, "author", "Bob");
    js::set_str(&pr, "state", "approved");
    js::set_str(&pr, "createdAt", "2026-11-27T22:04:00.000Z");
    js::set_str(&pr, "updatedAt", "2026-11-27T22:31:00.000Z");
    js::set(&pr, "mergeCommit", &JsValue::NULL);
    js::set(&pr, "mergedBy", &JsValue::NULL);
    let reviews = js_sys::Array::new();
    let r1 = js::obj();
    js::set_num(&r1, "id", 11.0);
    js::set_str(&r1, "reviewer", "Alice");
    js::set_str(&r1, "decision", "approve");
    js::set_str(&r1, "comment", "Nice — the HUD chip reads well at 720p.");
    js::set_str(&r1, "createdAt", "2026-11-27T22:12:00.000Z");
    reviews.push(&r1);
    let r2 = js::obj();
    js::set_num(&r2, "id", 12.0);
    js::set_str(&r2, "reviewer", "ajcs-bot");
    js::set_str(&r2, "decision", "comment");
    js::set_str(&r2, "comment", "");
    js::set_str(&r2, "createdAt", "2026-11-27T22:13:00.000Z");
    reviews.push(&r2);
    js::set(&pr, "reviews", &reviews);
    let attachments = js_sys::Array::new();
    let at = js::obj();
    js::set_num(&at, "id", 3.0);
    js::set_str(&at, "name", "demo-ember-ring.mp4");
    js::set_str(&at, "mime", "video/mp4");
    js::set_num(&at, "size", 9029974.0);
    attachments.push(&at);
    js::set(&pr, "attachments", &attachments);
    pr.into()
}

fn repo_sample() -> JsValue {
    let repo = js::obj();
    js::set_str(&repo, "id", "r-sample");
    js::set_str(&repo, "name", "vibe-arcade");
    js::set_str(&repo, "path", "/home/Bob/src/vibe-arcade");
    js::set_str(&repo, "description", "the arcade of small games");
    js::set_str(&repo, "defaultBranch", "main");
    js::set_str(&repo, "createdAt", "2026-11-20T00:00:00.000Z");
    repo.into()
}

fn job_sample() -> JsValue {
    let job = js::obj();
    js::set_str(&job, "id", "1");
    js::set_str(&job, "name", "verify");
    js::set_str(&job, "basedir", "vibe-arcade");
    js::set_str(&job, "display", "vibe-arcade:verify");
    js::set_str(&job, "description", "syntax-check the inline game scripts");
    js::set_str(&job, "cron", "0 9 * * 1-5");
    js::set_bool(&job, "enabled", true);
    js::set_str(&job, "sourcePath", "/home/Bob/src/vibe-arcade/.psicom/Jobfile.yaml");
    js::set_str(&job, "sourceSha", "9c22ee44ab");
    job.into()
}

fn journal_sample() -> JsValue {
    let rows = js_sys::Array::new();
    for (i, (kind, stage, message)) in [
        ("stage_start", "build", "stage 'build' started"),
        ("step_start", "build", "shell step 1"),
        ("note", "build", "$ node --check game.js\nok"),
        ("step_end", "build", "shell step 1 succeeded"),
        ("stage_end", "build", "stage 'build' success"),
        ("qa", "test", "which browser should the smoke test drive?"),
        ("qa", "test", "chromium — it is what the arcade serves", ),
        ("artifact", "open", "PR opened: vibe-arcade#7"),
        ("error", "lint", "eslint: 2 warnings"),
    ]
    .iter()
    .enumerate()
    {
        let e = js::obj();
        js::set_num(&e, "id", (i + 1) as f64);
        js::set_str(&e, "kind", kind);
        js::set_str(&e, "stage", stage);
        js::set_str(&e, "level", if *kind == "error" { "error" } else { "info" });
        js::set_str(&e, "message", message);
        js::set_str(&e, "createdAt", "2026-11-27T22:31:00.000Z");
        rows.push(&e);
    }
    rows.into()
}

fn run_sample() -> JsValue {
    let run = js::obj();
    js::set_str(&run, "id", "2");
    js::set_str(&run, "jobName", "verify");
    js::set_num(&run, "number", 3.0);
    js::set_str(&run, "label", "vibe-arcade:verify#3");
    js::set_str(&run, "state", "running");
    js::set_str(&run, "triggerSource", "trigger");
    let stages = js_sys::Array::new();
    for (i, (name, state, group, sandbox, session)) in [
        ("build", "success", "", "sbox-0a1b2c3d", ""),
        ("test", "running", "", "", "2026-11-27T22-31-00_ab12cd"),
        ("lint", "pending", "fork-a", "", ""),
        ("smoke", "blocked", "fork-a", "", ""),
    ]
    .iter()
    .enumerate()
    {
        let st = js::obj();
        js::set_str(&st, "name", name);
        js::set_str(&st, "state", state);
        js::set_str(&st, "group", group);
        if sandbox.is_empty() {
            js::set(&st, "sandboxId", &JsValue::NULL);
        } else {
            js::set_str(&st, "sandboxId", sandbox);
        }
        if session.is_empty() {
            js::set(&st, "sessionId", &JsValue::NULL);
        } else {
            js::set_str(&st, "sessionId", session);
        }
        js::set_num(&st, "position", i as f64);
        let artifacts = js_sys::Array::new();
        if i == 0 {
            let a = js::obj();
            js::set_str(&a, "repo", "vibe-arcade");
            js::set_num(&a, "number", 7.0);
            artifacts.push(&a);
        }
        js::set(&st, "artifacts", &artifacts);
        stages.push(&st);
    }
    js::set(&run, "stages", &stages);
    run.into()
}

//! `samples` — the design guide's conversation, as a store.
//!
//! The guide does not get its own markup. Every route under `/storybook`
//! mounts the **same** builders the app mounts, pointed at this store instead
//! of `live`. So a component has exactly one definition, and the guide is
//! what that definition looks like with data in it.
//!
//! That is the whole reason this file exists. The alternative — a `card()`
//! per component taking static text, beside a second rendering of the same
//! card bound to the store — is two definitions of one thing, and the one
//! nobody is looking at goes stale. It had: the guide was still drawing flat
//! bubbles and `.tool-card` chrome long after the app had grown turn rails,
//! dots, agent colours and fold cards.
//!
//! The store's shape is the live store's shape, so [`crate::components::session_panel::panel`]
//! cannot tell the difference. Its **content** is built by the app's own
//! renderers (`markdown_to_html`, `metrics_html`) rather than hand-written
//! HTML, for the same reason: a sample that spells out its own markup is one
//! more copy to keep in step.

use wasm_bindgen::JsValue;

use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

/// The store's name, and the expression templates read it through.
pub const STORE: &str = "samples";
pub const EXPR: &str = "$store.samples";

/// The expression naming the sample row of one kind — what a single card's
/// demo is built over. The row carries that `kind`, so the card's own
/// `x-if` is exercised by the demo rather than bypassed by it.
pub fn row(kind: &str) -> String {
    format!("{EXPR}.row('{kind}')")
}

/// A transcript row, in the shape the cards read.
struct Row {
    id: &'static str,
    kind: &'static str,
    text: &'static str,
    /// Blank for the rows that are nobody's: the user's own, the app's.
    handle: &'static str,
    /// Whether the row belongs to an inference — what draws the rail and dot.
    turn: bool,
    first: bool,
    last: bool,
    /// Fold cards start shut in the app. The guide opens two of them so both
    /// states are on the page at once.
    open: bool,
    /// Tool rows only.
    name: &'static str,
    /// Queued rows only: the model being waited on, and the place in line.
    spec: &'static str,
    position: u32,
    /// User rows only: who said it (PLAN44 A6) — blank is "you".
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
    spec: "",
    position: 0,
    from: "",
};

/// One turn, start to finish: the question, the paperwork, the thinking, a
/// tool call and its result, the answer. Then the two rows that belong to
/// nobody — an error and the room noting who arrived. Every kind the
/// transcript can draw, in the order it draws them.
fn rows() -> Vec<Row> {
    vec![
        Row {
            id: "u1",
            kind: "user",
            text: "Read https://example.com/config and tell me which model it points at.",
            ..BLANK
        },
        // The same kind, another voice: a shared room's other human wears
        // the `from-other` variant (left, teal) rather than the me-bubble.
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
            id: "q1",
            kind: "queued",
            handle: "fish",
            turn: true,
            text: "waiting for llama-server:default slot…",
            spec: "llama-server:default",
            position: 3,
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
        // The byline's time runs through the same formatter the app uses, so
        // the guide reads "just now" the way a live transcript would.
        js::set_str(&o, "ts", &now);
        js::set_bool(&o, "turn", r.turn);
        js::set_bool(&o, "turnFirst", r.first);
        js::set_bool(&o, "turnLast", r.last);
        js::set_bool(&o, "expanded", r.open);
        js::set_bool(&o, "ok", true);
        js::set_str(&o, "spec", r.spec);
        js::set_num(&o, "position", r.position as f64);
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
            // `command` is what the allowlist was matched against — the
            // reason the card can answer "why was this not permitted?".
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
        // Message bodies use the same renderer as the live store.
        if matches!(r.kind, "user" | "assistant" | "reasoning") {
            js::set_str(&o, "html", &crate::helpers::markdown_to_html(r.text));
        }
        out.push(&o);
    }
    out.into()
}

/// The sessions the guide's project tree and tab strip show.
///
/// `(ordinal, id, title, status)` — the same fields the live list carries,
/// because the tree and the strip are the live ones.
/// The last field is the folder it is filed under — `""` for unfiled, which
/// the tree draws in its own group at the bottom. One of each, so the guide
/// exercises a nested folder, a sibling, and the unfiled case rather than
/// just the happy one.
/// The guide's conversations.
///
/// `turns` and the age are here because the project tree renders both: a
/// sample missing a field the tree reads is a guide that shows less than the
/// app does, which is how the two versions of one component drift. The same
/// goes for `contextWindow` and `totalTokens` — the pair the state symbol's
/// ring (and the composer's context pie) fills by. The open
/// conversation's numbers match the store's own `totalTokens` /
/// `session.contextWindow`, so the guide's pie and its rings agree.
const SESSIONS: &[(f64, &str, &str, &str, &str, f64, f64, f64, f64)] = &[
    // ordinal, id, title, status, project, turns, minutes since it started,
    // context window, tokens used
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

/// The models the guide's picker offers.
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
        // Started `minutes` ago, so the tree's age reads the way a real one
        // does rather than every row saying "just now".
        let created = js_sys::Date::new_0();
        created.set_time(now.get_time() - minutes * 60_000.0);
        js::set_str(&o, "createdAt", &iso(&created));
        js::set_str(&o, "updatedAt", &iso(&created));
        js::set_str(&o, "lastEventAt", &iso(&created));
        out.push(&o);
    }
    out.into()
}

/// The guide's instances for the three composer pickers: `{ id, name, icon,
/// hint }`, which is the shape [`crate::components::picker`] reads.
///
/// Enough of each to exercise the menu rather than illustrate it — more rows
/// than fit without scrolling in one case, a hint on every branch, and a
/// selection already made in three of the four so the check has somewhere to
/// land.
fn targets(rows: &[(&str, &str, &str, &str)]) -> JsValue {
    let out = js_sys::Array::new();
    for (id, name, icon, hint) in rows {
        let o = js::obj();
        js::set_str(&o, "id", id);
        js::set_str(&o, "name", name);
        js::set_str(&o, "icon", icon);
        js::set_str(&o, "hint", hint);
        out.push(&o);
    }
    out.into()
}

/// The guide's sandboxes for the menu (PLAN55 rows): `{ id, image, state,
/// session, agent }` — the shape the listing reads. Two live `sbox-video`
/// containers so the per-image rollup has something to count, and one
/// unassigned stopped box so the "unassigned" label and a zero count show.
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

/// (id, name, cron, prompt, agent, model, timeoutMin, enabled)
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

/// The id **is** the name, as `/api/projects` returns it: a project has no
/// identity apart from its label, and the hierarchy is the `/` in it.
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

/// The hint carries the repo, because the name is only a branch — two repos
/// both having `main` is the normal case, not the edge one.
const BRANCHES: &[(&str, &str, &str, &str)] = &[
    ("br-chat-main", "main", "git-branch", "psicom/chat"),
    ("br-chat-sbox", "sbox-a", "git-branch", "psicom/chat"),
    ("br-angela-main", "main", "git-branch", "angela-rs"),
    ("br-angela-rust", "rust", "git-branch", "angela-rs"),
    ("br-agl-rust", "rust", "git-branch", "agl-rs"),
];

/// The guide's pickers open the same inspector form the app's do, pointed
/// back at this store — so the form saves nothing, because `saveProject`
/// here is inert.
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

/// `dropSession(id, project)` — the guide's drag-and-drop really moves the
/// row. Inert here would be worse than absent: a drag that visibly snaps
/// back reads as a broken component rather than as a disabled one.
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

/// `pickTarget(kind, id)` — the guide's pickers really do select, because a
/// menu whose check never moves teaches nothing about the check.
fn pick_target(_this: JsValue, args: js_sys::Array) -> JsValue {
    let field = match js::arg_str(&args, 0).as_str() {
        "project" => "projectId",
        "worktree" => "worktreeId",
        "branch" => "branchId",
        _ => return JsValue::UNDEFINED,
    };
    let s = mjs::store_ref(STORE);
    js::set_str(&s, field, &js::arg_str(&args, 1));
    // Picking is also a dismissal: the menu has done its job.
    js::call(&mjs::store_ref("picker"), "close", &[]);
    mjs::redraw();
    JsValue::UNDEFINED
}

/// The automation form behaves like the app without reaching the real
/// service: writes land in the samples store so the guide's rail redraws.
fn save_target_sample(_this: JsValue, args: js_sys::Array) -> JsValue {
    let action = js::arg_str(&args, 0);
    let payload = args.get(1);
    let host = mjs::store_ref(STORE);
    if action == "sandbox-remove" {
        // The guide's Destroy behaves like the app: the row goes, the
        // groups recompute, the form closes.
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
        // Mirror the server: an id replaces, no id mints one.
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

/// `openSession(id)` — the guide's rail and strip really do switch tabs.
/// Interactive where the app is interactive, inert where the app would reach
/// the network: a guide that cannot be clicked teaches half the component.
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

/// `row(what)` — the sample row with that **id**, or failing that the first
/// of that **kind**. Most cards want "the approval row"; a card with two
/// states to show wants one specific one.
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

/// `decideApproval(id, verdict)` — the guide's card answers for real, so the
/// three buttons and the resolved state can both be seen without a session.
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

/// `toggleCard(id)` — the guide's fold cards open and shut like the app's,
/// because the component under test is the app's, click handler included.
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

/// The accent and glyph a byline is drawn with, read from this store's own
/// roster maps — the same lookup the live store does.
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

/// The bell flips, because being clickable is the point of showing it — but
/// it writes to the guide's own store, so nothing is persisted and no real
/// notification is armed.
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

/// Everything the guide has no business doing: no turn to send, no request to
/// stop, nothing to persist. A no-op rather than an omission, because a
/// missing method is a template error, not a quiet nothing — and a guide that
/// throws teaches nothing.
fn inert(_this: JsValue, _args: js_sys::Array) -> JsValue {
    JsValue::UNDEFINED
}

pub fn register() {
    let accents = js::obj();
    js::set_str(&accents, "fish", "#38bdf8");
    js::set_str(&accents, "joker", "#be5cff");
    let icons = js::obj();
    js::set_str(&icons, "fish", "fish");
    js::set_str(&icons, "joker", "mask-happy");

    // The room `session::messages` renders against: you, the agent answering,
    // and the one who just joined (the transcript ends on "joker joined the
    // conversation"). Same shape `store_roster` writes, so the roster template
    // reads the guide exactly the way it reads the app.
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
    let sample_sessions = sessions();
    let initial_tabs = js_sys::Array::new();
    let all = js_sys::Array::from(&sample_sessions);
    initial_tabs.push(&all.get(0));
    initial_tabs.push(&all.get(1));

    let mut d = Def::new().named("store samples");
    d
        // ── the conversation ───────────────────────────────────────
        .val("messages", &messages())
        .val("threads", &js_sys::Array::new().into())
        // Nothing is waiting in a still picture; the guide's approvals are
        // rows in the transcript, in both of their states.
        .val("approvals", &js_sys::Array::new().into())
        .val("session", &session.into())
        .val("accents", &accents.into())
        .val("icons", &icons.into())
        // ── the composer's target row ─────────────────────────────
        .val("sandboxes", &sandbox_rows())
        .val("sandboxGroups", &crate::components::sandboxes_menu::group_rows(&sandbox_rows()))
        .val("sandboxOpen", &js::obj().into())
        .val("automations", &sample_automations())
        .val("projects", &targets(PROJECTS))
        .val("worktrees", &targets(WORKTREES))
        .val("repos", &targets(REPOS))
        .bool("sboxUp", true)
        .val("repoBranches", &targets(BRANCHES))
        .str("projectId", "psicom/chat")
        // Nothing chosen, so the guide shows the null row checked too.
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
        // ── the room ───────────────────────────────────────────────
        .val("others", &others.clone().into())
        .val("activeOthers", &others.into())
        .str("userName", "Bob")
        .num("agentCount", 2.0)
        .bool("isGroup", true)
        .bool("compacting", false)
        // ── the composer ───────────────────────────────────────────
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
        // ── nothing is streaming in a still picture ────────────────
        .str("streamText", "")
        .str("streamHtml", "")
        .str("streamReasoning", "")
        .str("streamReasoningHtml", "")
        .str("streamHandle", "")
        .bool("streamReasoningOpen", false)
        // ── methods ────────────────────────────────────────────────
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
        // The same clock *and* the same rendering the app uses: the guide's
        // project tree is the same component, so its ages must read `12m`,
        // not `12min ago`. `relAgo` is the prose form, for bylines.
        .method("toggleCard", toggle_card)
        .method("accentOf", accent_of)
        .method("iconOf", icon_of)
        .method("statusParts", crate::components::system_card::status_parts)
        .method("turnEdges", crate::components::session::turn_edges)
        .method("streamEdges", crate::components::session::stream_edges)
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
        // The pencil beside the cluster. Inert like the rest: the editor it
        // will open is not mounted yet, and a missing method is a template
        // error, not a quiet nothing.
        .method("openSessionEditor", inert)
        .method("saveAllowlist", inert)
        .method("revertAllowlist", inert)
        .method("setModel", inert)
        .method("setPolicy", inert)
        .method("setReasoning", inert)
        .method("dropSession", drop_session)
        .method("refreshModels", inert)
        .method("loadProjects", inert)
        .method("loadTargets", inert)
        // The guide's form opens and validates; it persists nothing.
        .method("saveTarget", save_target_sample)
        .method("saveProject", inert)
        .method("renameTreeRow", rename_tree_row)
        .method("pickTarget", pick_target)
        .method("editTarget", edit_target)
        .method("createTarget", create_target)
        .method("persistComposerSettings", inert)
        // The guide's rail heads have buttons; the guide creates nothing.
        .method("noop", inert)
        .method("toggleSandboxGroup", crate::components::sandboxes_menu::toggle_group);
    mjs::store(STORE, &d.finish());
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

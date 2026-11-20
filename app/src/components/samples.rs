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
            text: "Read the config and tell me which model it points at.",
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
            id: "ap1",
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
                   next turn will use.",
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
            text: "joker joined the conversation.",
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
        // Assistant and reasoning bodies are pre-rendered in the live store
        // too — the cards read `html || text`.
        if matches!(r.kind, "assistant" | "reasoning") {
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
/// app does, which is how the two versions of one component drift.
const SESSIONS: &[(f64, &str, &str, &str, &str, f64, f64)] = &[
    // ordinal, id, title, status, project, turns, minutes since it started
    (
        1.0,
        "samples",
        "Which model does the config point at?",
        "ready",
        "psicom/chat",
        6.0,
        12.0,
    ),
    (2.0, "s-2", "Migrate the auth service", "running", "psicom/chat", 3.0, 4.0),
    (3.0, "s-3", "Retry logic refactor", "ready", "psicom/sbox", 9.0, 45.0),
    (4.0, "s-4", "Scratch: regex for the log parser", "ready", "", 0.0, 130.0),
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
    for (ordinal, id, title, status, project, turns, minutes) in SESSIONS {
        let o = js::obj();
        js::set_num(&o, "ordinal", *ordinal);
        js::set_str(&o, "id", id);
        js::set_str(&o, "title", title);
        js::set_str(&o, "status", status);
        js::set_str(&o, "project", project);
        js::set_str(&o, "agent", "fish");
        js::set_str(&o, "model", "muse:opus");
        js::set_num(&o, "turns", *turns);
        // Started `minutes` ago, so the tree's age reads the way a real one
        // does rather than every row saying "just now".
        let created = js_sys::Date::new_0();
        created.set_time(now.get_time() - minutes * 60_000.0);
        js::set_str(&o, "createdAt", &iso(&created));
        js::set_str(&o, "updatedAt", &iso(&now));
        out.push(&o);
    }
    out.into()
}

/// The guide's instances for the four composer pickers: `{ id, name, icon,
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

const SANDBOXES: &[(&str, &str, &str, &str)] = &[
    ("sb-kiets", "Kiets-Spaceship", "desktop", "ready"),
    ("sb-atlas", "atlas-ci", "desktop", "ready"),
    ("sb-borealis", "borealis-replay", "desktop", "booting"),
];

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
        "sandbox" => "sandboxId",
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

/// `openSession(id)` — the guide's rail and strip really do switch tabs.
/// Interactive where the app is interactive, inert where the app would reach
/// the network: a guide that cannot be clicked teaches half the component.
fn open_session(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_str(&s, "activeId", &js::arg_str(&args, 0));
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
    let icons = js::obj();
    js::set_str(&icons, "fish", "fish");

    let metrics = crate::helpers::metrics_html(&crate::helpers::SessionMetrics {
        tok_per_sec: 42.0,
        duration_ms: 3100.0,
        input_tokens: 1761.0,
        output_tokens: 214.0,
        bytes: 9400.0,
    });

    let session = js::obj();
    js::set_num(&session, "contextWindow", 200000.0);

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
        .val("sandboxes", &targets(SANDBOXES))
        .val("projects", &targets(PROJECTS))
        .val("worktrees", &targets(WORKTREES))
        .val("repos", &targets(REPOS))
        .bool("sboxUp", true)
        .val("repoBranches", &targets(BRANCHES))
        .str("sandboxId", "sb-kiets")
        .str("projectId", "psicom/chat")
        // Nothing chosen, so the guide shows the null row checked too.
        .str("worktreeId", "")
        .str("branchId", "br-chat-main")
        .str("activeId", "samples")
        .str("model", "muse:opus")
        .val("models", &models())
        .val("sessions", &sessions())
        .val("tabs", &sessions())
        .bool("threadView", false)
        // ── the room ───────────────────────────────────────────────
        .val("others", &js_sys::Array::new().into())
        .val("branches", &js_sys::Array::new().into())
        .num("agentCount", 1.0)
        .bool("isGroup", false)
        .bool("hasBranches", false)
        .num("memCount", 0.0)
        .bool("compacting", false)
        // ── the composer ───────────────────────────────────────────
        .str("draft", "")
        .bool("waiting", false)
        .bool("stopping", false)
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
        .method("deleteSession", inert)
        .method("closeTab", inert)
        .method("newSession", inert)
        // The same clock *and* the same rendering the app uses: the guide's
        // project tree is the same component, so its ages must read `12m`,
        // not `12min ago`. `relAgo` is the prose form, for bylines.
        .method("relTime", crate::components::live::rel_time)
        .method("toggleCard", toggle_card)
        .method("accentOf", accent_of)
        .method("iconOf", icon_of)
        .method("relAgo", crate::components::live::rel_ago)
        .method("keepPinned", inert)
        .method("onScroll", inert)
        .method("forkPrompt", inert)
        .method("send", inert)
        .method("stop", inert)
        .method("compact", inert)
        .method("toggleThread", inert)
        .method("toggleThreadView", inert)
        .method("removeAgent", inert)
        .method("switchBranch", inert)
        .method("saveAllowlist", inert)
        .method("revertAllowlist", inert)
        .method("setModel", inert)
        .method("setPolicy", inert)
        .method("setReasoning", inert)
        .method("dropSession", drop_session)
        .method("loadProjects", inert)
        .method("loadTargets", inert)
        // The guide's form opens and validates; it persists nothing.
        .method("saveTarget", inert)
        .method("saveProject", inert)
        .method("pickTarget", pick_target)
        .method("editTarget", edit_target)
        .method("createTarget", create_target)
        .method("persistComposerSettings", inert)
        // The guide's rail heads have buttons; the guide creates nothing.
        .method("noop", inert);
    mjs::store(STORE, &d.finish());
}

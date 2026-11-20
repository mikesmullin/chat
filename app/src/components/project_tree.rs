//! `project tree` — the folders in the left rail, with their conversations
//! nested inside, and drag-and-drop to refile one.
//!
//! The sidebar already lists every session flat, newest first, which answers
//! "what was I just doing?" and nothing else. This answers the other
//! question — "where does this belong?" — and it is the only place the
//! project labels become a *shape* rather than a string on each row.
//!
//! ## The hierarchy is not stored anywhere
//!
//! A project is a name like `psicom/chat`. There is no parent column and no
//! tree table; the folders are read out of the separators, the way a
//! filesystem path implies its directories. So `psicom/chat` and
//! `psicom/sbox` produce three folders, and `psicom` exists even though
//! nothing was ever filed directly in it.
//!
//! That is why this file flattens rather than nests. m-js has no recursive
//! template, and a tree drawn by hand at three fixed depths is a tree that
//! breaks at four. [`rows`] walks the names once and emits a flat list
//! carrying a `depth`, which indents; the markup is then one `x-for` that
//! does not care how deep anything is.
//!
//! ```text
//!   ▾ psicom              2        <- implied: nothing is filed here directly
//!     ▾ chat              3
//!         iran war status     ◐ 3m   <- a conversation, draggable
//!         set lights blue      ● 1d
//!     ▸ sbox              1        <- collapsed: its children are skipped
//!   ▾ Unfiled             4        <- not a project; the drop target for "none"
//! ```
//!
//! Each conversation carries the shared `uiSessionState` symbol — `◐` working
//! (a turn is in flight), `●` done (an answer to review), `○` idle (nothing
//! said yet) — the same symbol the tab strip wears, then its age.
//!
//! ## Drag and drop
//!
//! A conversation is dragged onto a folder to refile it. `Unfiled` is a real
//! drop target rather than a label, because taking something *out* of a
//! folder has to be as easy as putting it in — otherwise the only way back
//! is the composer's picker, on a session you may not have open.
//!
//! Folders themselves are not draggable here. Moving a folder is a rename
//! (`psicom/chat` → `work/chat`), it takes every descendant with it, and it
//! is destructive enough to want a form and a confirmation rather than a
//! gesture that can be made by accident on the way to a scrollbar. The
//! inspector's project form does it.
//!
//! m-js has no `.prevent` event modifier, so the handlers call
//! `$event.preventDefault()` themselves — without it on `dragover` the drop
//! never fires, which presents as "drag does nothing" with no error.

use wasm_bindgen::{JsCast, JsValue};

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::components::rail_head;
use crate::components::session_state;
use crate::css;

const STORE: &str = "ptree";

pub const CSS: &str = r#"
.project-tree { display: flex; flex-direction: column; }

.ptree-list { display: flex; flex-direction: column; gap: 1px; }

/* One row shape for folders and conversations alike; `--d` is the depth the
   flattener worked out, so indentation is data rather than markup. */
.ptree-row {
  display: flex; align-items: center; gap: 6px;
  min-width: 0;
  padding: 4px 6px;
  padding-left: calc(6px + var(--d, 0) * 14px);
  border-radius: 7px;
  border: 1px solid transparent;
  color: var(--gray);
  font-size: 12px;
  line-height: 18px;
  cursor: pointer;
  user-select: none;
}
.ptree-row:hover { background: var(--panel-2); color: var(--fg); }

.ptree-caret {
  flex: 0 0 auto;
  width: 12px;
  font-size: 10px;
  color: var(--gray-dim);
  transition: transform .13s;
}
/* A folder with nothing under it still reserves the caret's width, so names
   line up whether or not a row can open. */
.ptree-row.is-leaf .ptree-caret { visibility: hidden; }
.ptree-row.is-open .ptree-caret { transform: rotate(90deg); }

.ptree-icon { flex: 0 0 auto; font-size: 13px; color: var(--gray-dim); }
.ptree-name {
  flex: 1 1 auto; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.ptree-count {
  flex: 0 0 auto;
  font-size: 11px;
  color: var(--gray-dim);
  font-variant-numeric: tabular-nums;
}
/* The conversation's age: pinned to the row's right edge. The name keeps
   `flex: 1 1 auto` plus ellipsis, so it takes exactly the space between
   icon and age and truncates rather than wrapping. */
.ptree-time {
  flex: 0 0 auto;
  font-size: 11px;
  color: var(--gray-dim);
  font-variant-numeric: tabular-nums;
}

/* A conversation reads lighter than the folder holding it: the folders are
   the structure, the conversations are the contents. */
.ptree-row.is-session .ptree-name { color: var(--gray); }
.ptree-row.is-session.is-active {
  background: var(--accent-tint);
  border-color: color-mix(in srgb, var(--accent) 40%, transparent);
}
.ptree-row.is-session.is-active .ptree-name { color: var(--fg); }

/* ---- drag and drop ----------------------------------------------
   The row being dragged fades; the folder under the pointer lights up.
   Both are necessary: without the first you lose track of what you picked
   up, and without the second you cannot tell which of two adjacent folders
   you are about to drop into. */
.ptree-row.is-dragging { opacity: .4; }
.ptree-row.is-drop {
  background: var(--accent-glow);
  border-color: var(--accent);
  color: var(--fg);
}
.ptree-row.is-drop .ptree-icon { color: var(--accent); }

.ptree-empty {
  margin: 0; padding: 10px 6px;
  color: var(--gray-dim); font-size: 11px;
}

/* demo framing — the rail is a narrow column, so the guide shows it at the
   width it actually gets. */
.ptree-demo { width: 280px; }
"#;

/// The rail section: a heading with a `+`, then the flattened tree.
///
/// `st` is the store holding `projects` and `sessions`; `on_open` and
/// `on_new` are expressions, so the guide's tree opens the guide's session
/// and the app's opens a real one.
pub fn tree(st: &str, on_open: &str, on_new: &str) -> Node {
    let rows = format!("$store.{STORE}.rows({st}.projects, {st}.sessions)");
    m("div").class("project-tree").kids(vec![
        rail_head::head("Projects", Some(("New project", on_new))),
        m("div").class("ptree-list").kids(vec![
            row(st, on_open),
            m("p")
                .class("ptree-empty")
                .show(&format!("{rows}.length === 0"))
                .child(lit("No projects yet. Use + to make one.")),
        ]),
    ])
}

/// One row — folder or conversation, told apart by `r.kind`.
///
/// Both are the same element because they share the indent, the hover and
/// the drop highlight; splitting them would mean keeping three rules in step
/// across two builders for no gain.
fn row(st: &str, on_open: &str) -> Node {
    let rows = format!("$store.{STORE}.rows({st}.projects, {st}.sessions)");
    let click = format!(
        "(r.kind === 'session' ? {} : $store.{STORE}.toggle(r.name))",
        on_open.replace("{}", "r.id")
    );
    m("div")
        .class("ptree-row")
        .for_each(&format!("r in {rows}"))
        .key("r.key")
        .bind("style", "('--d:' + r.depth)")
        .bind(
            "class",
            &format!(
                "((r.kind === 'session' ? 'is-session' : '') \
                 + (r.leaf ? ' is-leaf' : '') \
                 + (r.open ? ' is-open' : '') \
                 + (r.id && r.id === {st}.activeId ? ' is-active' : '') \
                 + ($store.{STORE}.dragId === r.id && r.kind === 'session' ? ' is-dragging' : '') \
                 + ($store.{STORE}.dropName === r.drop && r.kind !== 'session' ? ' is-drop' : ''))"
            ),
        )
        .bind("title", "r.name")
        // Only conversations move. A folder is moved by renaming it, which
        // is a form, not a gesture.
        .bind("draggable", "(r.kind === 'session' ? 'true' : 'false')")
        .on("click", &click)
        .on("dragstart", &format!("$store.{STORE}.dragStart(r.id)"))
        .on("dragend", &format!("$store.{STORE}.dragEnd()"))
        // Without `preventDefault` on dragover the browser never fires
        // `drop`, and the whole interaction silently does nothing.
        .on(
            "dragover",
            &format!(
                "$event.preventDefault(); \
                 (r.kind !== 'session' && $store.{STORE}.over(r.drop))"
            ),
        )
        .on("dragleave", &format!("$store.{STORE}.out(r.drop)"))
        .on(
            "drop",
            &format!(
                "$event.preventDefault(); \
                 (r.kind !== 'session' && {st}.dropSession($store.{STORE}.take(), r.drop))"
            ),
        )
        .kids(vec![
            m("i").class("ph ph-caret-right ptree-caret"),
            Node::icon_dyn("r.icon").class("ph ptree-icon"),
            m("span").class("ptree-name").text("r.label"),
            // The conversation's state, immediately left of its age: the
            // shared `uiSessionState` unit, so a tree row wears the same
            // symbol the tab strip and the agents rail wear. Folders have no
            // state of their own, so they render none.
            Node::host("uiSessionState", &[])
                .bind("tone", &session_state::tone_of("r"))
                .iff("r.kind === 'session'"),
            // Sessions only; folders carry no `created`, so the span stays
            // hidden on them. Empty `created` (a session predating the field)
            // is falsy too, which keeps the row's shape unchanged.
            m("span")
                .class("ptree-time relative-timestamp")
                .show("r.created")
                .text(&format!("{st}.relTime(r.created)")),
            m("span")
                .class("ptree-count")
                .show("r.count")
                .text("r.count"),
        ])
}

// ── the flattener ───────────────────────────────────────────────────────

/// Every folder implied by a set of project names, deepest paths included.
///
/// `psicom/chat` implies `psicom` as well — a parent nobody created is still
/// a folder, because its children have to hang from something.
fn folders(projects: &js_sys::Array) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in projects.iter() {
        let name = js::get_str(&p, "name");
        let mut acc = String::new();
        for seg in name.split('/').filter(|s| !s.is_empty()) {
            if !acc.is_empty() {
                acc.push('/');
            }
            acc.push_str(seg);
            if !out.contains(&acc) {
                out.push(acc.clone());
            }
        }
    }
    out.sort();
    out
}

fn row_obj(kind: &str, key: &str, depth: usize) -> JsValue {
    let o = js::obj();
    js::set_str(&o, "kind", kind);
    js::set_str(&o, "key", key);
    js::set_num(&o, "depth", depth as f64);
    o.into()
}

/// `rows(projects, sessions)` — the tree, flattened, honouring collapse.
///
/// Folders first in path order, each followed by the conversations filed
/// directly in it, then `Unfiled` last. A collapsed folder suppresses its
/// whole subtree, which is why this is one pass with a prefix check rather
/// than a per-folder recursion.
fn rows(_this: JsValue, args: js_sys::Array) -> JsValue {
    let out = js_sys::Array::new();
    let projects = args
        .get(0)
        .dyn_into::<js_sys::Array>()
        .unwrap_or_else(|_| js_sys::Array::new());
    let sessions = args
        .get(1)
        .dyn_into::<js_sys::Array>()
        .unwrap_or_else(|_| js_sys::Array::new());

    let st = mjs::store_ref(STORE);
    let collapsed: Vec<String> = js::get(&st, "collapsed")
        .dyn_into::<js_sys::Array>()
        .map(|a| a.iter().map(|v| v.as_string().unwrap_or_default()).collect())
        .unwrap_or_default();
    let is_collapsed = |name: &str| collapsed.iter().any(|c| c == name);
    // Hidden because an *ancestor* is shut, not this one.
    let hidden_under = |name: &str| {
        collapsed
            .iter()
            .any(|c| name.len() > c.len() && name.starts_with(&format!("{c}/")))
    };

    let session_project = |s: &JsValue| js::get_str(s, "project");

    for name in folders(&projects) {
        if hidden_under(&name) {
            continue;
        }
        let depth = name.matches('/').count();
        let shut = is_collapsed(&name);
        let label = name.rsplit('/').next().unwrap_or(&name).to_string();
        // Direct members only: the children are drawn beneath, and counting
        // them here would show every conversation twice on the way up.
        let mut n = 0usize;
        for s in sessions.iter() {
            if session_project(&s) == name {
                n += 1;
            }
        }
        // A folder is a leaf when nothing hangs under it — no conversations
        // of its own and no deeper folder.
        let has_child_folder = folders(&projects)
            .iter()
            .any(|f| f.len() > name.len() && f.starts_with(&format!("{name}/")));
        let r = row_obj("project", &format!("p:{name}"), depth);
        js::set_str(&r, "name", &name);
        js::set_str(&r, "label", &label);
        js::set_str(&r, "drop", &name);
        js::set_str(&r, "id", "");
        js::set_str(&r, "icon", if shut { "folder" } else { "folder-open" });
        js::set_num(&r, "count", n as f64);
        js::set_bool(&r, "open", !shut);
        js::set_bool(&r, "leaf", n == 0 && !has_child_folder);
        out.push(&r);

        if shut {
            continue;
        }
        for s in sessions.iter() {
            if session_project(&s) != name {
                continue;
            }
            out.push(&session_row(&s, depth + 1));
        }
    }

    // Unfiled: a drop target, not a project. Its `drop` is the empty string,
    // which is what the server reads as "unfile this".
    let loose: Vec<JsValue> = sessions
        .iter()
        .filter(|s| session_project(s).is_empty())
        .collect();
    if !loose.is_empty() {
        let shut = is_collapsed("");
        let r = row_obj("unfiled", "p:__unfiled", 0);
        js::set_str(&r, "name", "Unfiled");
        js::set_str(&r, "label", "Unfiled");
        js::set_str(&r, "drop", "");
        js::set_str(&r, "id", "");
        js::set_str(&r, "icon", "tray");
        js::set_num(&r, "count", loose.len() as f64);
        js::set_bool(&r, "open", !shut);
        js::set_bool(&r, "leaf", false);
        out.push(&r);
        if !shut {
            for s in &loose {
                out.push(&session_row(s, 1));
            }
        }
    }
    out.into()
}

fn session_row(s: &JsValue, depth: usize) -> JsValue {
    let id = js::get_str(s, "id");
    let title = js::get_str(s, "title");
    let r = row_obj("session", &format!("s:{id}"), depth);
    js::set_str(&r, "id", &id);
    js::set_str(&r, "name", &title);
    // An untitled conversation still needs something to drag.
    js::set_str(
        &r,
        "label",
        if title.trim().is_empty() {
            "Untitled"
        } else {
            &title
        },
    );
    js::set_str(&r, "drop", "");
    js::set_str(&r, "icon", "chat-circle");
    // When it was started: `relTime` formats the same `createdAt` field
    // everywhere, so no part of the UI can disagree about an age.
    js::set_str(&r, "created", &js::get_str(s, "createdAt"));
    // The state symbol reads these two. A row that carries neither renders
    // the widget's default tone, which looks like every conversation is done.
    js::set_str(&r, "status", &js::get_str(s, "status"));
    let turns = js::get(s, "turns");
    if !turns.is_undefined() && !turns.is_null() {
        js::set_num(&r, "turns", turns.as_f64().unwrap_or(0.0));
    }
    js::set_num(&r, "count", 0.0);
    js::set_bool(&r, "open", false);
    js::set_bool(&r, "leaf", true);
    r
}

// ── store ───────────────────────────────────────────────────────────────

fn toggle(_this: JsValue, args: js_sys::Array) -> JsValue {
    let name = js::arg_str(&args, 0);
    let st = mjs::store_ref(STORE);
    let cur = js::get(&st, "collapsed")
        .dyn_into::<js_sys::Array>()
        .unwrap_or_else(|_| js_sys::Array::new());
    let next = js_sys::Array::new();
    let mut found = false;
    for v in cur.iter() {
        if v.as_string().unwrap_or_default() == name {
            found = true;
        } else {
            next.push(&v);
        }
    }
    if !found {
        next.push(&JsValue::from_str(&name));
    }
    js::set(&st, "collapsed", &next.into());
    mjs::redraw();
    JsValue::UNDEFINED
}

fn drag_start(_this: JsValue, args: js_sys::Array) -> JsValue {
    js::set_str(&mjs::store_ref(STORE), "dragId", &js::arg_str(&args, 0));
    mjs::redraw();
    JsValue::UNDEFINED
}

/// Both the id and the highlight are cleared here, because `dragend` fires
/// whether the drop landed on a target or on nothing at all — a drag
/// abandoned over the page would otherwise leave a folder lit.
fn drag_end(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let st = mjs::store_ref(STORE);
    js::set_str(&st, "dragId", "");
    js::set_str(&st, "dropName", "\u{0}");
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `over(name)` — light this folder. No-op while nothing is being dragged,
/// so a plain mouse-over cannot highlight a drop target.
fn over(_this: JsValue, args: js_sys::Array) -> JsValue {
    let st = mjs::store_ref(STORE);
    if js::get_str(&st, "dragId").is_empty() {
        return JsValue::UNDEFINED;
    }
    let name = js::arg_str(&args, 0);
    if js::get_str(&st, "dropName") != name {
        js::set_str(&st, "dropName", &name);
        mjs::redraw();
    }
    JsValue::UNDEFINED
}

/// `out(name)` — unlight, but only if this row is the lit one. Leaving a
/// child row fires `dragleave` after the parent's `dragover`, so an
/// unconditional clear would flicker the highlight off the row the pointer
/// is actually on.
fn out(_this: JsValue, args: js_sys::Array) -> JsValue {
    let st = mjs::store_ref(STORE);
    if js::get_str(&st, "dropName") == js::arg_str(&args, 0) {
        js::set_str(&st, "dropName", "\u{0}");
        mjs::redraw();
    }
    JsValue::UNDEFINED
}

/// `take()` — the dragged id, consumed. The drop handler needs it *and*
/// needs it gone; returning and clearing in one call means a second drop
/// cannot replay the first.
fn take(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let st = mjs::store_ref(STORE);
    let id = js::get_str(&st, "dragId");
    js::set_str(&st, "dragId", "");
    js::set_str(&st, "dropName", "\u{0}");
    JsValue::from_str(&id)
}

/// standalone demo — the rail section over the guide's store.
pub fn demo() -> Node {
    let st = crate::components::samples::EXPR;
    m("div").class("ptree-demo").child(tree(
        st,
        &format!("{st}.openSession({{}})"),
        &format!("{st}.createTarget('project')"),
    ))
}

pub fn register() {
    css::inject("project-tree", CSS);
    let mut d = Def::new().named("store ptree");
    d.val("collapsed", &js_sys::Array::new().into())
        .str("dragId", "")
        // Not `""`: the empty string is a real drop target (Unfiled), so
        // "nothing is lit" needs a value no folder can have.
        .str("dropName", "\u{0}")
        .method("rows", rows)
        .method("toggle", toggle)
        .method("dragStart", drag_start)
        .method("dragEnd", drag_end)
        .method("over", over)
        .method("out", out)
        .method("take", take);
    mjs::store(STORE, &d.finish());
}

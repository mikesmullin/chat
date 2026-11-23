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
//!   ▾ psicom                       <- implied: nothing is filed here directly
//!     ▾ chat
//!         iran war status      3m   <- a conversation, draggable
//!         set lights blue      1d
//!     ▸ sbox                       <- collapsed: its children are skipped
//!   ▾ (ungrouped)                  <- not a project; the drop target for "none"
//! ```
//!
//! Each conversation carries the shared `uiSessionState` symbol — the same
//! ring the tab strip wears, tinted by state (amber working: a turn is in
//! flight; accent done: an answer to review; gray idle: nothing said yet)
//! and filled with the conversation's context usage — then its age.
//!
//! ## Drag and drop
//!
//! A conversation is dragged onto a folder to refile it. `(ungrouped)` is a real
//! drop target rather than a label, because taking something *out* of a
//! folder has to be as easy as putting it in — otherwise the only way back
//! is the composer's picker, on a session you may not have open.
//!
//! Folders themselves are not draggable here. Double-clicking a folder edits
//! its last path segment in place; moving it between parents remains in the
//! inspector. Double-clicking a conversation edits its title in the same row.
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
:is(#sb, #home) input.ptree-edit {
  flex: 1 1 auto; min-width: 0; width: 0;
  box-sizing: border-box; min-height: 0; height: 18px;
  padding: 0; border: 0; border-radius: 0;
  font: inherit; line-height: inherit; color: inherit;
  background: transparent; user-select: text;
}
:is(#sb, #home) input.ptree-edit:focus {
  outline: none; box-shadow: inset 0 -1px var(--gray-dim);
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
.ptree-time .ui-reltime { min-width: 0; font: inherit; color: inherit; }
/* Narrow shell (see home.rs breakpoints): the row is a centred glyph. Rows
   still toggle on click; the caret is just not given room. The row `title`
   still names the folder or session on hover. */
@media (max-width: 900px) {
  .ptree-row { justify-content: center; padding: 4px 0; }
  .ptree-row .ptree-caret,
  .ptree-row .ptree-name,
  .ptree-row .ptree-time { display: none; }
}

/* A conversation reads lighter than the folder holding it: the folders are
   the structure, the conversations are the contents. */
.ptree-row.is-session .ptree-name { color: var(--gray); }
.ptree-row.is-session.is-active {
  background: var(--accent-tint);
  border-color: color-mix(in srgb, var(--accent) 40%, transparent);
}
.ptree-row.is-session.is-active .ptree-name { color: var(--fg); }
/* Unread: the newest unread wins the eye without a colour, so it stays in
   the same family as the dimmed conversation text but reads bolder. */
.ptree-row.is-unread .ptree-name { font-weight: bold; }

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
    m("div").class("project-tree").bind("class", "($store.home.projectsOpen ? '' : 'is-collapsed')").kids(vec![
        rail_head::head("Projects", "folders", Some(("New project", on_new, "$store.ident.can('project.create')")), Some("projects")),
        m("div")
            .class("ptree-list")
            .show("$store.home.projectsOpen")
            .kids(vec![
            row(st, on_open),
            m("p")
                .class("ptree-empty")
                .show(&format!("{rows}.length === 0"))
                .child(lit("No projects yet.")),
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
        "(r.kind === 'session' ? {} : $store.{STORE}.toggle(r.drop))",
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
                 + (r.kind === 'session' && {st}.unreadSessions.includes(r.id) ? ' is-unread' : '') \
                 + ($store.{STORE}.dragId === r.id && r.kind === 'session' ? ' is-dragging' : '') \
                 + ($store.{STORE}.dropName === r.drop && r.kind !== 'session' ? ' is-drop' : ''))"
            ),
        )
        .bind("title", "r.name")
        // Only conversations move; folders can be moved in the inspector.
        .bind("draggable", "(r.kind === 'session' ? 'true' : 'false')")
        .on("click", &click)
        .on("dblclick", &format!("r.kind !== 'unfiled' && $store.{STORE}.edit(r.key, r.label)"))
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
            // The tooltip is the untruncated name: the label ellipsises on
            // screen, the hover shows all of it.
            m("span").class("ptree-name")
                .show(&format!("$store.{STORE}.editing !== r.key"))
                .bind("title", "r.name")
                .text("r.label"),
            m("input")
                .class("ptree-edit")
                .attr("type", "text")
                .attr("aria-label", "Edit name")
                .iff(&format!("$store.{STORE}.editing === r.key"))
                .model(&format!("$store.{STORE}.editDraft"))
                .init("$el.focus(); $el.select()")
                .on("click.stop", "")
                .on("dblclick.stop", "")
                .on("keydown.enter", &format!("$event.preventDefault(); $store.{STORE}.editing === r.key && ({st}.renameTreeRow(r.kind, r.name, r.id, $el.value), $store.{STORE}.finish())"))
                .on("keydown.escape", &format!("$store.{STORE}.finish()"))
                .on("focusout", &format!("$store.{STORE}.editing === r.key && ({st}.renameTreeRow(r.kind, r.name, r.id, $el.value), $store.{STORE}.finish())")),
            // The conversation's state, immediately left of its age: the
            // shared `uiSessionState` unit, so a tree row wears the same
            // symbol the tab strip and the agents rail wear. Folders have no
            // state of their own, so they render none. The ring fills with
            // this conversation's context window, and its tooltip names the
            // status and the counts behind the fill.
            Node::host("uiSessionState", &[])
                .bind("tone", &session_state::tone_of("r"))
                .bind("fraction", &session_state::fraction_of("r"))
                .bind("title", &session_state::hint_of("r"))
                .iff("r.kind === 'session'"),
            // Sessions only; folders have no event timestamp. The tooltip is
            // the two instants behind the age, in long form.
            Node::host("uiRelTime", &[])
                .class("ptree-time relative-timestamp")
                .bind("at", "r.lastEventAt")
                .bind("title", &stamp_hint("r"))
                .iff("r.lastEventAt"),
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
/// directly in it, newest event first, then `(ungrouped)` last. A collapsed folder suppresses its
/// whole subtree, which is why this is one pass with a prefix check rather
/// than a per-folder recursion.
fn rows(_this: JsValue, args: js_sys::Array) -> JsValue {
    let out = js_sys::Array::new();
    let projects = args
        .get(0)
        .dyn_into::<js_sys::Array>()
        .unwrap_or_else(|_| js_sys::Array::new());
    let mut sessions: Vec<JsValue> = args
        .get(1)
        .dyn_into::<js_sys::Array>()
        .unwrap_or_else(|_| js_sys::Array::new())
        .iter()
        .collect();
    sessions.sort_by(|a, b| {
        let time = |s: &JsValue| {
            let iso = session_event_time(s);
            let n = js_sys::Date::parse(&iso);
            if n.is_finite() { n } else { f64::NEG_INFINITY }
        };
        time(b).total_cmp(&time(a)).then_with(|| js::get_str(a, "id").cmp(&js::get_str(b, "id")))
    });

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
        // Direct members determine whether this folder is a leaf.
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

    // Ungrouped: a drop target, not a project. Its `drop` is the empty string,
    // which is what the server reads as "unfile this".
    let loose: Vec<JsValue> = sessions
        .iter()
        .filter(|s| session_project(s).is_empty())
        .cloned()
        .collect();
    if !loose.is_empty() {
        let shut = is_collapsed("");
        let r = row_obj("unfiled", "p:__unfiled", 0);
        js::set_str(&r, "name", "(ungrouped)");
        js::set_str(&r, "label", "(ungrouped)");
        js::set_str(&r, "drop", "");
        js::set_str(&r, "id", "");
        js::set_str(&r, "icon", "tray");
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

/// The displayed age and sort order share the same fallback for empty sessions.
fn session_event_time(s: &JsValue) -> String {
    let last = js::get_str(s, "lastEventAt");
    if last.is_empty() { js::get_str(s, "createdAt") } else { last }
}

/// The age's tooltip: `created {long} modified {long}`.
fn stamp_hint(row: &str) -> String {
    format!(
        "'created ' + {created} + ' modified ' + {modified}",
        created = long_stamp(&format!("{row}.createdAt")),
        modified = long_stamp(&format!("{row}.updatedAt")),
    )
}

/// Long form for one ISO timestamp expression: `Sun, Nov 22 1:04PM MST` —
/// weekday, month and day, then 12-hour time with the zone, minus the space
/// before the meridiem, in the viewer's own locale and zone. An empty
/// timestamp is an em dash rather than the browser's "Invalid Date".
fn long_stamp(iso: &str) -> String {
    format!(
        "(({iso}) ? (new Date({iso}).toLocaleDateString('en-US', \
         {{weekday: 'short', month: 'short', day: 'numeric'}}) + ' ' + \
         new Date({iso}).toLocaleTimeString('en-US', \
         {{hour: 'numeric', minute: '2-digit', timeZoneName: 'short'}}) \
         .replace(' ', '')) : '—')"
    )
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
    // Use the most recent event, falling back for a session with no events.
    js::set_str(&r, "lastEventAt", &session_event_time(s));
    // The state symbol reads these two. A row that carries neither renders
    // the widget's default tone, which looks like every conversation is done.
    js::set_str(&r, "status", &js::get_str(s, "status"));
    let turns = js::get(s, "turns");
    if !turns.is_undefined() && !turns.is_null() {
        js::set_num(&r, "turns", turns.as_f64().unwrap_or(0.0));
    }
    // The working ring's fill: how full this conversation's context window
    // is. `totalTokens` is the last generation's total — the row contract's
    // copy of what the `usage` row streams to the composer's context pie.
    js::set(&r, "contextWindow", &js::get(s, "contextWindow"));
    js::set(&r, "totalTokens", &js::get(s, "totalTokens"));
    // The age's tooltip names the two instants behind it.
    js::set(&r, "createdAt", &js::get(s, "createdAt"));
    js::set(&r, "updatedAt", &js::get(s, "updatedAt"));
    js::set_bool(&r, "open", false);
    js::set_bool(&r, "leaf", true);
    r
}

// ── store ───────────────────────────────────────────────────────────────

/// Which groups were shut is a per-browser memory (a reload should return
/// to the tree you left), kept the way `home` keeps its rail sections.
const COLLAPSED_KEY: &str = "chat.ptree.collapsed";

fn collapsed_from_storage() -> JsValue {
    let storage = js::get(&js::global(), "localStorage");
    let text = js::call(&storage, "getItem", &[JsValue::from_str(COLLAPSED_KEY)])
        .as_string()
        .unwrap_or_default();
    let out = js_sys::Array::new();
    if !text.is_empty() {
        let json = js::get(&js::global(), "JSON");
        let parsed = js::call(&json, "parse", &[JsValue::from_str(&text)]);
        if js_sys::Array::is_array(&parsed) {
            for v in js_sys::Array::from(&parsed).iter() {
                if let Some(s) = v.as_string() {
                    out.push(&JsValue::from_str(&s));
                }
            }
        }
    }
    out.into()
}

fn collapsed_to_storage(collapsed: &JsValue) {
    let storage = js::get(&js::global(), "localStorage");
    let json = js::get(&js::global(), "JSON");
    let text = js::call(&json, "stringify", &[collapsed.clone()]);
    js::call(
        &storage,
        "setItem",
        &[JsValue::from_str(COLLAPSED_KEY), text],
    );
}

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
    let merged: JsValue = next.into();
    js::set(&st, "collapsed", &merged);
    collapsed_to_storage(&merged);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn edit(_this: JsValue, args: js_sys::Array) -> JsValue {
    let st = mjs::store_ref(STORE);
    js::set_str(&st, "editDraft", &js::arg_str(&args, 1));
    js::set_str(&st, "editing", &js::arg_str(&args, 0));
    mjs::redraw();
    JsValue::UNDEFINED
}

fn finish(_this: JsValue, _args: js_sys::Array) -> JsValue {
    js::set_str(&mjs::store_ref(STORE), "editing", "");
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
        &format!("{st}.previewSession({{}})"),
        &format!("{st}.createTarget('project')"),
    ))
}

pub fn register() {
    css::inject("project-tree", CSS);
    let mut d = Def::new().named("store ptree");
    d.val("collapsed", &collapsed_from_storage())
        .str("editing", "")
        .str("editDraft", "")
        .str("dragId", "")
        // Not `""`: the empty string is a real drop target ((ungrouped)), so
        // "nothing is lit" needs a value no folder can have.
        .str("dropName", "\u{0}")
        .method("rows", rows)
        .method("toggle", toggle)
        .method("edit", edit)
        .method("finish", finish)
        .method("dragStart", drag_start)
        .method("dragEnd", drag_end)
        .method("over", over)
        .method("out", out)
        .method("take", take);
    mjs::store(STORE, &d.finish());
}

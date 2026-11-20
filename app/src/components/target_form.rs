//! `target form` — the inspector panel behind the composer row's **Create**
//! and **Edit** actions.
//!
//! The four pickers under the composer can each make a new instance, and
//! each row in them can be edited. Neither belongs in the menu: a create is
//! a form with fields and failure modes, and a menu that grows a form is a
//! menu you cannot dismiss by picking something. So the menu closes and the
//! third column takes over — the same column, and the same chrome, that
//! editing an agent already uses.
//!
//! ## It reuses the agent editor's clothes on purpose
//!
//! `.agent-editor`, `.ae-field`, `.ae-label`, `.ae-actions`, `.ae-btn` and
//! `.ae-error` are defined once, in
//! [`agent_editor`](crate::components::agent_editor), and the settings panel
//! already borrows them. A third set of inspector-form styles would be a
//! third thing to keep in step with the first two, and the inspector should
//! look like one place rather than three panels that happen to share a
//! column.
//!
//! ## Why it is told which store to write to
//!
//! `openCreate(kind, store)` carries the name of the store that opened it,
//! and `save` calls back into that store. The design guide's form therefore
//! writes to the guide's store and the app's to the session — one form
//! definition, two destinations, no copy. It is the same seam the picker
//! uses for `on_pick`, spelled for a panel rather than an expression.
//!
//! ## What works today
//!
//! `project` is complete: create, rename (which carries every child folder
//! and conversation with it) and delete (which unfiles rather than deletes).
//! The other three kinds open and say what they are waiting on, because a
//! button that opens nothing reads as broken, and a form that silently fails
//! to save reads as worse.

use wasm_bindgen::JsValue;

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::css;

const STORE: &str = "tform";
const ST: &str = "$store.tform";

pub const CSS: &str = r#"
/* Only what the agent editor has no equivalent of. Everything else —
   fields, labels, the action row, the error line — comes from there. */
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
/* The path a name implies, shown under the field as it is typed, so the
   folder you are about to make is visible before you make it. */
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

.tform-demo { width: 320px; }
"#;

/// The panel. Mounted in the inspector beside the agent editor, and shown by
/// its own flag, exactly as the other two panels there are.
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
                m("button")
                    .class("ae-btn")
                    .attr("type", "button")
                    .attr("title", "Close")
                    .on("click", &format!("{ST}.close()"))
                    .child(Node::icon("x").class("ph ph-x")),
            ]),
            // ---- the project form ------------------------------------
            m("div")
                .class("ae-field")
                .show(&format!("{ST}.kind === 'project'"))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![
                        m("span").child(lit("Name")),
                        m("span")
                            .class("hint")
                            .child(lit("use / to nest, e.g. psicom/chat")),
                    ]),
                    m("input")
                        .attr("type", "text")
                        .attr("spellcheck", "false")
                        .attr("placeholder", "psicom/chat")
                        .model(&format!("{ST}.name"))
                        // Enter saves: this form is one field, and reaching
                        // for the mouse to commit a single word is friction
                        // the agent editor does not have because it has ten.
                        .on("keydown.enter", &format!("{ST}.save()")),
                ]),
            // The hierarchy the name implies, as it is typed.
            m("div")
                .class("tform-path")
                .show(&format!("{ST}.kind === 'project' && {ST}.segments().length > 1"))
                .kids(vec![m("span")
                    .class("seg")
                    .for_each(&format!("seg in {ST}.segments()"))
                    .key("seg")
                    .text("seg")]),
            // In edit mode these kinds have nothing to change: the panel
            // exists to show which row it is and to offer Delete.
            m("p")
                .class("tform-note")
                .show(&format!("{ST}.mode === 'edit' && {ST}.kind !== 'project'"))
                .text(&format!("{ST}.original")),
            // ---- repo: an absolute path, because nothing is scanned ----
            m("div")
                .class("ae-field")
                .show(&format!("{ST}.kind === 'repo' && {ST}.mode !== 'edit'"))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![
                        m("span").child(lit("Path")),
                        m("span").class("hint").child(lit("absolute, on this machine")),
                    ]),
                    m("input")
                        .attr("type", "text")
                        .attr("spellcheck", "false")
                        .attr("placeholder", "/workspace/psicom/chat")
                        .model(&format!("{ST}.path"))
                        .on("keydown.enter", &format!("{ST}.save()")),
                ]),
            // ---- branch and worktree both hang off a repo --------------
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
            // A worktree can check out a branch that does not exist yet —
            // one `git worktree add -b`, so there is no window in which the
            // branch exists and the checkout does not.
            m("label")
                .class("ae-label")
                .show(&format!("{ST}.kind === 'worktree' && {ST}.mode !== 'edit'"))
                .kids(vec![
                    m("input")
                        .attr("type", "checkbox")
                        .model(&format!("{ST}.create")),
                    m("span").child(lit("create this branch too")),
                ]),
            // ---- sandbox ----------------------------------------------
            m("div")
                .class("ae-field")
                .show(&format!("{ST}.kind === 'sandbox' && {ST}.mode !== 'edit'"))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![
                        m("span").child(lit("Image")),
                        m("span").class("hint").child(lit("headless has no desktop")),
                    ]),
                    m("select").model(&format!("{ST}.image")).kids(vec![
                        m("option").attr("value", "headless").child(lit("headless")),
                        m("option").attr("value", "headed").child(lit("headed")),
                    ]),
                ]),
            m("label")
                .class("ae-label")
                .show(&format!("{ST}.kind === 'sandbox' && {ST}.mode !== 'edit'"))
                .kids(vec![
                    m("input")
                        .attr("type", "checkbox")
                        .model(&format!("{ST}.network")),
                    m("span").child(lit("give it network access")),
                ]),
            // What it will mount, decided by the row above rather than by
            // this form: the sandbox gets whatever checkout the composer has
            // selected, which is the whole point of the row.
            m("p")
                .class("tform-note")
                .show(&format!("{ST}.kind === 'sandbox' && {ST}.mode !== 'edit'"))
                .text(&format!("{ST}.mountNote")),
            m("p")
                .class("tform-note")
                .show(&format!("{ST}.note"))
                .text(&format!("{ST}.note")),
            m("p")
                .class("ae-error")
                .show(&format!("{ST}.error"))
                .text(&format!("{ST}.error")),
            m("div").class("ae-actions").kids(vec![
                m("button")
                    .class("ae-btn primary")
                    .attr("type", "button")
                    // Only a project can be renamed. A sandbox's id is
                    // sbox-b's, a repo's identity is its path on disk and a
                    // worktree's is its branch — none of them has a name
                    // this form could change, so in edit mode the button is
                    // not disabled, it is absent.
                    .show(&format!("({ST}.mode !== 'edit' || {ST}.kind === 'project')"))
                    .bind("disabled", &format!("({ST}.busy || !{ST}.ready())"))
                    .on("click", &format!("{ST}.save()"))
                    .text(&format!(
                        "({ST}.busy ? 'Working…' : ({ST}.mode === 'edit' ? 'Rename' : 'Create'))"
                    )),
                m("span").class("spacer"),
                // Two-step, like the agent editor's: the first press arms
                // it and says so, the second does it. A folder holding
                // forty conversations is not a thing to lose to one click.
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

/// standalone demo — the form as the inspector shows it, over the guide.
pub fn demo() -> Node {
    m("div")
        .class("tform-demo")
        .init(&format!("{ST}.openCreate('project', 'samples')"))
        .child(panel())
}

// ── store ───────────────────────────────────────────────────────────────

/// What each kind is called and what glyph it wears.
fn kind_info(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "project" => ("Project", "folder-plus"),
        "sandbox" => ("Sandbox", "desktop"),
        "worktree" => ("Worktree", "tree-structure"),
        "branch" => ("Branch", "git-branch"),
        "repo" => ("Repo", "folder-open"),
        _ => ("", "question"),
    }
}

/// What a confirmed delete will actually do — different enough per kind
/// that one sentence cannot cover it, and each one is the thing you want to
/// read before the second click.
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
            "Destroying a sandbox removes the container. A worktree chat mounted into \
             it goes too; a repo you registered does not."
        }
        _ => "",
    }
}

fn open_with(kind: &str, mode: &str, store: &str, name: &str) {
    let (label, icon) = kind_info(kind);
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "open", true);
    js::set_str(&s, "kind", kind);
    js::set_str(&s, "mode", mode);
    js::set_str(&s, "store", store);
    js::set_str(&s, "name", name);
    js::set_str(&s, "original", name);
    // The bare Phosphor name: the template hands this to `Node::icon_dyn`,
    // which supplies the `ph ph-` prefix itself. Writing the prefix here too
    // yields `ph ph-ph-folder-plus`, which draws nothing.
    js::set_str(&s, "icon", icon);
    js::set_str(&s, "dangerNote", danger_note(kind));
    js::set_bool(&s, "busy", false);
    js::set_str(&s, "path", "");
    js::set_bool(&s, "create", true);
    js::set_str(&s, "image", "headless");
    js::set_bool(&s, "network", true);
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
    // The inspector shows one panel at a time; opening this one puts the
    // agent editor away rather than stacking two forms in a column.
    if mjs::has_store("agents") {
        js::set_bool(&mjs::store_ref("agents"), "open", false);
    }
    if mjs::has_store("settings") {
        js::set_bool(&mjs::store_ref("settings"), "open", false);
    }
    if mjs::has_store("sessionDraft") {
        js::set_bool(&mjs::store_ref("sessionDraft"), "open", false);
    }
    mjs::redraw();
}

/// Fill in what the host store already knows, so the form opens on a
/// sensible answer rather than empty.
///
/// A branch or worktree form defaults to the first registered repo, because
/// with one repo registered — the common case — picking it again is a step
/// that teaches nothing. A sandbox form says which checkout it is about to
/// mount, which is decided by the composer row rather than by this form.
fn seed_from_host(s: &JsValue, kind: &str, mode: &str, name: &str) {
    let host_name = js::get_str(s, "store");
    if host_name.is_empty() || !mjs::has_store(&host_name) {
        return;
    }
    let host = mjs::store_ref(&host_name);
    js::set_str(s, "note", "");
    js::set_str(s, "mountNote", "");

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
        // Editing a branch row means its `<repoId>:<branch>` id: split it
        // back apart so the form opens on the repo it belongs to.
        if mode == "edit" {
            if let Some((repo, branch)) = name.split_once(':') {
                js::set_str(s, "repoId", repo);
                js::set_str(s, "name", branch);
            }
        }
    }

    if kind == "sandbox" {
        if !js::get_bool(&host, "sboxUp") {
            js::set_str(
                s,
                "note",
                "The sandbox service is not answering. Start sbox-b and reopen this.",
            );
        }
        let wt = js::get_str(&host, "worktreeId");
        let br = js::get_str(&host, "branchId");
        js::set_str(
            s,
            "mountNote",
            &if !wt.is_empty() {
                format!(
                    "Mounts the selected worktree at /work, read-write. Destroying this \
                     sandbox will remove that worktree."
                )
            } else if !br.is_empty() {
                "Mounts the selected repo at /work, read-write. Note this mounts the \
                 clone as it currently stands — it does not switch its branch. Pick a \
                 worktree if you want this box on a branch of its own."
                    .to_string()
            } else {
                "Nothing is selected in the row below the composer, so this sandbox \
                 will have no checkout mounted."
                    .to_string()
            },
        );
    }
}

/// `repos()` — the host store's registered repos, for the picker in the form.
fn repos(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let host_name = js::get_str(&s, "store");
    if host_name.is_empty() || !mjs::has_store(&host_name) {
        return js_sys::Array::new().into();
    }
    js::get(&mjs::store_ref(&host_name), "repos")
}

/// `ready()` — is there enough here to submit?
///
/// Per kind, because the required field differs: a repo needs a path, a
/// branch needs a name and a repo, a sandbox needs nothing at all.
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
        "sandbox" => true,
        _ => false,
    };
    JsValue::from_bool(ok)
}

/// `deletable()` — every kind can be removed except a branch.
///
/// Deleting a git branch from a dropdown's pencil is a destructive action
/// with no undo and no obvious blast radius, and it is one `git branch -d`
/// away in a terminal. The form does not offer it.
fn deletable(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let kind = js::get_str(&mjs::store_ref(STORE), "kind");
    JsValue::from_bool(matches!(
        kind.as_str(),
        "project" | "repo" | "worktree" | "sandbox"
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
    js::set_bool(&s, "open", false);
    js::set_bool(&s, "confirming", false);
    js::set_str(&s, "error", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `segments()` — the folders the typed name implies, for the preview.
fn segments(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let out = js_sys::Array::new();
    let name = js::get_str(&mjs::store_ref(STORE), "name");
    for seg in name.split('/').map(str::trim).filter(|s| !s.is_empty()) {
        out.push(&JsValue::from_str(seg));
    }
    out.into()
}

/// Same rules as the server's `normalize`, so the form rejects what the
/// server would reject instead of round-tripping to find out.
///
/// It is a copy of a rule, which is a thing worth being uneasy about — but
/// the server still validates, and the alternative is a form that cannot say
/// "that name is empty" without asking.
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

/// Build the payload for a host-side write and send it.
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
                    // Empty start point: git branches from HEAD, which is
                    // what "make a branch here" means.
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
            "sandbox" => {
                let host = mjs::store_ref(&js::get_str(&s, "store"));
                send(
                    &s,
                    "sandbox-add",
                    &[
                        ("kind", JsValue::from_str(&js::get_str(&s, "image"))),
                        ("network", JsValue::from_bool(js::get_bool(&s, "network"))),
                        // The checkout comes from the composer row, not from
                        // this form — the row is where that choice lives.
                        (
                            "worktreeId",
                            JsValue::from_str(&js::get_str(&host, "worktreeId")),
                        ),
                        (
                            "branchId",
                            JsValue::from_str(&js::get_str(&host, "branchId")),
                        ),
                    ],
                )
            }
            _ => {}
        }
        return JsValue::UNDEFINED;
    }
    let name = js::get_str(&s, "name");
    if let Some(msg) = invalid(&name) {
        js::set_str(&s, "error", &msg);
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    let host = mjs::store_ref(&js::get_str(&s, "store"));
    let original = js::get_str(&s, "original");
    let editing = js::get_str(&s, "mode") == "edit";
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

/// First press arms, second press deletes. Any other interaction disarms it,
/// which is why `close` and `open_with` both clear the flag.
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
        _ => return JsValue::UNDEFINED,
    };
    // These close on the response rather than now: a destroy that fails
    // needs somewhere to say so, and `saveTarget` keeps the panel open on
    // an error for exactly that.
    send(&s, action, &[("id", JsValue::from_str(&original))]);
    JsValue::UNDEFINED
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
        .str("heading", "")
        .str("icon", "folder-plus")
        .str("note", "")
        .str("mountNote", "")
        .str("dangerNote", "")
        .str("error", "")
        .str("path", "")
        .str("repoId", "")
        .str("image", "headless")
        .bool("network", true)
        .bool("create", true)
        .bool("busy", false)
        .bool("confirming", false)
        .method("openCreate", open_create)
        .method("openEdit", open_edit)
        .method("close", close)
        .method("save", save)
        .method("remove", remove)
        .method("segments", segments)
        .method("repos", repos)
        .method("ready", ready)
        .method("deletable", deletable);
    mjs::store(STORE, &d.finish());
}

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
//! `.agent-editor`, `.ae-field`, `.ae-label`, `.ae-actions` and `.ae-btn`
//! are defined once, in
//! [`agent_editor`](crate::components::agent_editor), and the settings panel
//! already borrows them. The verdict banner is
//! [`validation_info`](crate::components::validation_info)'s alone. A third
//! set of inspector-form styles would be a
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
use crate::components::{dropdown, validation_info};

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
/* A note that blocks you (bad service, no repos) is an error, not a hint: red
   ink on a tinted danger surface, with the phosphor error glyph so it reads
   as "stop" rather than as a quiet aside. */
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

/* The icon and its animation belong to the status field in this form. */
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
            ]),
            // The verdict banner: always at the top of the form, just
            // above the first field — one brick, every aside form.
            validation_info::live("'error'", &format!("{ST}.error")),
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
            // The label you give the box is what the sandbox menu reads; the
            // id sbox-b hands back is a key, not something a person should
            // squint at every time.
            // x-if, not x-show: the field remounts on every open, so its
            // init fires each time and the cursor lands in a fresh input —
            // the one case where show-then-focus would silently do nothing.
            m("div")
                .class("ae-field")
                .iff(&format!("{ST}.kind === 'sandbox'"))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![
                        m("span").child(lit("Name")),
                        m("span").class("hint").child(lit("shown in the menu")),
                    ]),
                    m("input")
                        .attr("type", "text")
                        .attr("spellcheck", "false")
                        .model(&format!("{ST}.name"))
                        .on("keydown.enter", &format!("{ST}.save()"))
                        .init("$el.focus()"),
                ]),
            m("div")
                .class("ae-field")
                .show(&format!("{ST}.kind === 'sandbox' && {ST}.mode !== 'edit'"))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![
                        m("span").child(lit("Image")),
                        m("span").class("hint").child(lit("headless has no desktop")),
                    ]),
                    dropdown::choice("sandbox-image", "desktop", &format!("{ST}.image"),
                        &["headless", "headed"], &format!("{ST}.setImage('{{}}')")),
                ]),
            m("div").class("ae-field")
                .show(&format!("{ST}.kind === 'sandbox' && {ST}.mode === 'edit'"))
                .kids(vec![m("label").class("ae-label").child(lit("Status")),
                    m("span").class("tform-status").attr("role", "status")
                        .bind("class", &format!("{ST}.statusClass({ST}.status)"))
                        .kids(vec![
                            Node::icon_dyn(&format!("{ST}.statusIcon({ST}.status)"))
                                .class("tform-status-icon").attr("aria-hidden", "true"),
                            m("span").text(&format!("{ST}.status")),
                        ])]),
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
            // ---- automation -------------------------------------------
            // A named prompt on a cron: firing creates a fresh session and
            // submits the prompt there. Every field here travels with the
            // save; the schedule is validated for real on the server.
            m("div")
                .class("ae-field")
                .show(&format!("{ST}.kind === 'automation'"))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![
                        m("span").child(lit("Name")),
                        m("span").class("hint").child(lit("shown in the rail")),
                    ]),
                    m("input")
                        .attr("type", "text")
                        .attr("spellcheck", "false")
                        .attr("placeholder", "issue-triage")
                        .model(&format!("{ST}.name"))
                        .on("keydown.enter", &format!("{ST}.save()")),
                ]),
            m("label")
                .class("ae-label")
                .show(&format!("{ST}.kind === 'automation'"))
                .kids(vec![
                    m("input")
                        .attr("type", "checkbox")
                        .model(&format!("{ST}.enabled")),
                    m("span").child(lit("active")),
                ]),
            // Catch-up (B3): the machine was off when the schedule came
            // round — run it once at startup, or let it go. Off by default:
            // a backfill nobody asked for is a surprise bill.
            m("label")
                .class("ae-label")
                .show(&format!("{ST}.kind === 'automation'"))
                .kids(vec![
                    m("input")
                        .attr("type", "checkbox")
                        .model(&format!("{ST}.catchUp")),
                    m("span").child(lit("catch up missed runs at startup")),
                ]),
            m("div")
                .class("ae-field")
                .show(&format!("{ST}.kind === 'automation'"))
                .kids(vec![
                    m("label").class("ae-label").kids(vec![
                        m("span").child(lit("Schedule")),
                        m("span").class("hint").child(lit("presets fill the cron")),
                    ]),
                    m("div").class("ae-modes").kids(vec![
                        cron_preset("Hourly", "0 * * * *"),
                        cron_preset("Every morning", "0 9 * * *"),
                        cron_preset("Weekdays 9:00", "0 9 * * 1-5"),
                        cron_preset("Mondays 9:00", "0 9 * * 1"),
                        cron_preset("Fridays 17:00", "0 17 * * 5"),
                        cron_custom(),
                    ]),
                    m("input")
                        .class("tform-cron")
                        .attr("type", "text")
                        .attr("spellcheck", "false")
                        .attr("placeholder", "0 9 * * 1-5")
                        .model(&format!("{ST}.cron"))
                        .on("keydown.enter", &format!("{ST}.save()")),
                ]),
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
                    m("select").model(&format!("{ST}.agent")).kids(vec![
                        m("option").attr("value", "").child(lit("Default")),
                        m("option")
                            .for_each("a in $store.agents.agents")
                            .key("a.name")
                            .bind("value", "a.name")
                            .text("a.name"),
                    ]),
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
                    // Only a project, a sandbox and an automation have a
                    // renameable name. A repo's identity is its path on disk
                    // and a worktree's is its branch — neither of those can
                    // be renamed from here, so in edit mode the button is
                    // not disabled, it is absent.
                    .show(&format!("({ST}.mode !== 'edit' || {ST}.kind === 'project' || {ST}.kind === 'sandbox' || {ST}.kind === 'automation')"))
                    .bind("disabled", &format!("({ST}.busy || !{ST}.ready())"))
                    .on("click", &format!("{ST}.save()"))
                    .text(&format!(
                        "({ST}.busy ? 'Working…' : ({ST}.mode === 'edit' ? ({ST}.kind === 'automation' ? 'Save' : 'Rename') : 'Create'))"
                    )),
                // The share dialog reaches projects, sandboxes and
                // automations too (PLAN44 C2) — same dialog, same grants.
                m("button")
                    .class("ae-btn")
                    .attr("type", "button")
                    .show(&format!("({ST}.mode === 'edit' && ['project', 'sandbox', 'automation'].includes({ST}.kind))"))
                    .on("click", "$store.share.open($store.tform.kind, $store.tform.name)")
                    .kids(vec![
                        Node::icon("users-three").class("ph ph-users-three"),
                        m("span").child(lit("Share…")),
                    ]),
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
pub fn demo() -> Node { demo_for("project") }

pub fn demo_sandbox() -> Node { demo_for("sandbox") }

pub fn demo_automation() -> Node { demo_for("automation") }

fn demo_for(kind: &str) -> Node {
    m("div").class("tform-demo")
        .init(&format!("{ST}.openCreate('{kind}', 'samples')"))
        .child(panel())
}

// ── store ───────────────────────────────────────────────────────────────

/// What each kind is called and what glyph it wears.
fn kind_info(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "project" => ("Project", "folder-plus"),
        "sandbox" => ("Sandbox", "desktop"),
        "automation" => ("Automation", "timer"),
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
    js::set_bool(&s, "open", true);
    // One panel at a time: the inspector shows what you last selected.
    js::set_bool(&mjs::store_ref("ident"), "panelOpen", false);
    js::set_bool(&mjs::store_ref("settings"), "open", false);
    js::set_str(&s, "kind", kind);
    js::set_str(&s, "mode", mode);
    js::set_str(&s, "store", store);
    js::set_str(&s, "name", name);
    js::set_str(&s, "original", name);
    js::set_str(&s, "status", "Unknown");
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
    js::set_str(&s, "cron", "");
    js::set_str(&s, "prompt", "");
    js::set_str(&s, "agent", "");
    js::set_str(&s, "model", "");
    js::set_str(&s, "timeout", "60");
    js::set_bool(&s, "enabled", true);
    js::set_bool(&s, "catchUp", false);
    seed_from_host(&s, kind, mode, name);
    if kind == "sandbox" && store == "live" { sync_sandbox_status(&mjs::store_ref(store)); }
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
    // ...and the column it is drawn in has to be there to show it. The
    // inspector starts collapsed, so a form opened into it without this is a
    // `+` that appears to do nothing. Harmless on `/storybook`, which
    // registers the same `home` store but has no third column to collapse.
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
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

    if kind == "automation" {
        if mode == "edit" {
            // The id came in; the form opens on the stored schedule, not on
            // blanks. A missing row (deleted elsewhere) leaves the seeds
            // empty and the save round-trips the server's "no such" back.
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
        if mode == "edit" {
            // The id came in; the form should open on the label that row is
            // already showing, not a raw hex id.
            let id = name;
            let sandboxes = js_sys::Array::from(&js::get(&host, "sandboxes"));
            for item in sandboxes.iter() {
                if js::get_str(&item, "id") == id {
                    let state = js::get_str(&item, "state");
                    if !state.is_empty() { js::set_str(s, "status", &state); }
                    let display = js::get_str(&item, "name");
                    if !display.is_empty() {
                        js::set_str(s, "name", &display);
                    }
                    break;
                }
            }
        }
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
/// branch needs a name and a repo, a sandbox needs a name.
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
        "sandbox" => !js::get_str(&s, "name").trim().is_empty(),
        "automation" => {
            !js::get_str(&s, "name").trim().is_empty()
                && !js::get_str(&s, "prompt").trim().is_empty()
                && cron_shape_ok(&js::get_str(&s, "cron"))
        }
        _ => false,
    };
    JsValue::from_bool(ok)
}

/// The cron's shape, client-side: five to seven space-separated fields, or
/// an `@`-shorthand. The server parses for real on save; this just gates
/// the obvious empties so the button does not promise what it cannot do.
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

/// `deletable()` — every kind can be removed except a branch.
///
/// Deleting a git branch from a dropdown's pencil is a destructive action
/// with no undo and no obvious blast radius, and it is one `git branch -d`
/// away in a terminal. The form does not offer it.
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
                // An edit carries the id so the save replaces; a create
                // omits it so the server mints one.
                if js::get_str(&s, "mode") == "edit" {
                    let original = js::get_str(&s, "original");
                    fields.push(("id", JsValue::from_str(&original)));
                }
                send(&s, "automation-save", &fields)
            }
            "sandbox" => {
                let host = mjs::store_ref(&js::get_str(&s, "store"));
                if name.is_empty() {
                    crate::components::validation_info::note("tform", "error", "give the sandbox a name");
                    mjs::redraw();
                    return JsValue::UNDEFINED;
                }
                let editing = js::get_str(&s, "mode") == "edit";
                if editing {
                    // Rename only the label; sbox-b's id never changes.
                    send(
                        &s,
                        "sandbox-rename",
                        &[
                            ("id", JsValue::from_str(&js::get_str(&s, "original"))),
                            ("name", JsValue::from_str(&name)),
                        ],
                    )
                } else {
                    send(
                        &s,
                        "sandbox-add",
                        &[
                            ("name", JsValue::from_str(&name)),
                            ("kind", JsValue::from_str(&js::get_str(&s, "image"))),
                            ("network", JsValue::from_bool(js::get_bool(&s, "network"))),
                            // The checkout comes from the composer row, not
                            // from this form — the row is where that choice
                            // lives.
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
            }
            _ => {}
        }
        return JsValue::UNDEFINED;
    }
    let name = js::get_str(&s, "name");
    if let Some(msg) = invalid(&name) {
        crate::components::validation_info::note("tform", "error", &msg);
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
        "automation" => "automation-remove",
        _ => return JsValue::UNDEFINED,
    };
    // These close on the response rather than now: a destroy that fails
    // needs somewhere to say so, and `saveTarget` keeps the panel open on
    // an error for exactly that.
    send(&s, action, &[("id", JsValue::from_str(&original))]);
    JsValue::UNDEFINED
}

/// One mapping for the label's icon and styling, including unknown states.
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

fn status_class(_this: JsValue, args: js_sys::Array) -> JsValue {
    JsValue::from_str(status_appearance(&js::arg_str(&args, 0)).1)
}

fn set_image(_this: JsValue, args: js_sys::Array) -> JsValue {
    let image = js::arg_str(&args, 0);
    if image == "headless" || image == "headed" { js::set_str(&mjs::store_ref(STORE), "image", &image); }
    JsValue::UNDEFINED
}

/// One schedule preset chip: sets the cron field. Highlights when the field
/// matches, reusing the agent editor's mode chips rather than growing a
/// second chip style.
fn cron_preset(label: &str, cron: &str) -> Node {
    m("button")
        .class("ae-mode")
        .attr("type", "button")
        .bind("class", &format!("({ST}.cron === '{cron}' ? 'is-on' : '')"))
        .on("click", &format!("{ST}.setCron('{cron}')"))
        .child(m("span").child(lit(label)))
}

/// The Custom chip: lit whenever the cron is hand-typed (non-empty and no
/// preset matches); clicking it drops the cursor into the cron field.
fn cron_custom() -> Node {
    m("button")
        .class("ae-mode")
        .attr("type", "button")
        .bind(
            "class",
            &format!(
                "({ST}.cron !== '' && {ST}.cron !== '0 * * * *' && {ST}.cron !== '0 9 * * *' \
                 && {ST}.cron !== '0 9 * * 1-5' && {ST}.cron !== '0 9 * * 1' && {ST}.cron !== '0 17 * * 5' \
                 ? 'is-on' : '')"
            ),
        )
        .on("click", &format!("{ST}.setCron('__custom')"))
        .child(m("span").child(lit("Custom")))
}

fn set_cron(_this: JsValue, args: js_sys::Array) -> JsValue {
    let cron = js::arg_str(&args, 0);
    if cron == "__custom" {
        let document = js::get(&js::global(), "document");
        let input = js::call(&document, "querySelector", &[JsValue::from_str(".tform-cron")]);
        if !input.is_undefined() && !input.is_null() {
            js::call(&input, "focus", &[]);
        }
        return JsValue::UNDEFINED;
    }
    js::set_str(&mjs::store_ref(STORE), "cron", &cron);
    JsValue::UNDEFINED
}

/// Update only the status, leaving a user's in-progress name edit untouched.
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

/// Switch a successful create into the same editor used by the picker.
pub(crate) fn sandbox_created(id: &str, state: &str, name: &str, host: &str) {
    open_with("sandbox", "edit", host, id);
    let form = mjs::store_ref(STORE);
    if js::get_str(&form, "name") == id && !name.is_empty() { js::set_str(&form, "name", name); }
    if !state.is_empty() { js::set_str(&form, "status", state); }
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
        .str("status", "Unknown")
        .str("cron", "")
        .str("prompt", "")
        .str("agent", "")
        .str("model", "")
        .str("timeout", "60")
        .bool("enabled", true)
        .bool("catchUp", false)
        .num("requestId", 0.0)
        .bool("network", true)
        .bool("create", true)
        .bool("busy", false)
        .bool("confirming", false)
        .method("statusIcon", status_icon)
        .method("statusClass", status_class)
        .method("setImage", set_image)
        .method("setCron", set_cron)
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

//! `session editor` — the panel the presence cluster's `+` opens.
//!
//! Not a dialog. The third column already means "whatever is selected
//! elsewhere", so a modal for the same idea would be a second paradigm for one
//! gesture. Clicking `+` in the roster puts this in the inspector, beside the
//! agent editor and the target form.
//!
//! ## It is one form, not an invite box
//!
//! The thing you are doing when you add a participant is editing the
//! conversation. Renaming it belongs in the same place, because a conversation
//! you have just opened up to three agents is exactly the moment its title
//! stops fitting.
//!
//! Five fields, and every one of them is already accepted by
//! `POST /api/sessions/:id/settings` (`serve.rs`), which whitelists
//! `purpose`, `notes`, `routing`, `memoryHigh` and `title`. `conversation.title`
//! already reaches the client too — the tab strip binds its tooltip to
//! `s.title` and displays only the ordinal. So this panel is the missing UI
//! for a route that is already finished.
//!
//! ## `role` is not what the reference shows
//!
//! `participants.role` exists and is unused by the UI, but it is a *routing*
//! role: `lead` picks who runs the routing call (`harness.rs`). It is not a job
//! title. A job title written into it would silently change who routes a group
//! round, so there is no "Role" field here.
//!
//! ## The chrome is the agent editor's
//!
//! `.agent-editor`, `.agent-editor-head`, `.ae-field`, `.ae-label`, `.ae-mode`,
//! `.ae-btn` — the inspector's form idiom, reused rather than redrawn, exactly
//! as `target_form` does. This file owns only what the editor has no equivalent
//! of: the participants block and the invite row.

use wasm_bindgen::JsValue;

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::components::{picker, presence, validation_info};
use crate::css;

const STORE: &str = "sessionDraft";
const ST: &str = "$store.sessionDraft";

/// The agents this project defines, so an unknown name is answered locally
/// rather than by seating a reference to something that does not exist — the
/// same check `/invite @name` makes.
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
/* The participants block and the invite row: the only parts of this panel the
   agent editor has no equivalent of. */
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
/* The search glyph inside the field, so the row says "type a name" before you
   type one. */
:is(#sb, #home) .se-invite-field { position: relative; flex: 1 1 auto; min-width: 0; }
:is(#sb, #home) .se-invite-field > i {
  position: absolute; left: 9px; top: 50%; transform: translateY(-50%);
  font-size: 13px; color: var(--gray-dim); pointer-events: none;
}
:is(#sb, #home) .se-invite-field input { padding-left: 30px; }

/* The Worktree section: its own label + hint above the repo picker, the
   two pattern inputs stacked beside the same fields as the rest of the
   form, a note carrying the resolved path, and the action that removes
   the checkout. */
:is(#sb, #home) .se-worktree { display: flex; flex-direction: column; gap: 8px; }
/* The repo picker reads like a text field: full width, same pill shape. */
:is(#sb, #home) .se-worktree .picker { display: flex; width: 100%; }
:is(#sb, #home) .se-worktree .dropdown-btn {
  flex: 1 1 auto; box-sizing: border-box; width: 100%;
  background: var(--panel-2); color: var(--fg);
  border: 1px solid transparent; border-radius: 8px;
  padding: 7px 9px; font-size: 13px; line-height: 1.5;
}
:is(#sb, #home) .se-worktree .dropdown-btn:hover { background: var(--panel-2); border-color: transparent; }
:is(#sb, #home) .se-worktree .dropdown-value { flex: 1 1 auto; color: var(--fg); }
/* The menu opens upward from the bottom of a narrow column, so it hangs
   off the right edge anchored left — anchor it right, like the deleted
   composer row's menus did at the composer's right edge. */
:is(#sb, #home) .se-worktree .picker .picker-menu,
:is(#sb, #home) .se-project .picker .picker-menu { left: auto; right: 0; }
/* The same for the Project picker, which shares the form. */
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

/// One labelled row, in the inspector's idiom. `rows` 0 is an input, any other
/// number a textarea of that height.
fn field(label: &str, hint: &str, model: &str, placeholder: &str, rows: usize) -> Node {
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
    m("div").class("ae-field").kids(vec![
        m("label").class("ae-label").kids(vec![
            m("span").child(lit(label)),
            m("span").class("hint").child(lit(hint)),
        ]),
        input,
    ])
}

/// The two-way routing switch, in the agent editor's `.ae-mode` shape.
fn mode(value: &str, label: &str, icon: &str) -> Node {
    m("button")
        .class("ae-mode")
        .attr("type", "button")
        .bind("class", &format!("({ST}.routing === '{value}' ? 'is-on' : '')"))
        .on("click", &format!("{ST}.setRouting('{value}')"))
        .kids(vec![m("i").class(&format!("ph {icon}")), m("span").child(lit(label))])
}

/// Who is seated, and the row that adds one.
///
/// The cluster is `presence`'s markup — the same `seat()` the guide features
/// alone at `/storybook/presence`. The editor shows **every** seat: this panel
/// is where the truncated ones live, so it is the thing `+N` opens. Removal
/// happens here too, but through the dialog's own control — never a button on
/// a seat, which would break the stack.
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
                    // Enter invites: this is one word, and reaching for the
                    // mouse to commit a single word is friction.
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

/// The project block: the existing `picker::project` brick, wrapped in the
/// same `ae-field` idiom as the rest of the form. It persists through the
/// app's own `pickTarget('project', …)` — one brick, one behavior, mounted
/// here by the page that passes the live store.
fn project_block(st: &str) -> Node {
    m("div").class("ae-field se-project").kids(vec![
        m("label").class("ae-label").kids(vec![
            m("span").child(lit("Project")),
            m("span").class("hint").child(lit("the folder this conversation is filed under")),
        ]),
        picker::project(st),
    ])
}

/// The worktree section: the repo the checkout is provisioned from, the two
/// patterns that name it, where it lands, and the action that takes it away.
/// The resolved-path readout reads the session's live record — the server
/// re-resolves it on save, so the path shown is the path the gate checks.
fn worktree_section(st: &str) -> Node {
    let readout = format!(
        "({st}.session.worktree && {st}.session.worktree.provisioned ? {st}.session.worktree.path : 'will be provisioned on save')"
    );
    m("div").class("se-worktree").kids(vec![
        m("label").class("ae-label").kids(vec![
            m("span").child(lit("Worktree")),
            m("span").class("hint").child(lit("provisioned from a repo")),
        ]),
        picker::repo(st),
        field(
            "Branch pattern",
            "the branch to check out; {{hex}} draws a fresh suffix",
            &format!("{ST}.worktreeBranchPattern"),
            "chat/{{hex}}",
            0,
        ),
        field(
            "Path pattern",
            "where the checkout lands",
            &format!("{ST}.worktreePathPattern"),
            "{{parent}}/{{repo}}/{{prefix}}-{{hash}}",
            0,
        ),
        m("p").class("se-worktree-path").text(&readout),
        m("div").class("se-worktree-actions").kids(vec![
            m("button")
                .class("ae-btn")
                .attr("type", "button")
                .show(&format!("({st}.session.worktree && {st}.session.worktree.provisioned)"))
                .on("click", &format!("{ST}.removeWorktree()"))
                .kids(vec![
                    Node::icon("trash"),
                    m("span").child(lit("Remove worktree")),
                ]),
        ]),
    ])
}

/// The panel. `st` is the app store behind the form — `$store.live` in the
/// app, `$store.samples` in the guide — threaded to the pickers that read
/// the app's target lists, so the same builder serves both pages.
pub fn panel(st: &str) -> Node {
    m("div")
        .class("agent-editor session-editor")
        .show(&format!("{ST}.open"))
        .kids(vec![
        m("div").class("agent-editor-head").kids(vec![
            Node::icon("pencil-simple"),
            m("span").class("agent-editor-title").child(lit("Edit session")),
        ]),
        // The verdict banner: always at the top of the form, just above the
        // first field — one brick, every aside form.
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
        m("div").class("ae-field").kids(vec![
            m("label").class("ae-label").kids(vec![
                m("span").child(lit("Routing")),
                m("span")
                    .class("hint")
                    .child(lit("who takes a turn in a group round")),
            ]),
            m("div").class("ae-modes").kids(vec![
                mode("everyone", "Everyone", "ph-chats-circle"),
                mode("lead", "Lead", "ph-megaphone"),
            ]),
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
                .attr("placeholder", "512")
                .model(&format!("{ST}.memoryHigh")),
        ]),
        project_block(st),
        field(
            "Workspace root",
            "absolute path — the session's file/shell root on the host",
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
            // Who else can reach this conversation (PLAN44 C4): the share
            // dialog owns the grants; this only opens it.
            m("button")
                .class("ae-btn")
                .attr("type", "button")
                .on("click", "$store.share.openForActive()")
                .kids(vec![
                    Node::icon("users-three").class("ph ph-users-three"),
                    m("span").child(lit("Share…")),
                ]),
            m("span").class("spacer"),
            // Two presses rather than a dialog: the destructive press is a
            // different press from the one that reveals it — same idiom as
            // the agent editor's remove and the target form. A session is a
            // conversation with a history; one stray click should not be the
            // whole cost of a typo.
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

/// The guide's mount: the panel with the guide's store behind it.
pub fn demo() -> Node {
    // The panel hides on its own `open` flag in the inspector; the guide
    // turns it on at mount and seeds the draft from the guide's sample
    // session, so every section — Project, Workspace root, Worktree —
    // shows what the app's form shows with data in it.
    m("div")
        .class("se-editor-demo")
        .init("$store.sessionDraft.open = true; $store.sessionDraft.seed($store.samples.session, 'samples')")
        .child(panel(crate::components::samples::EXPR))
}

fn say(s: &JsValue, tone: &str, message: &str) {
    js::set_str(s, "messageTone", tone);
    // Through the brick: the message fades in a minute, re-armed by each
    // new one.
    crate::components::validation_info::note(STORE, "message", message);
}

/// `invite()` — seat a name.
///
/// The same three answers `/invite @name` gives, because it *is* the same act:
/// a name is checked against the defined agents before anything is seated, and
/// an already-seated name is re-seated rather than duplicated.
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
            // Already in the room. Re-seat rather than add a second seat with
            // the same name, which would draw the same agent twice.
            js::set_bool(&p, "active", true);
            js::set_str(&s, "inviteHandle", "");
            say(&s, "ok", &format!("{handle} is already here"));
            mjs::redraw();
            return JsValue::UNDEFINED;
        }
    }

    // `max + 1`, never `length + 1`: a removed seat must not re-collide with a
    // surviving one and give x-for a duplicate key.
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

/// What the routing choice means, in one line, because "lead" alone does not
/// say what will happen.
fn routing_note(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let note = if js::get_str(&s, "routing") == "lead" {
        "one agent decides who speaks each round"
    } else {
        "every seated agent takes a turn"
    };
    JsValue::from_str(note)
}

/// `save()` — persist the five settings fields, then refresh.
///
/// Only the settings travel: the seats are draft-local until the participants
/// path is wired, which is why invite/unseat answer locally while save talks
/// to the server. Mirrors `live::rename_tree_row`: write the meta, refresh
/// the session, reload the list so the tab strip follows the new title.
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
    js::set_str(&payload, "routing", &js::get_str(&s, "routing"));
    // `x-model` on a number input stores a number or null, never a string —
    // sending the box's "" would store text in an INTEGER column.
    let mh = js::get(&s, "memoryHigh");
    if mh.is_null() || mh.is_undefined() || mh.as_f64().is_none() {
        js::set(&payload, "memoryHigh", &JsValue::NULL);
    } else {
        js::set(&payload, "memoryHigh", &mh);
    }
    // The session's two properties: the host file/shell root, and the
    // worktree CONFIG the server provisions on save (its answer carries the
    // resolved branch/path — the readout reads that back off the session).
    js::set_str(&payload, "workspaceRoot", &js::get_str(&s, "workspaceRoot").trim().to_string());
    let repo = js::get_str(&s, "worktreeRepoId").trim().to_string();
    if !repo.is_empty() {
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
            // A 4xx with `{ok:false, error}` still reaches this callback —
            // provisioning refusals (a path taken, a bad ref) must land in
            // the banner, not in a silent nothing.
            if js::get(&v, "ok").as_bool() == Some(false) {
                say(&s, "error", &js::get_str(&v, "error"));
                mjs::redraw();
                return;
            }
            if js::get_str(&live, "activeId") == id {
                js::set(&live, "session", &js::get(&v, "session"));
            }
            // New title, new tab strip: the tooltip reads the reloaded list.
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
    // Disarm: closing the panel is "I'm not deleting", whatever state it
    // was in — same rule `target_form.close` follows for `confirming`.
    js::set_bool(&s, "confirmDelete", false);
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `delete()` — the trash button in the form, two presses rather than a
/// dialog: the first arms it and says so, the second does it (see the
/// agent editor and the target form, where a folder holding conversations
/// is not a thing to lose to one click).
///
/// The target is `$store.live.activeId` — the session the draft was seeded
/// from. `live.deleteSession` removes it from the list immediately, re-syncs
/// with the server, and clears `activeId` if it was the one looked at; the
/// form then closes, since the thing it edited is gone.
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

/// `seed(meta, from)` — fill the draft from a session meta object.
///
/// One seeding implementation for both pages: the app calls it from
/// `live::open_session_editor` (which does the orchestration around it —
/// seats, closing sibling panels, opening the aside), the guide calls it at
/// mount. `from` names the store the meta belongs to (`live`/`samples`):
/// the title fallback chain reads that store's session list, because a
/// nameless session still shows the server's display title on its row.
fn seed(_this: JsValue, args: js_sys::Array) -> JsValue {
    let meta = args.get(0);
    let from = args.get(1).as_string().unwrap_or_else(|| "live".to_string());
    let s = mjs::store_ref(STORE);
    // `title` is seeded on its own: sessions made without a name carry no
    // stored one, and the tree/tab bar then show the server's display
    // fallback (`title || agent || id`). The form should match the name the
    // user sees on the row, not an empty box — and typing a real name in
    // `save()` persists it, which is exactly what renaming is.
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
    let routing = js::get_str(&meta, "routing");
    js::set_str(&s, "routing", if routing.is_empty() { "everyone" } else { &routing });
    // Numbers stay numbers: the meta's NULL/number goes straight through,
    // the same shape `save` sends back.
    let mh = js::get(&meta, "memoryHigh");
    if mh.is_null() || mh.is_undefined() || mh.as_f64().is_none() {
        js::set(&s, "memoryHigh", &JsValue::NULL);
    } else {
        js::set(&s, "memoryHigh", &mh);
    }
    // The session properties: workspace root, and the worktree config the
    // server resolves (the record's patterns are what re-save keeps stable —
    // a fresh {{hex}} is drawn only when these change).
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

/// The repo picker's pick: write the draft's repo and close the menu —
/// the same shape every picker's `on_pick` has, but landing on the form's
/// own field instead of the composer's.
fn pick_repo(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let id = args.get(0).as_string().unwrap_or_default();
    js::set_str(&s, "worktreeRepoId", &id);
    js::call(&mjs::store_ref("dropdown"), "closeIf", &[JsValue::from_str("repo")]);
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `removeWorktree()` — take the session's checkout away deliberately.
/// `worktree: null` is the settings body's "no worktree" spelling: the
/// server reaps the session's worktree(s) and answers the cleared meta.
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
        // The last one starts unseated, so the dimmed disc is on screen without
        // having to click first.
        js::set_bool(&p, "active", i + 1 < KNOWN.len());
        seats.push(&p);
    }
    let mut d = Def::new().named("store sessionDraft");
    d.bool("open", false)
        .bool("confirmDelete", false)
        .str("title", "iran war status")
        .str("purpose", "")
        .str("notes", "")
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

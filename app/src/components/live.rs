//! `live` — the store that binds the home route to angela.
//!
//! One store feeds all three columns, so they cannot disagree about which
//! session is open:
//!
//! | column | reads |
//! |---|---|
//! | project tree | `projects` + `sessions` |
//! | tab bar | `sessions` + `activeId` (one tab per session, per the design) |
//! | session panel | `messages`, plus the live `streamText` / `streamReasoning` |
//! | composer | `draft`, `waiting` |
//!
//! Streaming rows land here as they arrive. Per the m-js guidance on
//! high-frequency updates, a token delta appends to one store string rather
//! than rebuilding the message list — the list is only rewritten at turn
//! boundaries, which is a low-frequency change the diff handles happily.

use wasm_bindgen::JsValue;

use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::api;
use crate::helpers;

pub const STORE: &str = "live";

/// Pull the session list; refresh the rail and the tab bar.
fn load_sessions(_this: JsValue, _args: js_sys::Array) -> JsValue {
    api::sessions(apply_sessions);
    JsValue::UNDEFINED
}

pub(crate) fn apply_sessions(v: JsValue) {
        let s = mjs::store_ref(STORE);
        let rows = js::get(&v, "sessions");
        if !js_sys::Array::is_array(&rows) { return; }
        crate::components::notification_toggle::reconcile_unread(&s, &js_sys::Array::from(&rows));
        if same_json(&js::get(&s, "sessions"), &js::get(&v, "sessions")) && js::get_str(&s, "projectRoot") == js::get_str(&v, "projectRoot") { return; }
        js::set(&s, "sessions", &js::get(&v, "sessions"));
        js::set_str(&s, "projectRoot", &js::get_str(&v, "projectRoot"));
        rebuild_tabs(&s);
        // Re-open whatever was on the strip last time. Nothing is adopted
        // automatically: with no open tabs the column is deliberately bare.
        if js::get_str(&s, "activeId").is_empty() {
            let tabs = js_sys::Array::from(&js::get(&s, "tabs"));
            if tabs.length() > 0 {
                let remembered = read_active_id();
                let id = tabs.iter().find(|tab| js::get_str(tab, "id") == remembered)
                    .map(|tab| js::get_str(&tab, "id"))
                    .unwrap_or_else(|| js::get_str(&tabs.get(0), "id"));
                set_active(&s, &id);
                open_session_by_id(&id);
            }
        }
    }

/// Load one session's transcript into `messages`.
fn open_session_by_id(id: &str) {
    SESSION_LOADING.with(|v| *v.borrow_mut() = id.to_string());
    apply_cached_approvals();
    let want = id.to_string();
    api::session(id, move |v| {
        let s = mjs::store_ref(STORE);
        // A slow response for a session the user has since navigated away
        // from must not clobber the one they are now looking at.
        if js::get_str(&s, "activeId") != want {
            return;
        }
        let msgs = js_sys::Array::from(&js::get(&v, "messages"));
        for m in msgs.iter() {
            add_html(&m);
        }
        js::set(&s, "messages", &msgs.clone().into());
        js::set(&s, "session", &js::get(&v, "session"));
        let meta = js::get(&v, "session");
        let model = js::get_str(&meta, "model");
        if !model.is_empty() {
            // The session's own model wins over the remembered pick: it is
            // what the next turn will actually run on.
            js::set_str(&s, "model", &model);
            js::set_str(&mjs::store_ref("dropdown"), "model", &model);
        }
        // Where this conversation works. Restored from the session rather
        // than left as whatever the last tab happened to show — these are
        // per-conversation settings. There is no session sandbox any more
        // (PLAN55 D2): an agent's container is per (session → agent) and
        // owned by the agents plane.
        for field in ["project", "worktreeId", "branchId"] {
            let store_field = if field == "project" { "projectId" } else { field };
            js::set_str(&s, store_field, &js::get_str(&meta, field));
        }
        let config = js::get(&meta, "config");
        let reasoning = js::get_str(&config, "reasoningEffort");
        js::set_str(&mjs::store_ref("dropdown"), "level", if reasoning.is_empty() { "medium" } else { &reasoning });
        // A newly created or older session has no saved picker values yet;
        // seed its SQLite metadata from the controls now shown for it.
        persist_composer_settings(JsValue::UNDEFINED, js_sys::Array::new());
        // Rebuild the session totals from the reloaded transcript, so
        // reopening a session shows what it has cost so far rather than
        // starting from zero.
        store_metrics(&s, &session_metrics(&msgs));
        // The editor holds only the session's additional rules.
        let text = js::get_str(&v, "allowlist");
        js::set_str(&s, "allowlist", &text);
        let overridden = js::get_bool(&v, "allowlistOverridden");
        js::set_bool(&s, "allowlistOverridden", overridden);
        // Restore the policy so switching sessions shows each one's own.
        let unrestricted = js::get_str(&v, "allowlistMode") == "unrestricted";
        js::set_bool(&s, "unrestricted", unrestricted);
        js::set_str(
            &mjs::store_ref("dropdown"),
            "policy",
            if unrestricted { "Unrestricted" } else { "Allowlist" },
        );
        js::set_str(&s, "allowlistBaseline", "");
        clear_stream(&s);
        // The roster and the cursor come back on the same response, so the
        // attribution map is ready before the first row renders.
        store_roster(&s, &js_sys::Array::from(&js::get(&v, "participants")));
        js::set_str(&s, "cursor", &js::get_str(&v, "cursor"));
        SESSION_LOADING.with(|v| v.borrow_mut().clear());
        let turn_done = replay_active_turn(&want);
        mjs::redraw();
        if turn_done {
            open_session_by_id(&want);
            return;
        }
        // The tree and the memory view are separate reads: neither is needed
        // to draw the transcript, and asking for both up front would make
        // opening a session wait on work most sessions never show.
        load_view(JsValue::UNDEFINED, js_sys::Array::new());
    });
}

/// Forget who was in the last conversation.
///
/// Every field `store_roster` writes, so nothing survives a switch: a roster
/// left over from the previous session is indistinguishable from this one's,
/// and it decides which turn path the composer thinks it is on.
fn clear_roster(s: &JsValue) {
    js::set(s, "participants", &js_sys::Array::new().into());
    js::set(s, "others", &js_sys::Array::new().into());
    js::set(s, "activeOthers", &js_sys::Array::new().into());
    js::set(s, "handles", &js::obj().into());
    js::set(s, "accents", &js::obj().into());
    js::set(s, "icons", &js::obj().into());
    js::set_num(s, "agentCount", 0.0);
    js::set_bool(s, "isGroup", false);
    js::set_str(s, "cursor", "");
}

fn clear_stream(s: &JsValue) {
    js::set_str(s, "streamText", "");
    js::set_str(s, "streamHtml", "");
    js::set_str(s, "streamReasoning", "");
    js::set_str(s, "streamReasoningHtml", "");
    // Each turn's reasoning starts shut, whatever the reader did with the
    // last one: leaving it open would make the next turn's reasoning the
    // biggest thing on screen without them asking for it again.
    js::set_bool(s, "streamReasoningOpen", false);
    js::set_str(s, "streamHandle", "");
}

/// `openSession(id)` — select an already open tab.
fn open_session(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    if js::get_str(&s, "activeId") != id { crate::components::composer::reset_history(&s); }
    js::set_bool(&s, "treeOpen", false);
    js::set_num(&s, "treeEscapeAt", 0.0);
    close_preview_on_leave(&s, &id);
    // The next session's memory view decides whether it has summaries.
    js::set_bool(&s, "compacted", false);
    js::set(&s, "threads", &js_sys::Array::new().into());
    // Likewise the roster: the fetch below replaces it, and until it lands
    // the old session's agents would be shown as this one's.
    clear_roster(&s);
    // Opening from the project tree puts it on the strip if it is not there.
    add_open_id(&id);
    rebuild_tabs(&s);
    set_active(&s, &id);
    js::set(&s, "messages", &js_sys::Array::new().into());
    clear_stream(&s);
    pin_to_bottom();
    mjs::redraw();
    // Viewing a session (tree preview or tab) is not the same as asking for
    // the Edit Session form: opening the inspector on every conversation switch
    // is noise, and the column is contextual, opening only for the explicit
    // selectors (pencil, agents, settings, target form).
    open_session_by_id(&id);
    JsValue::UNDEFINED
}

/// A project-tree selection opens one replaceable preview, unless already open.
fn preview_session(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    if id.is_empty() { return JsValue::UNDEFINED; }
    let ids = read_open_ids();
    if !ids.iter().any(|open| open == &id) {
        let (next, preview) = preview_tabs(ids, &read_preview_id(), &id);
        write_open_ids(&next);
        write_preview_id(&preview);
    }
    open_session(JsValue::UNDEFINED, args)
}

/// Double-clicking the preview keeps it alongside the other tabs.
fn pin_tab(_this: JsValue, args: js_sys::Array) -> JsValue {
    keep_tab(&js::arg_str(&args, 0));
    JsValue::UNDEFINED
}

/// Keep a tab: clear its preview status so it stops being replaceable.
/// Talking in a session counts as keeping it — sending from the composer
/// pins the tab it lands in, same as the double-click.
fn keep_tab(id: &str) {
    if !id.is_empty() && read_preview_id() == id {
        write_preview_id("");
        js::set_str(&mjs::store_ref(STORE), "previewId", "");
        mjs::redraw();
    }
}

/// `newSession()` — the tab bar's `+`.
fn new_session(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let payload = js::obj();
    api::new_session(&payload.into(), |v| {
        let s = mjs::store_ref(STORE);
        let id = js::get_str(&js::get(&v, "session"), "id");
        if !id.is_empty() {
            close_preview_on_leave(&s, &id);
            crate::components::composer::reset_history(&s);
            js::set_bool(&s, "treeOpen", false);
            add_open_id(&id);
            set_active(&s, &id);
            js::set_bool(&s, "compacted", false);
            js::set(&s, "threads", &js_sys::Array::new().into());
            js::set(&s, "messages", &js_sys::Array::new().into());
            clear_stream(&s);
            // A new conversation is empty of agents. Without this the
            // previous session's roster stayed on screen — and since
            // nothing re-read it, those agents looked seated in a
            // conversation that had never heard of them.
            clear_roster(&s);
            // …and then read the real one back, so the new session's state
            // comes from the server rather than from what we assumed.
            // Do not carry the previous session's editor into the new one.
            js::set_bool(&mjs::store_ref("sessionDraft"), "open", false);
            open_session_by_id(&id);
        }
        mjs::redraw();
        if !id.is_empty() {
            let expected = id.clone();
            let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
                if js::get_str(&mjs::store_ref(STORE), "activeId") != expected {
                    return;
                }
                let document = js::get(&js_sys::global(), "document");
                let input = js::call(&document, "querySelector", &[JsValue::from_str("#home .composer .draft-ta")]);
                if !js::is_missing(&input) {
                    js::call(&input, "focus", &[]);
                }
            });
            js::call(&js_sys::global(), "requestAnimationFrame", &[callback]);
        }
        load_sessions(JsValue::UNDEFINED, js_sys::Array::new());
    });
    JsValue::UNDEFINED
}

fn history_move(_this: JsValue, args: js_sys::Array) -> JsValue {
    crate::components::composer::history_move(&mjs::store_ref(STORE), args)
}

fn draft_input(_this: JsValue, args: js_sys::Array) -> JsValue {
    crate::components::composer::draft_input(&mjs::store_ref(STORE), args)
}

/// Open the hidden file picker (A5). The input does the choosing; this
/// button is just its visible face.
fn pick_image(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let doc = js::get(&js::global(), "document");
    let el = js::call(&doc, "querySelector", &[JsValue::from_str(".composer-image-input")]);
    if !el.is_null() && !el.is_undefined() {
        js::call(&el, "click", &[]);
    }
    JsValue::UNDEFINED
}

/// The picked file becomes a POST of its raw bytes; the answer's URL is the
/// pending attachment. Images only, enforced server-side too.
fn image_chosen(_this: JsValue, args: js_sys::Array) -> JsValue {
    let ev = js::arg(&args, 0);
    let target = js::get(&ev, "target");
    let files = js::get(&target, "files");
    let file = js_sys::Array::from(&files).get(0);
    if file.is_null() || file.is_undefined() {
        return JsValue::UNDEFINED;
    }
    let name = js::get_str(&file, "name");
    let session_id = js::get_str(&mjs::store_ref(STORE), "activeId");
    let global = js::global();
    let opts = js::obj();
    js::set_str(&opts, "method", "POST");
    js::set(&opts, "body", &file);
    let url = format!("/api/upload?name={}", name.replace(' ', "%20"));
    let done = wasm_bindgen::closure::Closure::once_into_js(move |resp: JsValue| {
        let parsed = wasm_bindgen::closure::Closure::once_into_js(move |v: JsValue| {
            let s = mjs::store_ref(STORE);
            let url = js::get_str(&v, "url");
            if js::get_str(&s, "activeId") == session_id {
                js::set_str(&s, "image", &url);
            } else {
                COMPOSER_DRAFTS.with(|drafts| drafts.borrow_mut().entry(session_id).or_default().image = url);
            }
            mjs::redraw();
        });
        let body = js::call(&resp, "json", &[]);
        js::call(&body, "then", &[parsed]);
    });
    let resp = js::call(&global, "fetch", &[JsValue::from_str(&url), opts.into()]);
    js::call(&resp, "then", &[done]);
    JsValue::UNDEFINED
}

fn drop_image(_this: JsValue, _args: js_sys::Array) -> JsValue {
    js::set_str(&mjs::store_ref(STORE), "image", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `send()` — the composer. Streams the turn into the store.
fn send(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let draft = js::get_str(&s, "draft");
    // The pending attachment rides along as markdown — one content field,
    // one renderer, one transcript (PLAN44 A5).
    let image = js::get_str(&s, "image");
    let mut content = draft.trim().to_string();
    if !image.is_empty() {
        if !content.is_empty() {
            content.push_str("\n\n");
        }
        content.push_str(&format!("![image]({image})"));
    }
    if content.is_empty() || js::get_bool(&s, "waiting") || owns_turn(&js::get_str(&s, "activeId")) {
        return JsValue::UNDEFINED;
    }
    // A session you are talking in is one you mean to keep: submitting
    // pins the preview tab exactly like the double-click does.
    keep_tab(&js::get_str(&s, "activeId"));
    // A slash command is an instruction to the app, not a message to the
    // conversation — it never reaches a model and never enters the
    // transcript, so it is handled before anything else happens.
    if content.starts_with('/') {
        run_command(&s, &content);
        if js::get_str(&s, "draft").is_empty() {
            crate::components::composer::reset_history(&s);
        }
        return JsValue::UNDEFINED;
    }

    let baseline = js_sys::Array::from(&js::get(&s, "messages"));
    let original_id = js::get_str(&s, "activeId");
    let turn = js::obj();
    js::set_str(&turn, "sessionId", &original_id);
    js::set(&turn, "baseline", &baseline.into());
    js::set_str(&turn, "prompt", &content);
    js::set(&turn, "rows", &js_sys::Array::new().into());
    let turn: JsValue = turn.into();
    OWN_TURNS.with(|t| t.borrow_mut().push(turn.clone()));

    // Echo the prompt immediately: the turn is already durable server-side,
    // and waiting for the round trip would make the UI feel dead.
    crate::components::composer::reset_history(&s);
    push_message(&s, "user", &content, "");
    js::set_str(&s, "draft", "");
    js::set_str(&s, "image", "");
    js::set_bool(&s, "waiting", true);
    js::set_bool(&s, "stopping", false);
    js::set_bool(&s, "stopUnconfirmed", false);
    js::set_bool(&s, "stopPending", false);
    js::set(&s, "approvals", &js_sys::Array::new().into());
    apply_cached_approvals();
    clear_stream(&s);
    // Sending is an explicit "show me the bottom", even if the reader had
    // scrolled up a moment ago.
    pin_to_bottom();
    mjs::redraw();

    let payload = js::obj();
    js::set_str(&payload, "tabId", "1");
    js::set_str(&payload, "content", &content);
    let active = js::get_str(&s, "activeId");
    if !active.is_empty() { set_active(&s, &active); }
    let model = js::get_str(&s, "model");
    if !model.is_empty() {
        js::set_str(&payload, "model", &model);
    }
    let reasoning = js::get_str(&mjs::store_ref("dropdown"), "level");
    if !reasoning.is_empty() {
        js::set_str(&payload, "reasoningEffort", &reasoning);
    }
    if !active.is_empty() {
        js::set_str(&payload, "sessionId", &active);
    }

    let row_turn = turn.clone();
    api::chat_stream(
        &payload.into(),
        move |row| on_owned_row(&row_turn, &row),
        move |err| {
            let s = mjs::store_ref(STORE);
            let id = js::get_str(&turn, "sessionId");
            OWN_TURNS.with(|t| t.borrow_mut().retain(|other| !js_sys::Object::is(other, &turn)));
            if js::get_str(&s, "activeId") != id {
                load_sessions(JsValue::UNDEFINED, js_sys::Array::new());
                return;
            }
            // A stop we asked for ends the stream by aborting it, and the
            // abort surfaces here as a failed request. It is not one: the
            // turn did exactly what it was told, and an error row saying
            // "The user aborted a request" would be the app reporting the
            // user's own button press back to them as a fault.
            let stopped = js::get_bool(&s, "stopping") || js::get_bool(&s, "stopUnconfirmed");
            if stopped && js::get_bool(&s, "waiting") {
                // The stream ended without a done row. Keep listening to the
                // event feed; a closed connection does not stop the server.
                js::set_bool(&s, "stopping", false);
                js::set_bool(&s, "stopUnconfirmed", true);
                mjs::redraw();
                return;
            }
            js::set_bool(&s, "waiting", false);
            js::set_bool(&s, "stopping", false);
            js::set_bool(&s, "stopPending", false);
            if let Some(e) = err {
                if !stopped {
                    push_message(&s, "error", &e, "");
                }
            }
            // Reconcile with the server's own record once the turn is done.
            let id = js::get_str(&s, "activeId");
            if !id.is_empty() {
                open_session_by_id(&id);
            }
            load_sessions(JsValue::UNDEFINED, js_sys::Array::new());
            mjs::redraw();
        },
    );
    JsValue::UNDEFINED
}

/// `stop()` — the composer's stop button.
///
/// The expensive thing to cancel is not this browser's request, it is the
/// request the **server** has open to the model provider: that is what is
/// still being generated and still being billed. So the first thing this
/// does is tell the server, which drops that request where it stands.
///
/// Keep the stream open for confirmation and the final persisted rows.
fn stop(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    // Nothing in flight, or already stopping: a second press is not a
    // second stop.
    if !js::get_bool(&s, "waiting") || js::get_bool(&s, "stopping") {
        return JsValue::UNDEFINED;
    }
    js::set_bool(&s, "stopping", true);
    js::set_bool(&s, "stopUnconfirmed", false);
    let attempt = js::get(&s, "stopAttempt").as_f64().unwrap_or(0.0) + 1.0;
    js::set_num(&s, "stopAttempt", attempt);
    let id = js::get_str(&s, "activeId");
    if id.is_empty() {
        // The server has not named the session yet, so there is nothing to
        // address this to. `on_row` sends it the moment the `session` row
        // arrives, which is the first thing the turn emits.
        js::set_bool(&s, "stopPending", true);
    } else {
        crate::api::abort(&id, |_| {});
    }
    mjs::redraw();

    // A timeout is not confirmation: keep the turn attached, offer a retry,
    // and never replay it as if the user had not pressed Stop.
    rustex1_kit::lifecycle::after(2500.0, move || {
        let s = mjs::store_ref(STORE);
        if !js::get_bool(&s, "waiting") || !js::get_bool(&s, "stopping")
            || js::get(&s, "stopAttempt").as_f64() != Some(attempt) {
            return;
        }
        js::set_bool(&s, "stopping", false);
        js::set_bool(&s, "stopUnconfirmed", true);
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

thread_local! {
    static SESSION_LOADING: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    static OWN_TURNS: std::cell::RefCell<Vec<JsValue>> = const { std::cell::RefCell::new(Vec::new()) };
    static COMPOSER_DRAFTS: std::cell::RefCell<std::collections::HashMap<String, ComposerDraft>> = std::cell::RefCell::new(std::collections::HashMap::new());
    static TURNS: std::cell::RefCell<std::collections::HashMap<String, JsValue>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Install the server's turn journal, which includes the transcript as it
/// stood before the prompt. It can be replayed after an in-flight GET lands.
pub(crate) fn receive_turn_snapshot(snapshot: JsValue) {
    let id = js::get_str(&snapshot, "sessionId");
    if id.is_empty() { return; }
    let turn_id = js::get_str(&snapshot, "turnId");
    for row in js_sys::Array::from(&js::get(&snapshot, "rows")).iter() {
        if js::get_str(&row, "type") == "done" {
            crate::components::notification_toggle::completed(&id, &turn_id, None);
        }
    }
    TURNS.with(|t| { t.borrow_mut().insert(id.clone(), snapshot); });
    let s = mjs::store_ref(STORE);
    if js::get_str(&s, "activeId") == id && !owns_turn(&id)
        && SESSION_LOADING.with(|v| *v.borrow() != id) {
        if replay_active_turn(&id) { open_session_by_id(&id); }
    }
}

pub(crate) fn receive_turn_event(event: JsValue) {
    let id = js::get_str(&event, "sessionId");
    let turn_id = js::get_str(&event, "turnId");
    let row = js::get(&event, "row");
    if js::get_str(&row, "type") == "done" {
        crate::components::notification_toggle::completed(&id, &turn_id, None);
    }
    let cached = TURNS.with(|t| {
        let map = t.borrow();
        let Some(turn) = map.get(&id) else { return false; };
        if js::get_str(turn, "turnId") != turn_id { return false; }
        let rows = js_sys::Array::from(&js::get(turn, "rows"));
        rows.push(&row);
        js::set(turn, "rows", &rows.into());
        true
    });
    if !cached { return; }
    let done = js::get_str(&row, "type") == "done";
    let active = js::get_str(&mjs::store_ref(STORE), "activeId") == id;
    let loading = SESSION_LOADING.with(|v| *v.borrow() == id);
    if active && !loading && !owns_turn(&id) {
        on_row(&row);
        if done {
            TURNS.with(|t| { t.borrow_mut().remove(&id); });
            open_session_by_id(&id);
        }
    } else if done && (!active || owns_turn(&id)) {
        TURNS.with(|t| { t.borrow_mut().remove(&id); });
    }
}

/// Returns true if replay included the end of a turn and a fresh transcript
/// should be loaded once more (the first read may have raced finalization).
fn replay_active_turn(id: &str) -> bool {
    let turn = OWN_TURNS.with(|t| t.borrow().iter().find(|turn| js::get_str(turn, "sessionId") == id).cloned())
        .or_else(|| TURNS.with(|t| t.borrow().get(id).cloned()));
    let Some(turn) = turn else { return false; };
    let s = mjs::store_ref(STORE);
    let baseline = js_sys::Array::from(&js::get(&turn, "baseline"));
    for row in baseline.iter() { add_html(&row); }
    js::set(&s, "messages", &baseline.into());
    clear_stream(&s);
    js::set_bool(&s, "waiting", true);
    let prompt = js::get_str(&turn, "prompt");
    if !prompt.is_empty() { push_message(&s, "user", &prompt, ""); }
    let mut done = false;
    for row in js_sys::Array::from(&js::get(&turn, "rows")).iter() {
        done |= js::get_str(&row, "type") == "done";
        on_row(&row);
    }
    if done { TURNS.with(|t| { t.borrow_mut().remove(id); }); }
    done
}

fn owns_turn(id: &str) -> bool {
    OWN_TURNS.with(|t| t.borrow().iter().any(|turn| js::get_str(turn, "sessionId") == id))
}

/// Record the POST stream independently of the currently selected session.
fn on_owned_row(turn: &JsValue, row: &JsValue) {
    let s = mjs::store_ref(STORE);
    let active = js::get_str(&s, "activeId");
    let kind = js::get_str(row, "type");
    let (id, was_active) = {
        let previous = js::get_str(turn, "sessionId");
        let was_active = active == previous;
        if kind == "session" {
            let id = js::get_str(row, "sessionId");
            if !id.is_empty() { js::set_str(turn, "sessionId", &id); }
        }
        let rows = js_sys::Array::from(&js::get(turn, "rows"));
        rows.push(row);
        js::set(turn, "rows", &rows.into());
        (js::get_str(turn, "sessionId"), was_active)
    };
    if kind == "session" && !id.is_empty() && !was_active {
        add_open_id(&id);
        rebuild_tabs(&s);
    }
    let loading = SESSION_LOADING.with(|v| *v.borrow() == id);
    if was_active && !loading { on_row(row); }
}

fn on_row(row: &JsValue) {
    let s = mjs::store_ref(STORE);
    match js::get_str(row, "type").as_str() {
        // The server names the session it actually used — adopt it, so a
        // first prompt on a fresh tab binds the tab to its new session.
        "session" => {
            let id = js::get_str(row, "sessionId");
            if !id.is_empty() {
                add_open_id(&id);
                set_active(&s, &id);
                js::set_bool(&s, "waiting", true);
                // A mention may have invited an agent as this turn began.
                load_roster(&id);
                // Stop was pressed before the server said which session it
                // was making. This row is the first moment the request can
                // be addressed, so it goes out now rather than being lost.
                if js::get_bool(&s, "stopPending") {
                    js::set_bool(&s, "stopPending", false);
                    crate::api::abort(&id, |_| {});
                }
            }
            mjs::redraw();
        }
        // High-frequency: append to one string, never rebuild the list.
        // Who the in-flight rows belong to, so they take that agent's colour
        // before the sealed card arrives to say so.
        "speaking" => {
            js::set_str(&s, "streamHandle", &js::get_str(row, "handle"));
            mjs::redraw();
        }
        "assistant_delta" => {
            let h = js::get_str(row, "handle");
            if !h.is_empty() {
                js::set_str(&s, "streamHandle", &h);
            }
            let next = js::get_str(&s, "streamText") + &js::get_str(row, "text");
            js::set_str(&s, "streamText", &next);
            // Re-render on each delta rather than in a template binding: a
            // binding re-evaluates on every redraw, so a 10 KB message would
            // be re-parsed on every frame instead of once per token. The
            // renderer is prefix-safe (see helpers::streaming_md_tests), so
            // rendering half-arrived markdown cannot emit broken markup.
            js::set_str(&s, "streamHtml", &helpers::markdown_to_html(&next));
            mjs::redraw();
        }
        "reasoning_delta" => {
            // Reasoning usually streams before any content does, so this is
            // where the live card learns whose colour to wear.
            let h = js::get_str(row, "handle");
            if !h.is_empty() {
                js::set_str(&s, "streamHandle", &h);
            }
            let next = js::get_str(&s, "streamReasoning") + &js::get_str(row, "text");
            js::set_str(&s, "streamReasoning", &next);
            // Same prefix-safe renderer as the answer: half-arrived markdown
            // renders as itself rather than leaking broken tags.
            js::set_str(
                &s,
                "streamReasoningHtml",
                &helpers::markdown_to_html(&next),
            );
            mjs::redraw();
        }
        "usage" => {
            let p = js::get(row, "prompt_tokens").as_f64().unwrap_or(0.0);
            let t = js::get(row, "total_tokens").as_f64().unwrap_or(0.0);
            js::set_num(&s, "promptTokens", p);
            js::set_num(&s, "totalTokens", t);
            mjs::redraw();
        }
        "card" => {
            let card = js::get(row, "card");
            let kind = js::get_str(&card, "kind");
            let text = js::get_str(&card, "text");
            match kind.as_str() {
                // The sealed bubble replaces the streamed one.
                // The sealed bubble replaces the streamed one.
                "assistant" => {
                    let streamed = js::get_str(&s, "streamText");
                    let final_text = if text.is_empty() { streamed } else { text.clone() };
                    // Fold this turn into the running totals. The rates come
                    // from this turn's telemetry; the counts accumulate.
                    add_turn_metrics(&s, &js::get(&card, "telemetry"), &final_text);
                    if !final_text.trim().is_empty() {
                        push_card(&s, &card, "assistant", Some(&final_text));
                    }
                    js::set_str(&s, "streamText", "");
                    js::set_str(&s, "streamHtml", "");
                }
                "reasoning" => {
                    if !text.trim().is_empty() {
                        push_card(&s, &card, "reasoning", None);
                    }
                    js::set_str(&s, "streamReasoning", "");
                    js::set_str(&s, "streamReasoningHtml", "");
                }
                // Everything the turn produced goes through one path, so a
                // streaming turn and a reloaded one draw the same rail.
                "tool_call" | "tool_result" | "system_prompt" | "agent_prompt"
                | "decision" => push_card(&s, &card, &kind, None),
                "queued" => push_card(&s, &card, "queued", None),
                "error" => push_card(&s, &card, "error", None),
                // The server explaining itself — e.g. that the conversation
                // has no agents in it yet.
                "system" => push_message(&s, "system", &text, ""),
                _ => {}
            }
            mjs::redraw();
        }
        "done" => {
            js::set_bool(&s, "waiting", false);
            js::set_bool(&s, "stopping", false);
            js::set_bool(&s, "stopUnconfirmed", false);
            js::set_bool(&s, "stopPending", false);
            // A new message changes the tree's leaf set, so a threaded view
            // on screen is now one message short.
            load_view(JsValue::UNDEFINED, js_sys::Array::new());
            mjs::redraw();
        }
        _ => {}
    }
}

/// Append a card from the wire, keeping every field its template reads.
///
/// `text_override` is for the assistant row, whose final text may be what was
/// streamed rather than what the sealed card carries.
fn push_card(s: &JsValue, card: &JsValue, kind: &str, text_override: Option<&str>) {
    let list = js_sys::Array::from(&js::get(s, "messages"));
    let m = js::obj();
    js::set_str(&m, "kind", kind);
    for key in ["text", "handle", "turn", "eventId", "name", "model", "error", "ts", "spec"] {
        let v = js::get_str(card, key);
        if !v.is_empty() {
            js::set_str(&m, key, &v);
        }
    }
    if let Some(t) = text_override {
        js::set_str(&m, "text", t);
    }
    for key in ["bytes", "historyTurns", "confidence", "round", "position"] {
        if let Some(n) = js::get(card, key).as_f64() {
            js::set_num(&m, key, n);
        }
    }
    // Structured fields the cards render directly.
    for key in ["verdict", "speak", "args", "ok", "denied", "telemetry"] {
        let v = js::get(card, key);
        if !v.is_undefined() {
            js::set(&m, key, &v);
        }
    }
    js::set_bool(&m, "visible", true);
    js::set_str(&m, "id", &{
        let e = js::get_str(card, "eventId");
        if e.is_empty() {
            format!("local-{}", list.length())
        } else {
            e
        }
    });
    add_html(&m.clone().into());
    attach_turn(&list, &m.clone().into(), &js::get_str(card, "turn"));
    let next = list.clone();
    next.push(&m.into());
    js::set(s, "messages", &next.into());
}

/// Join a live row to the run the server says it belongs to.
///
/// `turnLast` moves: it is set on each arrival and cleared from the row
/// before, so the rail's end keeps pace as a turn produces more. Doing this
/// on the client matters because a streaming turn must draw the same rail the
/// reloaded transcript will — and the reloaded one computes it server-side.
fn attach_turn(list: &js_sys::Array, m: &JsValue, turn: &str) {
    if turn.is_empty() {
        return;
    }
    let mut first = true;
    for prev in list.iter() {
        if js::get_str(&prev, "turn") == turn {
            first = false;
            js::set_bool(&prev, "turnLast", false);
        }
    }
    js::set_bool(m, "turnFirst", first);
    js::set_bool(m, "turnLast", true);
}

/// Append one message to the transcript.
fn push_message(s: &JsValue, kind: &str, text: &str, name: &str) {
    push_message_as(s, kind, text, name, "")
}

/// Append one message, attributed to a participant's handle.
fn push_message_as(s: &JsValue, kind: &str, text: &str, name: &str, handle: &str) {
    let list = js_sys::Array::from(&js::get(s, "messages"));
    let m = js::obj();
    js::set_str(&m, "kind", kind);
    js::set_str(&m, "text", text);
    if !name.is_empty() {
        js::set_str(&m, "name", name);
    }
    if !handle.is_empty() {
        js::set_str(&m, "handle", handle);
    }
    // A locally made row happened now, by definition. Without this the
    // echoed prompt shows no time until the transcript is re-read.
    js::set_str(
        &m,
        "ts",
        &js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default(),
    );
    // A client-made row needs an id for `:key`; the server's ids are event
    // ids, so prefix to guarantee no collision.
    js::set_str(&m, "id", &format!("local-{}", list.length()));
    add_html(&m.clone().into());
    js::set(&m, "visible", &JsValue::TRUE);
    let next = list.clone();
    next.push(&m.into());
    // Replace the array rather than mutating it: an in-place push may not
    // notify (RUST_M_JS §3).
    js::set(s, "messages", &next.into());
}

// ── the model picker ───────────────────────────────────────────────────────
//
// It lived in the legacy `chat` store, beside the demo page that is now gone.
// It belongs here: the model is a composer setting, and the composer's other
// settings (reasoning level, tool policy) already persist through this store.

const MODEL_KEY: &str = "mjs-chat.model";
const DEFAULT_MODEL: &str = "llama-server:default";

/// The last model chosen, from localStorage — a per-browser convenience, so
/// the picker is never an empty box on first paint. The session's own model
/// is the server's business and arrives with the session.
fn saved_model() -> String {
    let ls = js::get(&js::global(), "localStorage");
    if !ls.is_undefined() && !ls.is_null() {
        if let Some(v) = js::call(&ls, "getItem", &[JsValue::from_str(MODEL_KEY)]).as_string() {
            if !v.trim().is_empty() {
                return v;
            }
        }
    }
    DEFAULT_MODEL.to_string()
}

fn save_model(m: &str) {
    if m.trim().is_empty() {
        return;
    }
    let ls = js::get(&js::global(), "localStorage");
    if ls.is_undefined() || ls.is_null() {
        return;
    }
    js::call(&ls, "setItem", &[JsValue::from_str(MODEL_KEY), JsValue::from_str(m)]);
}

fn default_models() -> JsValue {
    let a = js_sys::Array::new();
    a.push(&JsValue::from_str(DEFAULT_MODEL));
    a.into()
}

/// `setModel(id)` — pick one, remember it, tell the session.
fn set_model(_this: JsValue, args: js_sys::Array) -> JsValue {
    let v = js::arg_str(&args, 0);
    if v.is_empty() {
        return JsValue::UNDEFINED;
    }
    js::set_str(&mjs::store_ref(STORE), "model", &v);
    js::set_str(&mjs::store_ref("dropdown"), "model", &v);
    save_model(&v);
    persist_composer_settings(JsValue::UNDEFINED, js_sys::Array::new());
    mjs::redraw();
    JsValue::UNDEFINED
}

/// Fill the model list from agl's own config, and settle on one.
pub fn load_models() {
    crate::api::models(apply_models);
}

fn refresh_models(_this: JsValue, _args: js_sys::Array) -> JsValue {
    load_models();
    JsValue::UNDEFINED
}

fn same_json(a: &JsValue, b: &JsValue) -> bool {
    js_sys::JSON::stringify(a).ok() == js_sys::JSON::stringify(b).ok()
}

pub(crate) fn apply_models(v: JsValue) {
        let s = mjs::store_ref(STORE);
        let models = js::get(&v, "models");
        if models.is_undefined() || models.is_null() {
            return;
        }
        let changed = !same_json(&js::get(&s, "models"), &models);
        if changed { js::set(&s, "models", &models); }
        // Keep the current pick only if the server still offers it; otherwise
        // take the configured default, so the box never names a model that
        // would be refused.
        let current = js::get_str(&s, "model");
        let arr = js_sys::Array::from(&models);
        let valid = arr.iter().any(|x| x.as_string().as_deref() == Some(&current));
        if current.is_empty() || !valid {
            let configured = js::get_str(&v, "default_model");
            let pick = if configured.is_empty() { DEFAULT_MODEL.to_string() } else { configured };
            js::set_str(&s, "model", &pick);
            js::set_str(&mjs::store_ref("dropdown"), "model", &pick);
            save_model(&pick);
        }
    }

// ── approvals ──────────────────────────────────────────────────────────────
//
// A tool the allowlist does not permit stops the turn on the server and asks.
// The question cannot come down the turn's own stream — the thread writing
// that stream is the one blocked on the answer — the server pushes changes
// through the shared event stream.

thread_local! {
    static PENDING: std::cell::RefCell<JsValue> = const { std::cell::RefCell::new(JsValue::UNDEFINED) };
    static PENDING_KEY: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

pub(crate) fn apply_pending(v: JsValue) {
    PENDING.with(|p| *p.borrow_mut() = v);
    apply_cached_approvals();
}

fn apply_cached_approvals() {
    let s = mjs::store_ref(STORE);
    let id = js::get_str(&s, "activeId");
    let rows = PENDING.with(|p| js::get(&p.borrow(), &id));
    let rows = if rows.is_undefined() || rows.is_null() { js_sys::Array::new().into() } else { rows };
    let key = format!("{}:{}", id, js_sys::JSON::stringify(&rows).ok().and_then(|x| x.as_string()).unwrap_or_default());
    if PENDING_KEY.with(|p| *p.borrow() == key) { return; }
    PENDING_KEY.with(|p| *p.borrow_mut() = key);
    // Copy before adding UI fields; the cached wire snapshot stays unchanged.
    let copy = js_sys::JSON::stringify(&rows).ok().and_then(|v| v.as_string()).and_then(|v| js_sys::JSON::parse(&v).ok()).unwrap_or(rows);
    let arr = js_sys::Array::from(&copy);
    for r in arr.iter() {
        js::set_bool(&r, "expanded", true);
        js::set_bool(&r, "turn", true);
        js::set_str(&r, "handle", &js::get_str(&s, "streamHandle"));
    }
    if arr.length() != 0 || js_sys::Array::from(&js::get(&s, "approvals")).length() != 0 {
        js::set(&s, "approvals", &arr.into());
    }
}

/// `decideApproval(id, verdict)` — answer, and take the card off the screen.
///
/// The row is dropped here rather than waiting for the server: the turn
/// resumes the moment the answer lands, and the transcript will carry the
/// answered question as a real row a moment later. Leaving the live card up
/// until then would show it twice.
fn decide_approval(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    let verdict = js::arg_str(&args, 1);
    let s = mjs::store_ref(STORE);
    let kept = js_sys::Array::new();
    let rows = js_sys::Array::from(&js::get(&s, "approvals"));
    for i in 0..rows.length() {
        let r = rows.get(i);
        if js::get_str(&r, "id") != id {
            kept.push(&r);
        }
    }
    js::set(&s, "approvals", &kept.into());
    mjs::redraw();
    crate::api::approve(&id, &verdict, |_| {});
    JsValue::UNDEFINED
}

pub fn register() {
    let mut d = Def::new().named("store live");
    d.val("sessions", &js_sys::Array::new().into())
        .str("model", &saved_model())
        .val("models", &default_models())
        // `sessions` is every session on disk (the project tree).
        // `tabs` is the subset currently open (the tab strip). Closing a tab
        // must not destroy a session, so these are deliberately distinct.
        .val("tabs", &js_sys::Array::new().into())
        .str("previewId", "")
        .val("messages", &js_sys::Array::new().into())
        .val("session", &js::obj().into())
        .str("activeId", "")
        .bool("notifyEnabled", crate::components::notification_toggle::enabled())
        .val("unreadSessions", &crate::components::notification_toggle::unread())
        // ── the composer's target row ─────────────────────────────────
        // Where this session works: which folder it is filed under, which
        // checkout gets mounted. Empty lists until the pickers are wired to
        // their services — the row renders, every menu shows its null answer
        // and its create action, and nothing claims an instance exists that
        // does not.
        //
        // `sandboxes` + `sandboxGroups` are the left-rail sandboxes menu's
        // data (PLAN55 D6): container rows with their serving (session,
        // agent), grouped per image — computed by `sandboxes_menu::group_rows`
        // whenever rows arrive.
        .val("sandboxes", &js_sys::Array::new().into())
        .val("sandboxGroups", &js_sys::Array::new().into())
        // Which image groups are expanded in the sandboxes menu (absent =
        // open) — collapse state, not data, so the recompute cannot wipe it.
        .val("sandboxOpen", &js::obj().into())
        .val("projects", &js_sys::Array::new().into())
        .val("worktrees", &js_sys::Array::new().into())
        .val("repoBranches", &js_sys::Array::new().into())
        .val("repos", &js_sys::Array::new().into())
        // The scheduled prompts. Same contract as the target lists: the
        // rail renders, the form reads, and every write answers fresh.
        .val("automations", &js_sys::Array::new().into())
        // Whether the sandbox service answered at all. An empty list and a
        // dead service look identical in a menu; this is what tells them
        // apart.
        .bool("sboxUp", false)
        .str("projectId", "")
        .str("worktreeId", "")
        .str("branchId", "")
        .str("draft", "")
        // A pending attachment (PLAN44 A5): the uploaded image's URL, sent
        // as the message's `![image](…)` line.
        .str("image", "")
        .num("historyPosition", -1.0)
        .str("historyDraft", "")
        .str("streamText", "")
        .str("streamHtml", "")
        .str("streamReasoningHtml", "")
        // Reasoning opens on a click and starts shut, including the one
        // streaming right now — it is how the answer was reached, not the
        // answer, so it should not be the biggest thing on screen.
        .bool("streamReasoningOpen", false)
        // Who is currently speaking, so the in-flight rows take their colour.
        .str("streamHandle", "")
        .str("streamReasoning", "")
        .str("projectRoot", "")
        .str("allowlist", "")
        .str("allowlistBaseline", "")
        .bool("allowlistOpen", false)
        .bool("allowlistOverridden", false)
        // True while the session runs with `*`. The rules survive it.
        .bool("unrestricted", false)
        .bool("waiting", false)
        .val("approvals", &js_sys::Array::new().into())
        // Sticky scroll: true while the reader is at (or near) the bottom.
        .bool("stickBottom", true)
        .num("promptTokens", 0.0)
        .num("totalTokens", 0.0)
        // Metrics for the most recent turn that reported any, rendered
        // under the composer. Held as pre-built HTML for the same reason
        // the markdown is: a binding would rebuild it on every redraw.
        .str("lastMetrics", "")
        // Running session totals behind the rendered row.
        .num("mTps", 0.0)
        .num("mDur", 0.0)
        .num("mIn", 0.0)
        .num("mOut", 0.0)
        .num("mBytes", 0.0)
        // --- the memory tree ------------------------------------------------
        // `threads` is a FLAT list that renders nested (each row carries its
        // own `depth`): m-js diffs a keyed list far more cheaply than a
        // recursive tree, and the nesting here is purely presentational.
        .val("threads", &js_sys::Array::new().into())
        // True once something has actually been folded into a thread.
        .bool("compacted", false)
        // Show the threaded history instead of the flat transcript.
        .bool("compacting", false)
        // 0 = the server's default budget. Set it to force threading on a
        // conversation that would otherwise still fit.
        .num("memoryHigh", 0.0)
        // --- the roster -----------------------------------------------------
        .val("participants", &js_sys::Array::new().into())
        // The roster bar's list: everyone except the reader.
        .val("others", &js_sys::Array::new().into())
        .val("activeOthers", &js_sys::Array::new().into())
        // participant id -> handle, so a message can be attributed without
        // searching the roster on every row.
        .val("handles", &js::obj().into())
        // handle -> accent colour, for tinting a row to its author.
        .val("accents", &js::obj().into())
        // handle -> phosphor icon name, for drawing a byline as its author.
        .val("icons", &js::obj().into())
        // The reader's own name for the cluster's first disc. The server never
        // sends it (it sends the user *seat*, which the roster skips), so it
        // stays empty until something fills it - the template falls back to
        // 'You'.
        .str("userName", "")
        .num("agentCount", 0.0)
        .bool("isGroup", false)
        .bool("rosterOpen", false)
        // The stop press, between asking and the stream actually ending.
        // Separate from `waiting` because the composer must not accept a new
        // message yet — the old turn is still writing its last rows.
        .bool("stopping", false)
        .bool("stopUnconfirmed", false)
        .num("stopAttempt", 0.0)
        // A stop pressed before the server has even named the session it
        // made. There is nothing to address the stop to yet, so it is sent
        // the moment the `session` row arrives.
        .bool("stopPending", false)
        // --- branches -------------------------------------------------------
        .str("cursor", "")
        .bool("treeOpen", false)
        .bool("treeLoading", false)
        .num("treeEscapeAt", 0.0)
        .num("treeRequest", 0.0)
        .num("treeIndex", 0.0)
        .val("treeRows", &js_sys::Array::new().into())
        .method("loadSessions", load_sessions)
        .method("openSession", open_session)
        .method("toggleNotifications", crate::components::notification_toggle::toggle)
        .method("markSessionRead", crate::components::notification_toggle::mark_read)
        .method("previewSession", preview_session)
        .method("pinTab", pin_tab)
        .method("newSession", new_session)
        .method("closeTab", close_tab)
        .method("send", send)
        .method("draftInput", draft_input)
        .method("pickImage", pick_image)
        .method("imageChosen", image_chosen)
        .method("dropImage", drop_image)
        .method("historyMove", history_move)
        .method("statusParts", crate::components::system_card::status_parts)
        .method("turnEdges", crate::components::session::turn_edges)
        .method("streamEdges", crate::components::session::stream_edges)
        .method("relAgo", rel_ago)
        .method("onScroll", on_scroll)
        .method("keepPinned", keep_pinned)
        .method("deleteSession", delete_session)
        .method("toggleAllowlist", toggle_allowlist)
        .method("saveAllowlist", save_allowlist)
        .method("revertAllowlist", revert_allowlist)
        .method("setPolicy", set_policy)
        .method("setReasoning", set_reasoning)
        .method("persistComposerSettings", persist_composer_settings)
        // The target row. `pickTarget` records the choice locally; the
        // create/edit forms that these open are the next piece of work, so
        // they are named-but-idle rather than missing — an unresolved
        // method is a silent template failure, not a visible one.
        .method("loadProjects", load_projects)
        .method("loadTargets", load_targets)
        .method("loadAutomations", load_automations)
        .method("saveTarget", save_target)
        .method("dropSession", drop_session)
        .method("saveProject", save_project)
        .method("renameTreeRow", rename_tree_row)
        .method("pickTarget", pick_target)
        .method("editTarget", edit_target)
        .method("createTarget", create_target)
        .method("loadView", load_view)
        .method("compact", compact)
        .method("toggleThread", toggle_thread)
        .method("removeAgent", remove_agent)
        .method("openSessionEditor", open_session_editor)
        .method("treeEscape", tree_escape)
        .method("treeResetEscape", tree_reset_escape)
        .method("treeClose", tree_close)
        .method("treeSelect", tree_select)
        .method("treeMove", tree_move)
        .method("treeChoose", tree_choose)
        .method("treeChooseSelected", tree_choose_selected)
        .method("forkPrompt", fork_prompt)
        .method("handleOf", handle_of)
        .method("toggleCard", toggle_reasoning)
        .method("accentOf", accent_of)
        .method("iconOf", icon_of)
        .method("stop", stop)
        .method("decideApproval", decide_approval)
        .method("setModel", set_model)
        .method("refreshModels", refresh_models)
        .method("toggleSandboxGroup", crate::components::sandboxes_menu::toggle_group);
    mjs::store(STORE, &d.finish());
}

/// `toggleAllowlist()` — the shield button beside the composer.
fn toggle_allowlist(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let open = js::get_bool(&s, "allowlistOpen");
    js::set_bool(&s, "allowlistOpen", !open);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn persist_composer_settings(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let live = mjs::store_ref(STORE);
    let id = js::get_str(&live, "activeId");
    if id.is_empty() { return JsValue::UNDEFINED; }
    let dd = mjs::store_ref("dropdown");
    let payload = js::obj();
    js::set_str(&payload, "sessionId", &id);
    js::set_str(&payload, "model", &js::get_str(&live, "model"));
    js::set_str(&payload, "reasoningEffort", &js::get_str(&dd, "level"));
    rustex1_kit::api::post_json("/api/composer-settings", &payload.into(), |_| {});
    JsValue::UNDEFINED
}

fn set_reasoning(_this: JsValue, args: js_sys::Array) -> JsValue {
    js::set_str(&mjs::store_ref("dropdown"), "level", &js::arg_str(&args, 0));
    persist_composer_settings(JsValue::UNDEFINED, js_sys::Array::new());
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `saveAllowlist()` — on blur, as cowork does. Persisting per keystroke
/// would write a file on every character.
fn save_allowlist(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let text = js::get_str(&s, "allowlist");
    let baseline = js::get_str(&s, "allowlistBaseline");
    let overridden = text != baseline;
    js::set_bool(&s, "allowlistOverridden", overridden);

    let id = js::get_str(&s, "activeId");
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let payload = js::obj();
    js::set_str(&payload, "sessionId", &id);
    js::set_str(&payload, "allowlist", &text);
    js::set(&payload, "overridden", &JsValue::from_bool(overridden));
    rustex1_kit::api::post_json("/api/allowlist", &payload.into(), |_| {});
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `revertAllowlist()` — clear the session rules; agent rules still apply.
fn revert_allowlist(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let id = js::get_str(&s, "activeId");
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let payload = js::obj();
    js::set_str(&payload, "sessionId", &id);
    js::set_str(&payload, "allowlist", "");
    js::set(&payload, "overridden", &JsValue::FALSE);
    rustex1_kit::api::post_json("/api/allowlist", &payload.into(), move |_| {
        let s = mjs::store_ref(STORE);
        js::set_bool(&s, "allowlistOverridden", false);
        let id = js::get_str(&s, "activeId");
        if !id.is_empty() {
            open_session_by_id(&id);
        }
        mjs::redraw();
    });
    JsValue::UNDEFINED
}


/// Pre-render a message's markdown.
///
/// Done once when the message lands rather than in a template binding: a
/// bound expression is re-evaluated on every redraw, so rendering there would
/// re-parse the whole transcript on each streamed token.
fn add_html(msg: &JsValue) {
    let kind = js::get_str(msg, "kind");
    // Reasoning is prose from the same model and arrives with the same
    // markdown in it, so it gets the same treatment as an answer.
    if kind != "user" && kind != "assistant" && kind != "reasoning" {
        return;
    }
    let text = js::get_str(msg, "text");
    if text.is_empty() {
        return;
    }
    js::set_str(msg, "html", &helpers::markdown_to_html(&text));
}

/// Format a byline's elapsed time as prose, such as "24min ago".
pub(crate) fn rel_ago(_this: JsValue, args: js_sys::Array) -> JsValue {
    let iso = js::arg_str(&args, 0);
    if iso.is_empty() {
        return JsValue::from_str("");
    }
    let then = js_sys::Date::new(&JsValue::from_str(&iso)).get_time();
    if then.is_nan() {
        return JsValue::from_str("");
    }
    // Clamped at zero: a clock a second out of step should read "just now",
    // not "in the future".
    let secs = ((js_sys::Date::now() - then) / 1000.0).max(0.0);
    let out = if secs < 45.0 {
        "just now".to_string()
    } else if secs < 3600.0 {
        format!("{}min ago", (secs / 60.0).round().max(1.0) as i64)
    } else if secs < 86_400.0 {
        format!("{}hr ago", (secs / 3600.0).floor() as i64)
    } else {
        let d = (secs / 86_400.0).floor() as i64;
        format!("{d} day{} ago", if d == 1 { "" } else { "s" })
    };
    JsValue::from_str(&out)
}

/// `deleteSession(id)` — the trash button. Removes it from persistence.
fn delete_session(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    crate::components::notification_toggle::begin_delete(&id);
    // Drop it from the list immediately; the refetch below reconciles.
    let keep = js_sys::Array::new();
    for row in js_sys::Array::from(&js::get(&s, "sessions")).iter() {
        if js::get_str(&row, "id") != id {
            keep.push(&row);
        }
    }
    js::set(&s, "sessions", &keep.into());
    // A deleted session cannot stay on the strip.
    remove_open_id(&id);
    rebuild_tabs(&s);
    if js::get_str(&s, "activeId") == id {
        set_active(&s, "");
        js::set(&s, "messages", &js_sys::Array::new().into());
        clear_stream(&s);
    }
    mjs::redraw();
    api::delete_session(&id.clone(), move |v| {
        let s = mjs::store_ref(STORE);
        crate::components::notification_toggle::finish_delete(&s, &id, js::get_bool(&v, "ok"));
        load_sessions(JsValue::UNDEFINED, js_sys::Array::new());
    });
    JsValue::UNDEFINED
}


// ===========================================================================
// Open tabs: which sessions are on the strip.
// ===========================================================================

/// Where the open-tab list is remembered between reloads.
///
/// Per-viewer convenience, so browser storage is the right home: which tabs
/// *I* have open is not shared state and must not outlive my browser. Every
/// access is wrapped, because storage throws in a private window and reads
/// back empty after a clear.
const OPEN_KEY: &str = "chat.openTabs";
const PREVIEW_KEY: &str = "chat.previewTab";
const ACTIVE_KEY: &str = "chat.activeTab";

fn read_active_id() -> String {
    let store = js::get(&js::global(), "sessionStorage");
    js::call(&store, "getItem", &[JsValue::from_str(ACTIVE_KEY)]).as_string().unwrap_or_default()
}

#[derive(Default)]
struct ComposerDraft {
    text: String,
    image: String,
}

fn set_active(store: &JsValue, id: &str) {
    let previous = js::get_str(store, "activeId");
    if previous != id {
        COMPOSER_DRAFTS.with(|drafts| {
            let mut drafts = drafts.borrow_mut();
            drafts.insert(previous, ComposerDraft {
                text: js::get_str(store, "draft"),
                image: js::get_str(store, "image"),
            });
            let next = drafts.get(id);
            js::set_str(store, "draft", next.map_or("", |draft| &draft.text));
            js::set_str(store, "image", next.map_or("", |draft| &draft.image));
        });
        crate::components::composer::reset_history(store);
        js::set_bool(store, "waiting", false);
        js::set_bool(store, "stopping", false);
        js::set_bool(store, "stopUnconfirmed", false);
        js::set_bool(store, "stopPending", false);
    }
    js::set_str(store, "activeId", id);
    let storage = js::get(&js::global(), "sessionStorage");
    js::call(&storage, "setItem", &[JsValue::from_str(ACTIVE_KEY), JsValue::from_str(id)]);
    // Looking at a session means it is read: drop anything it had queued
    // as unread the moment it becomes the one on screen, regardless of how
    // it got there (tab, tree, preview, new session, restored tab).
    if !id.is_empty() {
        crate::components::notification_toggle::clear_session_unread(store, id);
    }
}


fn storage() -> JsValue {
    js::get(&js_sys::global(), "localStorage")
}

fn read_open_ids() -> Vec<String> {
    let st = storage();
    if st.is_undefined() || st.is_null() {
        return Vec::new();
    }
    let raw = js::call(&st, "getItem", &[JsValue::from_str(OPEN_KEY)]);
    let Some(text) = raw.as_string() else {
        return Vec::new();
    };
    text.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn write_open_ids(ids: &[String]) {
    let st = storage();
    if st.is_undefined() || st.is_null() {
        return;
    }
    js::call(
        &st,
        "setItem",
        &[JsValue::from_str(OPEN_KEY), JsValue::from_str(&ids.join(","))],
    );
}

fn read_preview_id() -> String {
    js::call(&storage(), "getItem", &[JsValue::from_str(PREVIEW_KEY)])
        .as_string().unwrap_or_default()
}

fn write_preview_id(id: &str) {
    js::call(&storage(), "setItem", &[JsValue::from_str(PREVIEW_KEY), JsValue::from_str(id)]);
}

/// Preserve the preview's position while replacing it, leaving kept tabs alone.
fn preview_tabs(mut ids: Vec<String>, previous: &str, id: &str) -> (Vec<String>, String) {
    if ids.iter().any(|open| open == id) {
        return (ids, previous.to_string());
    }
    if let Some(pos) = ids.iter().position(|open| !previous.is_empty() && open == previous) {
        ids[pos] = id.to_string();
    } else {
        ids.push(id.to_string());
    }
    (ids, id.to_string())
}

#[cfg(test)]
mod preview_tab_tests {
    use super::preview_tabs;

    #[test]
    fn preview_replaces_only_the_previous_preview() {
        let ids = vec!["kept".into(), "peek".into(), "other".into()];
        let (ids, preview) = preview_tabs(ids, "peek", "next");
        assert_eq!(ids, ["kept", "next", "other"]);
        assert_eq!(preview, "next");
        let (ids, preview) = preview_tabs(ids, &preview, "kept");
        assert_eq!(ids, ["kept", "next", "other"]);
        assert_eq!(preview, "next");
        let (ids, preview) = preview_tabs(ids, "", "new");
        assert_eq!(ids, ["kept", "next", "other", "new"]);
        assert_eq!(preview, "new");
    }
}

/// Rebuild `tabs` from the open-id list and the known sessions.
///
/// Ids whose session no longer exists are dropped, so deleting a session
/// elsewhere cannot leave a tab pointing at nothing.
fn rebuild_tabs(s: &JsValue) {
    let sessions = js_sys::Array::from(&js::get(s, "sessions"));
    let open = read_open_ids();
    let tabs = js_sys::Array::new();
    let mut kept: Vec<String> = Vec::new();
    for id in &open {
        for row in sessions.iter() {
            if &js::get_str(&row, "id") == id {
                tabs.push(&row);
                kept.push(id.clone());
                break;
            }
        }
    }
    if kept.len() != open.len() {
        write_open_ids(&kept);
    }
    let preview = read_preview_id();
    if !preview.is_empty() && !kept.contains(&preview) {
        write_preview_id("");
        js::set_str(s, "previewId", "");
    } else {
        js::set_str(s, "previewId", &preview);
    }
    js::set(s, "tabs", &tabs.into());
}

fn add_open_id(id: &str) {
    let mut ids = read_open_ids();
    if !ids.iter().any(|x| x == id) {
        ids.push(id.to_string());
        write_open_ids(&ids);
    }
}

fn remove_open_id(id: &str) {
    let ids: Vec<String> = read_open_ids().into_iter().filter(|x| x != id).collect();
    write_open_ids(&ids);
    if read_preview_id() == id { write_preview_id(""); }
}

/// Leaving the active preview closes its tab, without deleting the session.
fn close_preview_on_leave(s: &JsValue, next: &str) {
    let preview = read_preview_id();
    if !preview.is_empty() && preview != next && js::get_str(s, "activeId") == preview {
        remove_open_id(&preview);
    }
}

/// `closeTab(id)` — take it off the strip. The session stays on disk in
/// `sessions`; an empty session is deleted when its tab closes.
fn close_tab(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    remove_open_id(&id);
    let s = mjs::store_ref(STORE);
    rebuild_tabs(&s);
    if !owns_turn(&id) {
        let closed_id = id.clone();
        api::delete_session_if_empty(&id, move |result| {
            if js::get_bool(&result, "deleted") {
                let store = mjs::store_ref(STORE);
                crate::components::notification_toggle::begin_delete(&closed_id);
                crate::components::notification_toggle::finish_delete(&store, &closed_id, true);
                load_sessions(JsValue::UNDEFINED, js_sys::Array::new());
            }
        });
    }

    if js::get_str(&s, "activeId") == id {
        // Fall back to whatever tab is left, so closing the active one does
        // not leave an empty panel beside open tabs.
        let tabs = js_sys::Array::from(&js::get(&s, "tabs"));
        if tabs.length() > 0 {
            let next = js::get_str(&tabs.get(0), "id");
            crate::components::composer::reset_history(&s);
            js::set_bool(&s, "treeOpen", false);
            set_active(&s, &next);
            js::set(&s, "messages", &js_sys::Array::new().into());
            clear_stream(&s);
            js::set_bool(&s, "compacted", false);
            js::set(&s, "threads", &js_sys::Array::new().into());
            // Falling back to the next tab is a selection, but like every
            // other view of a session it does not force the inspector open.
            open_session_by_id(&next);
        } else {
            set_active(&s, "");
            js::set(&s, "messages", &js_sys::Array::new().into());
            clear_stream(&s);
        }
    }
    mjs::redraw();
    JsValue::UNDEFINED
}


// ===========================================================================
// Sticky scroll.
// ===========================================================================

/// How close to the bottom still counts as "following along".
///
/// Generous on purpose: a token arriving between the scroll event and the
/// next frame can move the bottom by a line, and a tight threshold would
/// unstick the view for no reason the reader can see.
const STICK_SLACK_PX: f64 = 48.0;

/// `onScroll($event)` — the reader decides whether we follow.
///
/// Scrolling up to read turns sticking off; scrolling back to the bottom
/// turns it on again. Nothing else changes it, so the view never fights the
/// person reading it.
fn on_scroll(_this: JsValue, args: js_sys::Array) -> JsValue {
    let ev = js::arg(&args, 0);
    let el = js::get(&ev, "currentTarget");
    if js::is_missing(&el) {
        return JsValue::UNDEFINED;
    }
    let top = js::get(&el, "scrollTop").as_f64().unwrap_or(0.0);
    let height = js::get(&el, "scrollHeight").as_f64().unwrap_or(0.0);
    let client = js::get(&el, "clientHeight").as_f64().unwrap_or(0.0);
    let at_bottom = height - (top + client) <= STICK_SLACK_PX;
    let s = mjs::store_ref(STORE);
    // Written directly rather than through a redraw: this fires on every
    // scroll frame, and the value only gates the next auto-scroll.
    js::set_bool(&s, "stickBottom", at_bottom);
    // Reaching the bottom of the transcript means the reader has caught up:
    // whatever this session had queued as unread is now read. The guard is
    // cheap (an `all()` over the unread list) and runs only on real scroll.
    if at_bottom {
        let active_id = js::get_str(&s, "activeId");
        if !active_id.is_empty() {
            crate::components::notification_toggle::clear_session_unread(&s, &active_id);
        }
    }
    JsValue::UNDEFINED
}

/// `keepPinned($el)` — scroll to the bottom if we are still following.
///
/// Called from `x-effect`, so it runs whenever the transcript changes. The
/// write is deferred to `$nextTick`: during the effect the diff has not
/// committed yet, so `scrollHeight` is still the previous height and the
/// view would land one row short of the bottom.
fn keep_pinned(_this: JsValue, args: js_sys::Array) -> JsValue {
    let el = js::arg(&args, 0);
    if js::is_missing(&el) {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    if !js::get_bool(&s, "stickBottom") {
        return JsValue::UNDEFINED;
    }
    // One direct DOM write per change, which is what m-js asks for at this
    // frequency rather than another round of diffing.
    let el2 = el.clone();
    let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
        let h = js::get(&el2, "scrollHeight");
        js::set(&el2, "scrollTop", &h);
    });
    let g = js_sys::global();
    js::call(&g, "requestAnimationFrame", &[cb]);
    JsValue::UNDEFINED
}

/// Re-pin when a session is opened, so a resumed transcript starts at the
/// newest message rather than the oldest.
fn pin_to_bottom() {
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "stickBottom", true);
}


/// `loadProjects()` — refresh the folder list.
///
/// Called from the shell's init beside `loadSessions`, and again after every
/// write, because the server returns the new listing with each one.
pub(crate) fn load_projects(_this: JsValue, _args: js_sys::Array) -> JsValue {
    api::projects(|v| {
        apply_projects(&js::get(&v, "projects"));
    });
    JsValue::UNDEFINED
}

/// Take a `projects` array from any response that carries one.
fn apply_projects(list: &JsValue) {
    if list.is_undefined() || list.is_null() {
        return;
    }
    js::set(&mjs::store_ref(STORE), "projects", list);
    mjs::redraw();
}

/// `loadTargets()` — the sandbox, repo, branch and worktree lists.
///
/// Separate from `loadProjects` because they come from different places:
/// projects live in the conversation database, these live on the host (and,
/// for sandboxes, in another service entirely).
pub(crate) fn load_targets(_this: JsValue, _args: js_sys::Array) -> JsValue {
    api::targets(|v| apply_targets(&v));
    JsValue::UNDEFINED
}

/// `loadAutomations()` — the scheduled prompts.
///
/// Separate from `loadTargets` because automations live in chat's own
/// registry (not the host registry), and the rail reads them on its own
/// cadence: a schedule list that only refreshes with the repo list would
/// go stale between target writes.
fn load_automations(_this: JsValue, _args: js_sys::Array) -> JsValue {
    api::automations(|v| apply_automations(&v));
    JsValue::UNDEFINED
}

/// Adopt an automations listing. Split out so the scheduler-independent
/// refresh path (`loadAutomations`) and the write path (`saveTarget`)
/// share it rather than re-typing the key.
fn apply_automations(v: &JsValue) {
    let s = mjs::store_ref(STORE);
    let list = js::get(v, "automations");
    if !list.is_undefined() && !list.is_null() && !same_json(&js::get(&s, "automations"), &list) {
        js::set(&s, "automations", &list);
        mjs::redraw();
    }
}

/// Adopt whatever target lists a response carries.
///
/// Every write answers with the fresh listing, so this runs after each one
/// too and there is never a second request to find out what just happened.
fn apply_targets(v: &JsValue) {
    let s = mjs::store_ref(STORE);
    // `automations` rides along because automation writes answer with the
    // same fresh-listing contract; a plain targets response simply lacks
    // the key and skips it.
    for key in ["sandboxes", "repos", "repoBranches", "worktrees", "automations"] {
        // An unavailable service does not mean the existing sandboxes vanished.
        let list = js::get(v, key);
        if key == "sandboxes" && js::get(v, "sboxUp").as_bool() == Some(false)
            && (js::is_missing(&list) || js_sys::Array::from(&list).length() == 0) { continue; }
        if !list.is_undefined() && !list.is_null() && !same_json(&js::get(&s, key), &list) {
            js::set(&s, key, &list);
        }
    }
    let up = js::get(v, "sboxUp");
    if let Some(up) = up.as_bool() {
        if js::get_bool(&s, "sboxUp") != up { js::set_bool(&s, "sboxUp", up); }
    }
    // The sandboxes menu groups rows per image (PLAN55 D6's rollup). One
    // computation, shared with the samples store — the listing and its
    // rollup cannot disagree because they are the same data.
    let groups = crate::components::sandboxes_menu::group_rows(&js::get(&s, "sandboxes"));
    if !same_json(&js::get(&s, "sandboxGroups"), &groups) {
        js::set(&s, "sandboxGroups", &groups);
    }
    // An open Sandbox form tracks its container's state in real time (the
    // feed republishes every couple of seconds).
    crate::components::target_form::sync_sandbox_status(&s);
}

pub(crate) fn apply_sandboxes(value: &JsValue) { apply_targets(value); }

/// `saveTarget(action, payload)` — every write behind the pickers' Create
/// and Edit actions, plus the automation form's.
///
/// The payload arrives as an object built by the form, because the actions
/// take different shapes and a positional signature covering all of them
/// would be unreadable. `automation-*` actions post to the automations
/// endpoint (chat's own registry, not the host's); everything else keeps
/// the targets endpoint. Both answer the fresh listing, so one callback
/// serves both.
fn save_target(_this: JsValue, args: js_sys::Array) -> JsValue {
    let action = js::arg_str(&args, 0);
    let payload = args.get(1);
    let form = mjs::store_ref("tform");
    let request_id = js::get_num(&form, "requestId");
    js::set_bool(&form, "busy", true);
    // One callback serves both endpoints: each answers the fresh listing,
    // and `apply_targets` adopts whichever keys are present.
    let callback = move |v: JsValue| {
        let form = mjs::store_ref("tform");
        let same_form = js::get_bool(&form, "open") && js::get_num(&form, "requestId") == request_id;
        if js::get(&v, "ok").as_bool().unwrap_or(false) {
            apply_targets(&v);
            if same_form {
                js::set_bool(&form, "busy", false);
                js::set_bool(&form, "open", false);
            }
        } else if same_form {
            js::set_bool(&form, "busy", false);
            let msg = js::get_str(&v, "error");
            crate::components::validation_info::note("tform", "error", if msg.is_empty() { "that did not work" } else { &msg });
        }
    };
    if action.starts_with("automation-") {
        api::automation_action(&action, &payload, callback);
    } else {
        api::target_action(&action, &payload, callback);
    }
    JsValue::UNDEFINED
}

/// `dropSession(id, project)` — the sidebar's drag-and-drop.
///
/// Refiles optimistically so the row lands where it was dropped rather than
/// snapping back for a round-trip, then reconciles with whatever the server
/// says. A drop onto the folder a conversation is already in is a no-op,
/// not a write — dragging something one pixel should not touch the file.
fn drop_session(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    let project = js::arg_str(&args, 1);
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    let sessions = js_sys::Array::from(&js::get(&s, "sessions"));
    let mut moved = false;
    for row in sessions.iter() {
        if js::get_str(&row, "id") == id {
            if js::get_str(&row, "project") == project {
                return JsValue::UNDEFINED;
            }
            js::set_str(&row, "project", &project);
            moved = true;
        }
    }
    if !moved {
        return JsValue::UNDEFINED;
    }
    // The open session's own copy of the metadata, so the composer's
    // project picker agrees with the tree it was just dragged in.
    if js::get_str(&s, "activeId") == id {
        js::set_str(&s, "projectId", &project);
        js::set_str(&js::get(&s, "session"), "project", &project);
    }
    mjs::redraw();

    let payload = js::obj();
    js::set_str(&payload, "sessionId", &id);
    js::set_str(&payload, "project", &project);
    api::project_action("assign", &payload.into(), |v| {
        apply_projects(&js::get(&v, "projects"));
    });
    JsValue::UNDEFINED
}

/// `saveProject(action, a, b)` — create, rename or delete a folder.
///
/// One method for the three writes because they differ only in which two
/// strings they send, and the response handling — adopt the new listing — is
/// identical.
fn save_project(_this: JsValue, args: js_sys::Array) -> JsValue {
    let action = js::arg_str(&args, 0);
    let payload = js::obj();
    match action.as_str() {
        "create" => js::set_str(&payload, "name", &js::arg_str(&args, 1)),
        "rename" => {
            js::set_str(&payload, "from", &js::arg_str(&args, 1));
            js::set_str(&payload, "to", &js::arg_str(&args, 2));
        }
        "delete" => js::set_str(&payload, "name", &js::arg_str(&args, 1)),
        _ => return JsValue::UNDEFINED,
    }
    api::project_action(&action, &payload.into(), |v| {
        apply_projects(&js::get(&v, "projects"));
        // Creating a folder files this conversation in it: the picker is
        // under the composer, so "new project" there means "and put this
        // one in it".
        let created = js::get_str(&v, "select");
        if !created.is_empty() {
            let s = mjs::store_ref(STORE);
            js::set_str(&s, "projectId", &created);
            let id = js::get_str(&s, "activeId");
            if !id.is_empty() {
                js::set_str(&js::get(&s, "session"), "project", &created);
                let p2 = js::obj();
                js::set_str(&p2, "sessionId", &id);
                js::set_str(&p2, "project", &created);
                api::project_action("assign", &p2.into(), |av| {
                    apply_projects(&js::get(&av, "projects"));
                });
            }
        }
        // A rename or a delete rewrites labels on conversations this client
        // is holding, so the session list is no longer trustworthy.
        api::sessions(|sv| {
            let s = mjs::store_ref(STORE);
            js::set(&s, "sessions", &js::get(&sv, "sessions"));
            mjs::redraw();
        });
    });
    JsValue::UNDEFINED
}

/// Rename the visible segment of a project, or a conversation's title.
fn rename_tree_row(_this: JsValue, args: js_sys::Array) -> JsValue {
    let kind = js::arg_str(&args, 0);
    let old = js::arg_str(&args, 1);
    let id = js::arg_str(&args, 2);
    let name = js::arg_str(&args, 3).trim().to_string();
    if name.is_empty() { return JsValue::UNDEFINED; }
    if kind == "project" {
        if name.contains('/') || name == "." || name == ".." { return JsValue::UNDEFINED; }
        let new = match old.rsplit_once('/') {
            Some((parent, _)) => format!("{parent}/{name}"),
            None => name,
        };
        if new != old {
            // The backend merges names on collision. An inline rename should
            // never silently merge two existing project trees.
            let s = mjs::store_ref(STORE);
            let projects = js_sys::Array::from(&js::get(&s, "projects"));
            if projects.iter().any(|p| {
                let path = js::get_str(&p, "name");
                path == new || path.starts_with(&format!("{new}/"))
            }) { return JsValue::UNDEFINED; }
            let a = js_sys::Array::new();
            for value in ["rename", &old, &new] { a.push(&JsValue::from_str(value)); }
            save_project(JsValue::UNDEFINED, a);
        }
    } else if kind == "session" && name != old && !id.is_empty() {
        let payload = js::obj();
        js::set_str(&payload, "title", &name);
        rustex1_kit::api::post_json(&format!("/api/sessions/{id}/settings"), &payload.into(), move |v| {
            let s = mjs::store_ref(STORE);
            if js::get_str(&s, "activeId") == id {
                js::set(&s, "session", &js::get(&v, "session"));
            }
            api::sessions(|v| {
                let s = mjs::store_ref(STORE);
                js::set(&s, "sessions", &js::get(&v, "sessions"));
                rebuild_tabs(&s);
                mjs::redraw();
            });
        });
    }
    JsValue::UNDEFINED
}

/// `pickTarget(kind, id)` — the composer's project / worktree /
/// branch row.
///
/// Records the choice and closes the menu. It does **not** yet persist: the
/// session-meta fields these map to land with the services behind them, and
/// writing a `projectId` that no `/api/projects` can resolve would put a
/// dangling id in the session file.
fn pick_target(_this: JsValue, args: js_sys::Array) -> JsValue {
    let field = match js::arg_str(&args, 0).as_str() {
        "project" => "projectId",
        "worktree" => "worktreeId",
        "branch" => "branchId",
        _ => return JsValue::UNDEFINED,
    };
    let value = js::arg_str(&args, 1);
    let s = mjs::store_ref(STORE);
    js::set_str(&s, field, &value);
    js::call(&mjs::store_ref("picker"), "close", &[]);
    mjs::redraw();
    // The worktree and branch ride the composer-settings path, the
    // same one the model and allowlist pickers use. They have to be
    // durable: the seam mounts the session's checkout into the agent's
    // sandbox from them (PLAN55), so a selection that
    // lived only in the tab would silently stop applying on reload.
    if field != "projectId" {
        let id = js::get_str(&s, "activeId");
        if !id.is_empty() {
            js::set_str(&js::get(&s, "session"), field, &value);
            let payload = js::obj();
            js::set_str(&payload, "sessionId", &id);
            js::set_str(&payload, field, &value);
            rustex1_kit::api::post_json("/api/composer-settings", &payload.into(), |_| {});
        }
    }
    if field == "projectId" {
        let id = js::get_str(&s, "activeId");
        if !id.is_empty() {
            js::set_str(&js::get(&s, "session"), "project", &value);
            let payload = js::obj();
            js::set_str(&payload, "sessionId", &id);
            js::set_str(&payload, "project", &value);
            api::project_action("assign", &payload.into(), |v| {
                apply_projects(&js::get(&v, "projects"));
                api::sessions(|sv| {
                    let st = mjs::store_ref(STORE);
                    js::set(&st, "sessions", &js::get(&sv, "sessions"));
                    mjs::redraw();
                });
            });
        }
    }
    JsValue::UNDEFINED
}

/// `createTarget(kind)` / `editTarget(kind, id)` — open the inspector form.
///
/// The store's own name goes with the call, so the form writes back here
/// rather than to whichever store happened to register last. The design
/// guide passes its own, and gets a form that persists nothing.
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

/// `setPolicy(mode)` — the composer's policy dropdown.
///
/// `Allowlist` enforces the rules and reveals the editor, because choosing
/// it is almost always a prelude to reading or changing them.
/// `Unrestricted` runs with `*`. It is a **temporary override, not an
/// edit**: the rules are left exactly as written and come back when the
/// policy is switched back, so turning it on is never destructive.
fn set_policy(_this: JsValue, args: js_sys::Array) -> JsValue {
    let mode = js::arg_str(&args, 0);
    let unrestricted = mode == "Unrestricted";
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "unrestricted", unrestricted);
    // Picking "Allowlist" opens the editor; picking "Unrestricted" leaves
    // whatever was open alone, so the rules stay readable while inert.
    if !unrestricted {
        js::set_bool(&s, "allowlistOpen", true);
    }
    mjs::redraw();

    let id = js::get_str(&s, "activeId");
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let payload = js::obj();
    js::set_str(&payload, "sessionId", &id);
    // No `allowlist` key: the server only patches the rules when it is
    // present, so this changes the mode and nothing else.
    js::set_str(
        &payload,
        "mode",
        if unrestricted { "unrestricted" } else { "allowlist" },
    );
    rustex1_kit::api::post_json("/api/allowlist", &payload.into(), |_| {});
    JsValue::UNDEFINED
}


// ===========================================================================
// Session metrics.
// ===========================================================================

/// Sum one session's telemetry: totals across every assistant turn, rates
/// from the most recent one that reported them.
fn session_metrics(msgs: &js_sys::Array) -> helpers::SessionMetrics {
    let mut m = helpers::SessionMetrics::default();
    for row in msgs.iter() {
        if js::get_str(&row, "kind") != "assistant" {
            continue;
        }
        let tel = js::get(&row, "telemetry");
        if tel.is_falsy() {
            continue;
        }
        m.input_tokens += nonneg(&tel, "prompt_tokens");
        m.output_tokens += nonneg(&tel, "completion_tokens");
        // Response bytes are the text's own UTF-8 size; the server does not
        // send it because only the rendered message knows it.
        m.bytes += js::get_str(&row, "text").len() as f64;
        // Last turn wins for the rates.
        let tps = nonneg(&tel, "tok_per_sec");
        if tps > 0.0 {
            m.tok_per_sec = tps;
        }
        let dur = nonneg(&tel, "duration_ms");
        if dur > 0.0 {
            m.duration_ms = dur;
        }
    }
    m
}

fn nonneg(o: &JsValue, key: &str) -> f64 {
    let v = js::get_num(o, key);
    if v.is_finite() && v > 0.0 {
        v
    } else {
        0.0
    }
}

/// Add one completed turn to the session totals and re-render the row.
///
/// Kept incremental rather than recomputed from `messages`: the sealed card
/// arrives before the message list is reconciled with the server, so
/// summing the list here would miss the turn that just finished.
fn add_turn_metrics(s: &JsValue, tel: &JsValue, text: &str) {
    let mut m = helpers::SessionMetrics {
        tok_per_sec: js::get_num(s, "mTps"),
        duration_ms: js::get_num(s, "mDur"),
        input_tokens: js::get_num(s, "mIn"),
        output_tokens: js::get_num(s, "mOut"),
        bytes: js::get_num(s, "mBytes"),
    };
    if !tel.is_falsy() {
        m.input_tokens += nonneg(tel, "prompt_tokens");
        m.output_tokens += nonneg(tel, "completion_tokens");
        let tps = nonneg(tel, "tok_per_sec");
        if tps > 0.0 {
            m.tok_per_sec = tps;
        }
        let dur = nonneg(tel, "duration_ms");
        if dur > 0.0 {
            m.duration_ms = dur;
        }
    }
    m.bytes += text.len() as f64;
    store_metrics(s, &m);
}

/// Persist the running totals plus their rendered row.
fn store_metrics(s: &JsValue, m: &helpers::SessionMetrics) {
    js::set_num(s, "mTps", m.tok_per_sec);
    js::set_num(s, "mDur", m.duration_ms);
    js::set_num(s, "mIn", m.input_tokens);
    js::set_num(s, "mOut", m.output_tokens);
    js::set_num(s, "mBytes", m.bytes);
    js::set_str(s, "lastMetrics", &helpers::metrics_html(m));
}

// ===========================================================================
// The memory tree in the UI: threads you can open one level at a time
// ===========================================================================
//
// `threads` is a **flat** array that renders as a nested list, because m-js
// diffs a keyed list far more cheaply than it diffs a recursive component
// tree, and the nesting here is only ever presentational. Each entry carries
// its own `depth`, and the template indents by it.
//
// Opening a thread splices its two halves in directly after it. Nothing is
// fetched until someone opens something, so the top of a very long
// conversation costs exactly as many rows as it shows.

/// Build one thread row from a `/view` or `/zoom` part.
fn thread_row(part: &JsValue, depth: f64) -> JsValue {
    let row = js::obj();
    let r = js::get_str(part, "ref");
    js::set_str(&row, "ref", &r);
    // The key has to be unique across the whole flat list, and the same
    // reference can legitimately appear at two depths while a parent is open.
    js::set_str(&row, "id", &format!("t{depth}-{r}"));
    js::set(&row, "from", &js::get(part, "id"));
    js::set(&row, "n", &js::get(part, "n"));
    js::set(&row, "level", &js::get(part, "level"));
    js::set_num(&row, "depth", depth);
    js::set_bool(&row, "leaf", js::get_bool(part, "leaf"));
    js::set_bool(&row, "built", js::get_bool(part, "built"));
    js::set_bool(&row, "clipped", js::get_bool(part, "clipped"));
    js::set_bool(&row, "expanded", false);
    js::set_bool(&row, "loading", false);
    let text = js::get_str(part, "text");
    js::set_str(&row, "text", &text);
    // A summary is prose from a model, so it gets the same markdown
    // treatment as an answer.
    js::set_str(&row, "html", &helpers::markdown_to_html(&text));
    // A leaf carries the original message, which renders as the card it
    // always was rather than as a summary line.
    let msg = js::get(part, "message");
    if !msg.is_undefined() && !msg.is_null() {
        add_html(&msg);
        js::set(&row, "message", &msg);
    }
    // How many messages this line stands for — the count the UI shows on a
    // collapsed thread.
    let n = js::get(part, "n").as_f64().unwrap_or(1.0);
    js::set_str(
        &row,
        "label",
        &if n <= 1.0 {
            String::new()
        } else {
            format!("{} messages", helpers::fmt_count(n))
        },
    );
    row.into()
}

/// `loadView()` — pull the memory view for the open session.
fn load_view(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let id = js::get_str(&s, "activeId");
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let want = id.clone();
    api::memory_view(&id, move |v| {
        let s = mjs::store_ref(STORE);
        if js::get_str(&s, "activeId") != want {
            return;
        }
        let parts = js_sys::Array::from(&js::get(&v, "parts"));
        let rows = js_sys::Array::new();
        for p in parts.iter() {
            rows.push(&thread_row(&p, 0.0));
        }
        js::set(&s, "threads", &rows.into());
        js::set_bool(&s, "compacted", js::get_bool(&v, "compacted"));
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

/// `compact()` — run compaction on the open conversation.
///
/// Explicit rather than automatic: compaction spends model calls, and the
/// moment a conversation gets re-shaped under the reader is a moment they
/// should have asked for.
fn compact(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let id = js::get_str(&s, "activeId");
    if id.is_empty() || js::get_bool(&s, "compacting") {
        return JsValue::UNDEFINED;
    }
    js::set_bool(&s, "compacting", true);
    mjs::redraw();
    let payload = js::obj();
    let high = js::get(&s, "memoryHigh").as_f64().unwrap_or(0.0);
    if high > 0.0 {
        js::set_num(&payload, "high", high);
    }
    api::compact(&id, &payload.into(), move |_| {
        let s = mjs::store_ref(STORE);
        js::set_bool(&s, "compacting", false);
        load_view(JsValue::UNDEFINED, js_sys::Array::new());
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

/// Index of the thread row with this key, or -1.
fn thread_index(rows: &js_sys::Array, key: &str) -> i32 {
    for (i, r) in rows.iter().enumerate() {
        if js::get_str(&r, "id") == key {
            return i as i32;
        }
    }
    -1
}

/// `toggleThread(key)` — open a thread one level, or close it.
fn toggle_thread(_this: JsValue, args: js_sys::Array) -> JsValue {
    let key = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    let rows = js_sys::Array::from(&js::get(&s, "threads"));
    let at = thread_index(&rows, &key);
    if at < 0 {
        return JsValue::UNDEFINED;
    }
    let row = rows.get(at as u32);
    if js::get_bool(&row, "leaf") {
        // A width-1 part is already the original message; there is nothing
        // under it to open.
        return JsValue::UNDEFINED;
    }
    if js::get_bool(&row, "expanded") {
        collapse_at(&s, &rows, at);
        return JsValue::UNDEFINED;
    }

    let id = js::get_str(&s, "activeId");
    let from = js::get(&row, "from").as_f64().unwrap_or(0.0);
    let n = js::get(&row, "n").as_f64().unwrap_or(1.0);
    js::set_bool(&row, "loading", true);
    mjs::redraw();

    let key2 = key.clone();
    api::zoom(&id, from, n, move |v| {
        let s = mjs::store_ref(STORE);
        let rows = js_sys::Array::from(&js::get(&s, "threads"));
        let at = thread_index(&rows, &key2);
        if at < 0 {
            return;
        }
        let row = rows.get(at as u32);
        js::set_bool(&row, "loading", false);
        let depth = js::get(&row, "depth").as_f64().unwrap_or(0.0);

        let kids = js_sys::Array::new();
        if js::get_str(&v, "kind") == "halves" {
            for p in js_sys::Array::from(&js::get(&v, "parts")).iter() {
                kids.push(&thread_row(&p, depth + 1.0));
            }
        }
        if kids.length() == 0 {
            mjs::redraw();
            return;
        }
        js::set_bool(&row, "expanded", true);
        // Splice the halves in right after their parent. The parent stays
        // visible as the thread's header, so you can see what you opened.
        let next = js_sys::Array::new();
        for (i, r) in rows.iter().enumerate() {
            next.push(&r);
            if i as i32 == at {
                for k in kids.iter() {
                    next.push(&k);
                }
            }
        }
        // Replace the array rather than mutating it: an in-place splice may
        // not notify.
        js::set(&s, "threads", &next.into());
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

/// Close a thread: drop every row nested under it.
fn collapse_at(s: &JsValue, rows: &js_sys::Array, at: i32) {
    let row = rows.get(at as u32);
    let depth = js::get(&row, "depth").as_f64().unwrap_or(0.0);
    js::set_bool(&row, "expanded", false);
    let next = js_sys::Array::new();
    let mut skipping = false;
    for (i, r) in rows.iter().enumerate() {
        let d = js::get(&r, "depth").as_f64().unwrap_or(0.0);
        if i as i32 == at {
            skipping = true;
            next.push(&r);
            continue;
        }
        if skipping {
            // Everything deeper than the parent belongs to it; the first row
            // back at the parent's own depth ends the subtree.
            if d > depth {
                continue;
            }
            skipping = false;
        }
        next.push(&r);
    }
    js::set(s, "threads", &next.into());
    mjs::redraw();
}

// ===========================================================================
// The roster: who is in the conversation
// ===========================================================================

/// Store the roster and build the id → handle map the transcript uses to
/// attribute each message.
fn store_roster(s: &JsValue, list: &js_sys::Array) {
    js::set(s, "participants", &list.clone().into());
    let by_id = js::obj();
    let mut agents = 0.0;
    // Who to show in the roster bar: everyone **except me**. A list that
    // includes the reader is a list of the room, not of who they are
    // talking to — and the one entry they can do nothing about.
    //
    // "Me" is the generic `user` seat; a **named** user seat is someone
    // invited in — the "other humans" this roster always meant to show
    // (a `@mention` or `/invite` of a person seats them exactly like an
    // agent).
    let others = js_sys::Array::new();
    let active_others = js_sys::Array::new();
    // handle -> colour, so a row can be tinted without searching the roster
    // on every redraw. Keyed by handle rather than by participant id because
    // a streamed card names its speaker by handle.
    let accents = js::obj();
    // …and the same for the glyph, so a byline can be drawn as its author
    // without the row carrying a copy of the definition.
    let icons = js::obj();
    for p in list.iter() {
        let id = js::get(&p, "id").as_f64().unwrap_or(-1.0);
        let handle = js::get_str(&p, "handle");
        js::set_str(&by_id, &format!("{id}"), &handle);
        let kind = js::get_str(&p, "kind");
        // A person wears the person's mark whatever the row says: a user
        // seat has no agent icon or accent to copy.
        let is_person = kind == "user";
        let colour = js::get_str(&p, "accentColor");
        js::set_str(
            &accents,
            &handle,
            if is_person {
                crate::components::presence::USER_ACCENT
            } else if colour.is_empty() {
                crate::components::agents::DEFAULT_ACCENT
            } else {
                &colour
            },
        );
        let icon = js::get_str(&p, "icon");
        js::set_str(
            &icons,
            &handle,
            if is_person {
                crate::components::presence::USER_ICON
            } else if icon.is_empty() {
                crate::components::agents::DEFAULT_ICON
            } else {
                &icon
            },
        );
        if kind == "agent" {
            if js::get_bool(&p, "active") {
                agents += 1.0;
                active_others.push(&p);
            }
            others.push(&p);
        } else if is_person && handle != "user" {
            // The other humans: seated people appear in the roster bar.
            // The generic `user` seat stays out — that one is me.
            if js::get_bool(&p, "active") {
                active_others.push(&p);
            }
            others.push(&p);
        }
    }
    js::set(s, "others", &others.into());
    js::set(s, "activeOthers", &active_others.into());
    js::set(s, "accents", &accents);
    js::set(s, "icons", &icons);
    js::set(s, "handles", &by_id);
    js::set_num(s, "agentCount", agents);
    // Two or more agents is what makes this a group conversation, which is
    // what the server uses to pick the group turn path.
    js::set_bool(s, "isGroup", agents > 1.0);
}

/// `openSessionEditor()` — the pencil beside the presence cluster.
///
/// Seeds the draft from the live conversation — its meta for the five fields,
/// its roster for the seats — closes the panels it displaces, and opens the
/// column. The seats are copies, not the roster's own objects: unseating in
/// the editor must not dim the live cluster before anything is saved, and
/// inviting must not seat a reference the room never approved.
fn open_session_editor(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let live = mjs::store_ref(STORE);
    let draft = mjs::store_ref("sessionDraft");
    let meta = js::get(&live, "session");
    // `title` is seeded on its own: sessions made without a name carry no
    // stored one, and the tree/tab bar then show the server's display
    // fallback (`title || agent || id`). The form should match the name the
    // user sees on the row, not an empty box — and typing a real name in
    // `save()` persists it, which is exactly what renaming is.
    let mut title = js::get_str(&meta, "title");
    if title.is_empty() {
        // The list row carries the server's display name for a title-less
        // session; on a fresh `+` the list may not have refetched yet, so
        // the id is the last resort — the server's own fallback chain is
        // `title || agent || id`, so this never misleads.
        let row_id = js::get_str(&meta, "id");
        for r in js_sys::Array::from(&js::get(&live, "sessions")).iter() {
            if js::get_str(&r, "id") == row_id {
                title = js::get_str(&r, "title");
                break;
            }
        }
        if title.is_empty() { title = row_id; }
    }
    for (draft_key, meta_key) in [("purpose", "purpose"), ("notes", "notes"), ("routing", "routing")] {
        js::set_str(&draft, draft_key, &js::get_str(&meta, meta_key));
    }
    js::set_str(&draft, "title", &title);
    // A missing routing never leaves the switch without a side.
    if js::get_str(&draft, "routing").is_empty() {
        js::set_str(&draft, "routing", "everyone");
    }
    let mh = js::get(&meta, "memoryHigh");
    if mh.is_null() || mh.is_undefined() || mh.as_f64().is_none() {
        js::set(&draft, "memoryHigh", &JsValue::NULL);
    } else {
        js::set(&draft, "memoryHigh", &mh);
    }
    let seats = js_sys::Array::new();
    for p in js_sys::Array::from(&js::get(&live, "others")).iter() {
        let copy = js::obj();
        js::set(&copy, "id", &js::get(&p, "id"));
        for key in ["handle", "icon", "accentColor"] {
            js::set_str(&copy, key, &js::get_str(&p, key));
        }
        js::set_bool(&copy, "active", js::get_bool(&p, "active"));
        seats.push(&copy);
    }
    js::set(&draft, "seats", &seats.into());
    js::set_str(&draft, "inviteHandle", "");
    js::set_str(&draft, "message", "");
    js::set_str(&draft, "messageTone", "");
    js::set_bool(&draft, "open", true);
    // One panel at a time: the inspector shows what you last selected.
    let agents = mjs::store_ref("agents");
    js::set_bool(&agents, "open", false);
    js::set_str(&agents, "selected", "");
    js::set_bool(&mjs::store_ref("settings"), "open", false);
    js::set_bool(&mjs::store_ref("tform"), "open", false);
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `removeAgent(id)` — unseat a participant.
fn remove_agent(_this: JsValue, args: js_sys::Array) -> JsValue {
    let pid = js::arg_num(&args, 0, 0.0);
    if pid <= 0.0 {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    let id = js::get_str(&s, "activeId");
    api::remove_participant(pid, move |_| {
        if !id.is_empty() {
            load_roster(&id);
        }
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

fn load_roster(id: &str) {
    let want = id.to_string();
    rustex1_kit::api::get(
        &format!("/api/sessions/{id}/participants"),
        None,
        move |v| {
            let s = mjs::store_ref(STORE);
            if js::get_str(&s, "activeId") != want {
                return;
            }
            store_roster(&s, &js_sys::Array::from(&js::get(&v, "participants")));
            mjs::redraw();
        },
    );
}

// ===========================================================================
// Branching: the cursor picks which version of the conversation you are in
// ===========================================================================

fn tree_escape(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let event = js::arg(&args, 0);
    if js::get_bool(&event, "repeat") { return JsValue::UNDEFINED; }
    if js::get_bool(&s, "treeOpen") {
        js::call(&event, "preventDefault", &[]);
        return tree_close(JsValue::UNDEFINED, js_sys::Array::new());
    }
    let now = js_sys::Date::now();
    let previous = js::get(&s, "treeEscapeAt").as_f64().unwrap_or(0.0);
    js::set_num(&s, "treeEscapeAt", now);
    let id = js::get_str(&s, "activeId");
    if previous == 0.0 || now - previous > 750.0 || id.is_empty() || js::get_bool(&s, "waiting") {
        return JsValue::UNDEFINED;
    }
    js::call(&event, "preventDefault", &[]);
    js::set_num(&s, "treeEscapeAt", 0.0);
    js::set_bool(&s, "treeOpen", true);
    js::set_bool(&s, "treeLoading", true);
    let request = js::get(&s, "treeRequest").as_f64().unwrap_or(0.0) + 1.0;
    js::set_num(&s, "treeRequest", request);
    js::set(&s, "treeRows", &js_sys::Array::new().into());
    mjs::redraw();
    let want = id.clone();
    api::tree(&id, move |v| {
        let s = mjs::store_ref(STORE);
        if js::get_str(&s, "activeId") != want || !js::get_bool(&s, "treeOpen")
            || js::get(&s, "treeRequest").as_f64() != Some(request) { return; }
        let nodes = js_sys::Array::from(&js::get(&v, "nodes"));
        let cursor = js::get_str(&v, "cursor");
        let rows = crate::components::history_tree::flatten(&nodes, &cursor);
        let array = js_sys::Array::from(&rows);
        let index = array.iter().position(|r| js::get_str(&r, "id") == cursor).unwrap_or(0);
        js::set(&s, "treeRows", &rows);
        js::set_bool(&s, "treeLoading", false);
        crate::components::history_tree::select(&s, index as i64);
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

fn tree_reset_escape(_this: JsValue, _args: js_sys::Array) -> JsValue {
    js::set_num(&mjs::store_ref(STORE), "treeEscapeAt", 0.0);
    JsValue::UNDEFINED
}

fn tree_close(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "treeOpen", false);
    js::set_num(&s, "treeEscapeAt", 0.0);
    mjs::redraw();
    crate::components::history_tree::focus_composer();
    JsValue::UNDEFINED
}

fn tree_select(_this: JsValue, args: js_sys::Array) -> JsValue {
    crate::components::history_tree::select(&mjs::store_ref(STORE), js::arg_num(&args, 0, 0.0) as i64);
    JsValue::UNDEFINED
}

fn tree_move(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let index = js::get(&s, "treeIndex").as_f64().unwrap_or(0.0) as i64;
    crate::components::history_tree::select(&s, index + js::arg_num(&args, 0, 0.0) as i64);
    JsValue::UNDEFINED
}

fn tree_choose_selected(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let rows = js_sys::Array::from(&js::get(&s, "treeRows"));
    let index = js::get(&s, "treeIndex").as_f64().unwrap_or(0.0) as u32;
    let args = js_sys::Array::new();
    args.push(&JsValue::from_str(&js::get_str(&rows.get(index), "id")));
    tree_choose(JsValue::UNDEFINED, args)
}

fn tree_choose(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let id = js::arg_str(&args, 0);
    if id.is_empty() || js::get_bool(&s, "waiting") { return JsValue::UNDEFINED; }
    let session = js::get_str(&s, "activeId");
    let payload = js::obj();
    js::set_str(&payload, "eventId", &id);
    api::set_cursor(&session.clone(), &payload.into(), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get_str(&s, "activeId") != session || !js::get_bool(&v, "ok") { return; }
        js::set_str(&s, "cursor", &js::get_str(&v, "cursor"));
        js::set_bool(&s, "treeOpen", false);
        crate::components::composer::reset_history(&s);
        open_session_by_id(&session);
        mjs::redraw();
        let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
            let document = js::get(&js_sys::global(), "document");
            let input = js::call(&document, "querySelector", &[JsValue::from_str("#home .composer .draft-ta")]);
            if !js::is_missing(&input) { js::call(&input, "focus", &[]); }
        });
        js::call(&js_sys::global(), "requestAnimationFrame", &[callback]);
    });
    JsValue::UNDEFINED
}

/// Move before the selected prompt and place its text back in the composer.
fn fork_prompt(_this: JsValue, args: js_sys::Array) -> JsValue {
    let event_id = js::arg_str(&args, 0);
    let prompt = js::arg_str(&args, 1);
    let id = js::get_str(&mjs::store_ref(STORE), "activeId");
    if id.is_empty() || event_id.is_empty() { return JsValue::UNDEFINED; }
    let payload = js::obj();
    js::set_str(&payload, "eventId", &event_id);
    js::set_bool(&payload, "before", true);
    let want = id.clone();
    api::set_cursor(&id, &payload.into(), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get_str(&s, "activeId") != want || !js::get_bool(&v, "ok") { return; }
        js::set_str(&s, "cursor", &js::get_str(&v, "cursor"));
        js::set_str(&s, "draft", &prompt);
        open_session_by_id(&want);
        mjs::redraw();
        let expected = want.clone();
        let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
            if js::get_str(&mjs::store_ref(STORE), "activeId") != expected { return; }
            let document = js::get(&js_sys::global(), "document");
            let input = js::call(&document, "querySelector", &[JsValue::from_str("#home .composer .draft-ta")]);
            if !js::is_missing(&input) { js::call(&input, "focus", &[]); }
        });
        js::call(&js_sys::global(), "requestAnimationFrame", &[callback]);
    });
    JsValue::UNDEFINED
}

/// `handleOf(participantId)` — the `@handle` that authored a message.
fn handle_of(_this: JsValue, args: js_sys::Array) -> JsValue {
    let pid = js::arg_num(&args, 0, 0.0);
    let s = mjs::store_ref(STORE);
    let map = js::get(&s, "handles");
    let h = js::get_str(&map, &format!("{pid}"));
    JsValue::from_str(&h)
}


// ===========================================================================
// Slash commands
// ===========================================================================

/// Add a transient notice to the transcript.
///
/// Not persisted on purpose: it is the app answering you, not something
/// anyone said, and a reload should not replay it.
fn notice(s: &JsValue, text: &str) {
    push_message(s, "system", text, "");
    let rows = js_sys::Array::from(&js::get(s, "messages"));
    if rows.length() > 0 { js::set_bool(&rows.get(rows.length() - 1), "commandNotice", true); }
    pin_to_bottom();
    mjs::redraw();
}

/// Save a roster change before showing it, so transcript reloads retain it.
fn record_notice(session: &str, text: &str) {
    let payload = js::obj();
    js::set_str(&payload, "sessionId", session);
    js::set_str(&payload, "text", text);
    let session = session.to_string();
    let text = text.to_string();
    rustex1_kit::api::post_json("/api/session/status", &payload.into(), move |result| {
        let s = mjs::store_ref(STORE);
        if js::get_str(&s, "activeId") != session { return; }
        if !js::get_bool(&result, "ok") {
            notice(&s, "Could not save the session status.");
            return;
        }
        notice(&s, &text);
        let rows = js_sys::Array::from(&js::get(&s, "messages"));
        if rows.length() > 0 {
            let row = rows.get(rows.length() - 1);
            js::set_str(&row, "id", &js::get_str(&result, "eventId"));
            js::set_str(&row, "eventId", &js::get_str(&result, "eventId"));
            js::set_str(&row, "ts", &js::get_str(&result, "ts"));
        }
        mjs::redraw();
    });
}

/// Run a `/command` from the composer.
fn run_command(s: &JsValue, line: &str) {
    let mut parts = line.split_whitespace();
    let cmd = parts.next().unwrap_or("").to_ascii_lowercase();
    let args: Vec<String> = parts.map(|a| a.trim_start_matches('@').to_string()).collect();
    match cmd.as_str() {
        "/invite" => invite_command(s, &args),
        "/uninvite" | "/kick" => uninvite_command(s, &args),
        "/help" => {
            js::set_str(s, "draft", "");
            notice(
                s,
                "/invite @name — add an agent or person to this conversation\n\
                 /kick @name — remove an agent or person (/uninvite also works)\n\
                 /help — this list",
            );
        }
        other => {
            // Left in the box: a typo should not cost you what you typed.
            notice(s, &format!("Unknown command {other}. Try /help."));
        }
    }
}

/// `/invite @a @b` — seat agents or people in the open conversation.
///
/// The name is resolved server-side, the way a mention resolves it: an agent
/// of that name first, a user next — seating either kind is the same act.
/// The answers this side keeps are "already here" (the roster is right
/// here) and whatever the server says about the rest.
fn invite_command(s: &JsValue, names: &[String]) {
    let session = js::get_str(s, "activeId");
    if session.is_empty() {
        notice(s, "Open a conversation first, then /invite someone into it.");
        return;
    }
    if names.is_empty() {
        notice(s, "Usage: /invite @name — an agent's handle, or a person's name.");
        return;
    }

    let seats = js_sys::Array::from(&js::get(s, "others"));
    let seated: Vec<String> = seats.iter().filter(|p| js::get_bool(p, "active"))
        .map(|p| js::get_str(&p, "handle"))
        .collect();

    let mut invited: Vec<String> = Vec::new();
    let mut already: Vec<String> = Vec::new();
    for name in names {
        if invited.contains(name) || seated.contains(name) {
            already.push(name.clone());
        } else {
            invited.push(name.clone());
        }
    }

    let mut said: Vec<String> = Vec::new();
    if !already.is_empty() {
        said.push(format!("already here: @{}", already.join(", @")));
    }
    if invited.is_empty() {
        if !said.is_empty() {
            notice(s, &said.join("; "));
        }
        return;
    }

    js::set_str(s, "draft", "");
    // One request per name; the roster is re-read once at the end rather
    // than after each, so the bar does not flicker through intermediate
    // states on a multi-name invite.
    let total = invited.len();
    let results = std::rc::Rc::new(std::cell::RefCell::new((Vec::<String>::new(), Vec::<String>::new())));
    for name in invited.clone() {
        let payload = js::obj();
        // The server resolves the name — an agent first, a person next —
        // and re-seats a name that is already in the room.
        js::set_str(&payload, "name", &name);
        let session = session.clone();
        let results = std::rc::Rc::clone(&results);
        let said = said.clone();
        rustex1_kit::api::post_json(
            &format!("/api/sessions/{session}/participants"),
            &payload.into(),
            move |v| {
                let mut results = results.borrow_mut();
                if js::get_bool(&v, "ok") {
                    results.0.push(name.clone());
                } else {
                    let why = js::get_str(&v, "error");
                    results.1.push(if why.is_empty() {
                        format!("Could not invite @{name}")
                    } else {
                        format!("{why}")
                    });
                }
                if results.0.len() + results.1.len() < total { return; }
                load_roster(&session);
                let mut messages = Vec::new();
                if !results.0.is_empty() { messages.push(format!("Invited @{}", results.0.join(", @"))); }
                messages.extend(results.1.clone());
                messages.extend(said.clone());
                let message = messages.join("; ");
                if !results.0.is_empty() { record_notice(&session, &message); }
                else {
                    let s = mjs::store_ref(STORE);
                    if js::get_str(&s, "activeId") == session { notice(&s, &message); }
                }
            },
        );
    }
}

/// Remove active agents by handle, retaining their existing message authorship.
fn uninvite_command(s: &JsValue, names: &[String]) {
    let session = js::get_str(s, "activeId");
    if session.is_empty() {
        notice(s, "Open a conversation first, then /kick an agent from it.");
        return;
    }
    if names.is_empty() {
        notice(s, "Usage: /kick @name");
        return;
    }
    let seats = js_sys::Array::from(&js::get(s, "activeOthers"));
    let mut targets = Vec::new();
    let mut missing = Vec::new();
    for name in names {
        if targets.iter().any(|(handle, _)| handle == name) || missing.contains(name) { continue; }
        if let Some(p) = seats.iter().find(|p| js::get_str(p, "handle") == *name) {
            if let Some(pid) = js::get(&p, "id").as_f64().filter(|id| *id > 0.0) {
                targets.push((name.clone(), pid));
                continue;
            }
        }
        missing.push(name.clone());
    }
    if targets.is_empty() {
        notice(s, &format!("@{} is not in this conversation.", missing.join(", @")));
        return;
    }
    js::set_str(s, "draft", "");
    let total = targets.len();
    let results = std::rc::Rc::new(std::cell::RefCell::new((Vec::<String>::new(), Vec::<String>::new())));
    for (name, pid) in targets {
        let session = session.clone();
        let results = std::rc::Rc::clone(&results);
        let missing = missing.clone();
        api::remove_participant(pid, move |v| {
            let mut results = results.borrow_mut();
            if js::get_bool(&v, "ok") { results.0.push(name.clone()); }
            else { results.1.push(name.clone()); }
            if results.0.len() + results.1.len() != total { return; }
            load_roster(&session);
            let s = mjs::store_ref(STORE);
            let mut messages = Vec::new();
            if !results.0.is_empty() { messages.push(format!("Removed @{}", results.0.join(", @"))); }
            if !results.1.is_empty() { messages.push(format!("Could not remove @{}", results.1.join(", @"))); }
            if !missing.is_empty() { messages.push(format!("not in this conversation: @{}", missing.join(", @"))); }
            if !results.0.is_empty() { record_notice(&session, &messages.join("; ")); }
            else if js::get_str(&s, "activeId") == session { notice(&s, &messages.join("; ")); }
        });
    }
}



/// `toggleReasoning(id)` — open or shut one reasoning card.
///
/// An empty id means the turn in flight, which has no row of its own and
/// keeps its state on the store instead.
fn toggle_reasoning(_this: JsValue, args: js_sys::Array) -> JsValue {
    // Selecting text inside an open card is a click too. Collapsing it out
    // from under a reader who was trying to copy a line is the obvious way
    // to make "click anywhere" annoying, so a click that ended a selection
    // is not a toggle.
    if has_selection() {
        return JsValue::UNDEFINED;
    }
    let id = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    if id.is_empty() {
        let open = js::get_bool(&s, "streamReasoningOpen");
        js::set_bool(&s, "streamReasoningOpen", !open);
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    let list = js_sys::Array::from(&js::get(&s, "messages"));
    let next = js_sys::Array::new();
    for msg in list.iter() {
        if js::get_str(&msg, "id") == id {
            let open = js::get_bool(&msg, "expanded");
            js::set_bool(&msg, "expanded", !open);
        }
        next.push(&msg);
    }
    // Replace the array rather than relying on the in-place flag to notify:
    // a property written on a row inside a reactive array may not reach the
    // diff on its own (RUST_M_JS §3).
    js::set(&s, "messages", &next.into());
    mjs::redraw();
    JsValue::UNDEFINED
}


/// Is there a non-empty text selection on the page?
fn has_selection() -> bool {
    let sel = js::call(&js_sys::global(), "getSelection", &[]);
    if sel.is_undefined() || sel.is_null() {
        return false;
    }
    js::call(&sel, "toString", &[])
        .as_string()
        .map(|t| !t.trim().is_empty())
        .unwrap_or(false)
}


/// A removed, never-speaking agent no longer has a seat, but its definition
/// still supplies the same icon and colour to mentions in earlier notices.
fn defined_agent_field(handle: &str, field: &str) -> String {
    js_sys::Array::from(&js::get(&mjs::store_ref("agents"), "agents")).iter()
        .find(|agent| js::get_str(agent, "name") == handle)
        .map(|agent| js::get_str(&agent, field))
        .unwrap_or_default()
}

/// `accentOf(handle)` — the colour a row is drawn in.
///
/// Always a concrete colour, never empty: an empty custom property makes the
/// declaration that uses it invalid at computed-value time, which falls back
/// to *inherited*, not to the `var()` fallback — so the row would take its
/// neighbour's colour rather than the default.
fn accent_of(_this: JsValue, args: js_sys::Array) -> JsValue {
    let handle = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    let mut found = js::get_str(&js::get(&s, "accents"), &handle);
    if found.is_empty() { found = defined_agent_field(&handle, "accentColor"); }
    JsValue::from_str(if found.is_empty() {
        crate::components::agents::DEFAULT_ACCENT
    } else {
        &found
    })
}

/// `iconOf(handle)` — the glyph a byline is drawn with.
///
/// Always a concrete name, like [`accent_of`]: `'ph ph-' + ''` is the class
/// `ph-`, which draws a blank where the author's icon belongs. A handle the
/// roster does not know — the human seat, or an agent that has since been
/// unseated — falls back to the default rather than to nothing.
fn icon_of(_this: JsValue, args: js_sys::Array) -> JsValue {
    let handle = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    let mut found = js::get_str(&js::get(&s, "icons"), &handle);
    if found.is_empty() { found = defined_agent_field(&handle, "icon"); }
    JsValue::from_str(if found.is_empty() {
        crate::components::agents::DEFAULT_ICON
    } else {
        &found
    })
}

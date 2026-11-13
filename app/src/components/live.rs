//! `live` — the store that binds the home route to angela.
//!
//! One store feeds all three columns, so they cannot disagree about which
//! session is open:
//!
//! | column | reads |
//! |---|---|
//! | history rail | `sessions` |
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
    api::sessions(|v| {
        let s = mjs::store_ref(STORE);
        js::set(&s, "sessions", &js::get(&v, "sessions"));
        js::set_str(&s, "projectRoot", &js::get_str(&v, "projectRoot"));
        rebuild_tabs(&s);
        // Re-open whatever was on the strip last time. Nothing is adopted
        // automatically: with no open tabs the column is deliberately bare.
        if js::get_str(&s, "activeId").is_empty() {
            let tabs = js_sys::Array::from(&js::get(&s, "tabs"));
            if tabs.length() > 0 {
                let id = js::get_str(&tabs.get(0), "id");
                js::set_str(&s, "activeId", &id);
                open_session_by_id(&id);
            }
        }
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

/// Load one session's transcript into `messages`.
fn open_session_by_id(id: &str) {
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
        js::set(&s, "messages", &msgs.into());
        js::set(&s, "session", &js::get(&v, "session"));
        // Seed the allowlist editor: the session's override if it has one,
        // else the agent default. The baseline is what "overridden" is
        // measured against.
        let text = js::get_str(&v, "allowlist");
        js::set_str(&s, "allowlist", &text);
        let overridden = js::get_bool(&v, "allowlistOverridden");
        js::set_bool(&s, "allowlistOverridden", overridden);
        if !overridden {
            js::set_str(&s, "allowlistBaseline", &text);
        }
        clear_stream(&s);
        mjs::redraw();
    });
}

fn clear_stream(s: &JsValue) {
    js::set_str(s, "streamText", "");
    js::set_str(s, "streamHtml", "");
    js::set_str(s, "streamReasoning", "");
    js::set_str(s, "streamReasoningHtml", "");
}

/// `openSession(id)` — a click in the history rail or the tab bar.
fn open_session(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    // Opening from the history rail puts it on the strip if it is not there.
    add_open_id(&id);
    rebuild_tabs(&s);
    js::set_str(&s, "activeId", &id);
    js::set(&s, "messages", &js_sys::Array::new().into());
    clear_stream(&s);
    pin_to_bottom();
    mjs::redraw();
    open_session_by_id(&id);
    JsValue::UNDEFINED
}

/// `newSession()` — the tab bar's `+`.
fn new_session(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let payload = js::obj();
    api::new_session(&payload.into(), |v| {
        let s = mjs::store_ref(STORE);
        let id = js::get_str(&js::get(&v, "session"), "id");
        if !id.is_empty() {
            add_open_id(&id);
            js::set_str(&s, "activeId", &id);
            js::set(&s, "messages", &js_sys::Array::new().into());
            clear_stream(&s);
        }
        mjs::redraw();
        load_sessions(JsValue::UNDEFINED, js_sys::Array::new());
    });
    JsValue::UNDEFINED
}

/// `closeSession(id)` — a tab's `×`.
fn close_session(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    if js::get_str(&s, "activeId") == id {
        js::set_str(&s, "activeId", "");
        js::set(&s, "messages", &js_sys::Array::new().into());
    }
    api::delete_session(&id, |_| {
        load_sessions(JsValue::UNDEFINED, js_sys::Array::new());
    });
    JsValue::UNDEFINED
}

/// `send()` — the composer. Streams the turn into the store.
fn send(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let draft = js::get_str(&s, "draft");
    let content = draft.trim().to_string();
    if content.is_empty() || js::get_bool(&s, "waiting") {
        return JsValue::UNDEFINED;
    }

    // Echo the prompt immediately: the turn is already durable server-side,
    // and waiting for the round trip would make the UI feel dead.
    push_message(&s, "user", &content, "");
    js::set_str(&s, "draft", "");
    js::set_bool(&s, "waiting", true);
    clear_stream(&s);
    // Sending is an explicit "show me the bottom", even if the reader had
    // scrolled up a moment ago.
    pin_to_bottom();
    mjs::redraw();

    let payload = js::obj();
    js::set_str(&payload, "tabId", "1");
    js::set_str(&payload, "content", &content);
    let active = js::get_str(&s, "activeId");
    if !active.is_empty() {
        js::set_str(&payload, "sessionId", &active);
    }

    api::chat_stream(
        &payload.into(),
        |row| on_row(&row),
        |err| {
            let s = mjs::store_ref(STORE);
            js::set_bool(&s, "waiting", false);
            if let Some(e) = err {
                push_message(&s, "error", &e, "");
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

/// One NDJSON row from `/api/chat`.
fn on_row(row: &JsValue) {
    let s = mjs::store_ref(STORE);
    match js::get_str(row, "type").as_str() {
        // The server names the session it actually used — adopt it, so a
        // first prompt on a fresh tab binds the tab to its new session.
        "session" => {
            let id = js::get_str(row, "sessionId");
            if !id.is_empty() {
                add_open_id(&id);
                js::set_str(&s, "activeId", &id);
            }
            mjs::redraw();
        }
        // High-frequency: append to one string, never rebuild the list.
        "assistant_delta" => {
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
                "assistant" => {
                    let streamed = js::get_str(&s, "streamText");
                    let final_text = if text.is_empty() { streamed } else { text };
                    if !final_text.trim().is_empty() {
                        push_message(&s, "assistant", &final_text, "");
                    }
                    js::set_str(&s, "streamText", "");
                    js::set_str(&s, "streamHtml", "");
                }
                "reasoning" => {
                    if !text.trim().is_empty() {
                        push_message(&s, "reasoning", &text, "");
                    }
                    js::set_str(&s, "streamReasoning", "");
                    js::set_str(&s, "streamReasoningHtml", "");
                }
                "tool_call" => {
                    push_message(&s, "tool_call", &text, &js::get_str(&card, "name"));
                }
                "tool_result" => {
                    push_message(&s, "tool_result", &text, &js::get_str(&card, "name"));
                }
                "error" => push_message(&s, "error", &text, ""),
                _ => {}
            }
            mjs::redraw();
        }
        "done" => {
            js::set_bool(&s, "waiting", false);
            mjs::redraw();
        }
        _ => {}
    }
}

/// Append one message to the transcript.
fn push_message(s: &JsValue, kind: &str, text: &str, name: &str) {
    let list = js_sys::Array::from(&js::get(s, "messages"));
    let m = js::obj();
    js::set_str(&m, "kind", kind);
    js::set_str(&m, "text", text);
    if !name.is_empty() {
        js::set_str(&m, "name", name);
    }
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

pub fn register() {
    let mut d = Def::new().named("store live");
    d.val("sessions", &js_sys::Array::new().into())
        // `sessions` is every session on disk (the history rail).
        // `tabs` is the subset currently open (the tab strip). Closing a tab
        // must not destroy a session, so these are deliberately distinct.
        .val("tabs", &js_sys::Array::new().into())
        .val("messages", &js_sys::Array::new().into())
        .val("session", &js::obj().into())
        .str("activeId", "")
        .str("draft", "")
        .str("streamText", "")
        .str("streamHtml", "")
        .str("streamReasoningHtml", "")
        .str("streamReasoning", "")
        .str("projectRoot", "")
        .str("allowlist", "")
        .str("allowlistBaseline", "")
        .bool("allowlistOpen", false)
        .bool("allowlistOverridden", false)
        .bool("waiting", false)
        // Sticky scroll: true while the reader is at (or near) the bottom.
        .bool("stickBottom", true)
        .num("promptTokens", 0.0)
        .num("totalTokens", 0.0)
        .method("loadSessions", load_sessions)
        .method("openSession", open_session)
        .method("newSession", new_session)
        .method("closeTab", close_tab)
        .method("send", send)
        .method("relTime", rel_time)
        .method("onScroll", on_scroll)
        .method("keepPinned", keep_pinned)
        .method("deleteSession", delete_session)
        .method("toggleAllowlist", toggle_allowlist)
        .method("saveAllowlist", save_allowlist)
        .method("revertAllowlist", revert_allowlist);
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

/// `revertAllowlist()` — drop the override and fall back to the agent
/// default. Sends `overridden: false` rather than an empty string, which
/// would mean "deny everything".
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


/// Pre-render an assistant message's markdown.
///
/// Done once when the message lands rather than in a template binding: a
/// bound expression is re-evaluated on every redraw, so rendering there would
/// re-parse the whole transcript on each streamed token.
fn add_html(msg: &JsValue) {
    let kind = js::get_str(msg, "kind");
    // Reasoning is prose from the same model and arrives with the same
    // markdown in it, so it gets the same treatment as an answer.
    if kind != "assistant" && kind != "reasoning" {
        return;
    }
    let text = js::get_str(msg, "text");
    if text.is_empty() {
        return;
    }
    js::set_str(msg, "html", &helpers::markdown_to_html(&text));
}

/// `relTime(iso)` — "just now", "5m", "3h", "2d".
///
/// Coarse on purpose: the history rail wants a glanceable age, and a value
/// that changes every second would redraw the list for no reason.
fn rel_time(_this: JsValue, args: js_sys::Array) -> JsValue {
    let iso = js::arg_str(&args, 0);
    if iso.is_empty() {
        return JsValue::from_str("");
    }
    let date = js_sys::Date::new(&JsValue::from_str(&iso));
    let then = date.get_time();
    if then.is_nan() {
        return JsValue::from_str("");
    }
    let now = js_sys::Date::now();
    let secs = ((now - then) / 1000.0).max(0.0);
    let out = if secs < 45.0 {
        "just now".to_string()
    } else if secs < 3600.0 {
        format!("{}m", (secs / 60.0).round().max(1.0) as i64)
    } else if secs < 86_400.0 {
        format!("{}h", (secs / 3600.0).floor() as i64)
    } else {
        format!("{}d", (secs / 86_400.0).floor() as i64)
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
        js::set_str(&s, "activeId", "");
        js::set(&s, "messages", &js_sys::Array::new().into());
        clear_stream(&s);
    }
    mjs::redraw();
    api::delete_session(&id, |_| {
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
}

/// `closeTab(id)` — take it off the strip. The session stays on disk and in
/// the history rail; only the trash button deletes.
fn close_tab(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    remove_open_id(&id);
    let s = mjs::store_ref(STORE);
    rebuild_tabs(&s);

    if js::get_str(&s, "activeId") == id {
        // Fall back to whatever tab is left, so closing the active one does
        // not leave an empty panel beside open tabs.
        let tabs = js_sys::Array::from(&js::get(&s, "tabs"));
        if tabs.length() > 0 {
            let next = js::get_str(&tabs.get(0), "id");
            js::set_str(&s, "activeId", &next);
            open_session_by_id(&next);
        } else {
            js::set_str(&s, "activeId", "");
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

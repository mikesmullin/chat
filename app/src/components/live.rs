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
        js::set(&s, "messages", &msgs.clone().into());
        js::set(&s, "session", &js::get(&v, "session"));
        // Rebuild the session totals from the reloaded transcript, so
        // reopening a session shows what it has cost so far rather than
        // starting from zero.
        store_metrics(&s, &session_metrics(&msgs));
        // Seed the allowlist editor: the session's override if it has one,
        // else the agent default. The baseline is what "overridden" is
        // measured against.
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
        if !overridden {
            js::set_str(&s, "allowlistBaseline", &text);
        }
        clear_stream(&s);
        // The roster and the cursor come back on the same response, so the
        // attribution map is ready before the first row renders.
        store_roster(&s, &js_sys::Array::from(&js::get(&v, "participants")));
        js::set_str(&s, "cursor", &js::get_str(&v, "cursor"));
        mjs::redraw();
        // The tree and the memory view are separate reads: neither is needed
        // to draw the transcript, and asking for both up front would make
        // opening a session wait on work most sessions never show.
        load_branches(&want);
        load_view(JsValue::UNDEFINED, js_sys::Array::new());
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
    // Each session decides for itself whether it is threaded; carrying the
    // last one's answer over would show a flat conversation under a
    // "Threaded" label. `loadView` turns it back on if this one is compacted.
    js::set_bool(&s, "threadView", false);
    js::set(&s, "threads", &js_sys::Array::new().into());
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
                    let final_text = if text.is_empty() { streamed } else { text.clone() };
                    // In a group conversation the card names its speaker;
                    // without the label three agents' bubbles are
                    // indistinguishable.
                    let handle = js::get_str(&card, "handle");
                    // Fold this turn into the running totals. The rates come
                    // from this turn's telemetry; the counts accumulate.
                    let tel = js::get(&card, "telemetry");
                    add_turn_metrics(&s, &tel, &final_text);
                    if !final_text.trim().is_empty() {
                        push_message_as(&s, "assistant", &final_text, "", &handle);
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
            // A new message changes the tree's leaf set, so a threaded view
            // on screen is now one message short.
            if js::get_bool(&s, "threadView") {
                load_view(JsValue::UNDEFINED, js_sys::Array::new());
            }
            mjs::redraw();
        }
        _ => {}
    }
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
        // True while the session runs with `*`. The rules survive it.
        .bool("unrestricted", false)
        .bool("waiting", false)
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
        .bool("threadView", false)
        .bool("compacting", false)
        .num("memCount", 0.0)
        // 0 = the server's default budget. Set it to force threading on a
        // conversation that would otherwise still fit.
        .num("memoryHigh", 0.0)
        // --- the roster -----------------------------------------------------
        .val("participants", &js_sys::Array::new().into())
        // participant id -> handle, so a message can be attributed without
        // searching the roster on every row.
        .val("handles", &js::obj().into())
        .num("agentCount", 0.0)
        .bool("isGroup", false)
        .bool("rosterOpen", false)
        .str("newAgent", "")
        // --- branches -------------------------------------------------------
        .str("cursor", "")
        .val("branches", &js_sys::Array::new().into())
        .bool("hasBranches", false)
        .bool("branchOpen", false)
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
        .method("revertAllowlist", revert_allowlist)
        .method("setPolicy", set_policy)
        .method("loadView", load_view)
        .method("compact", compact)
        .method("toggleThread", toggle_thread)
        .method("toggleThreadView", toggle_thread_view)
        .method("addAgent", add_agent)
        .method("removeAgent", remove_agent)
        .method("forkAt", fork_at)
        .method("switchBranch", switch_branch)
        .method("handleOf", handle_of);
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
        js::set_num(&s, "memCount", js::get(&v, "count").as_f64().unwrap_or(0.0));
        // The threaded view is only worth showing once something has
        // actually been folded; before that it is the flat transcript with
        // extra chrome.
        if js::get_bool(&v, "compacted") {
            js::set_bool(&s, "threadView", true);
        }
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

/// `toggleThreadView()` — flat transcript or threaded history.
fn toggle_thread_view(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let on = !js::get_bool(&s, "threadView");
    js::set_bool(&s, "threadView", on);
    if on {
        load_view(JsValue::UNDEFINED, js_sys::Array::new());
    }
    mjs::redraw();
    JsValue::UNDEFINED
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
    for p in list.iter() {
        let id = js::get(&p, "id").as_f64().unwrap_or(-1.0);
        js::set_str(&by_id, &format!("{id}"), &js::get_str(&p, "handle"));
        if js::get_str(&p, "kind") == "agent" && js::get_bool(&p, "active") {
            agents += 1.0;
        }
    }
    js::set(s, "handles", &by_id);
    js::set_num(s, "agentCount", agents);
    // Two or more agents is what makes this a group conversation, which is
    // what the server uses to pick the group turn path.
    js::set_bool(s, "isGroup", agents > 1.0);
}

/// `addAgent()` — seat another agent from the composer's roster bar.
fn add_agent(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let id = js::get_str(&s, "activeId");
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let name = {
        let typed = js::arg_str(&args, 0);
        if typed.trim().is_empty() {
            js::get_str(&s, "newAgent")
        } else {
            typed
        }
    };
    let name = name.trim().to_string();
    if name.is_empty() {
        return JsValue::UNDEFINED;
    }
    let payload = js::obj();
    js::set_str(&payload, "kind", "agent");
    js::set_str(&payload, "handle", &name);
    js::set_str(&payload, "displayName", &name);
    // Also treated as an agent-definition name, so seating a configured
    // agent picks up its system prompt and model server-side.
    js::set_str(&payload, "agentName", &name);
    let want = id.clone();
    api::add_participant(&id, &payload.into(), move |_| {
        let s = mjs::store_ref(STORE);
        js::set_str(&s, "newAgent", "");
        if js::get_str(&s, "activeId") == want {
            load_roster(&want);
        }
        mjs::redraw();
    });
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

/// `forkAt(eventId)` — point the cursor at an earlier message.
///
/// This is the whole fork operation. The branch you were on is untouched and
/// still reachable; the next message you send simply hangs off the node you
/// chose instead of off the tail.
fn fork_at(_this: JsValue, args: js_sys::Array) -> JsValue {
    let event_id = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    let id = js::get_str(&s, "activeId");
    if id.is_empty() || event_id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let payload = js::obj();
    js::set_str(&payload, "eventId", &event_id);
    let want = id.clone();
    api::set_cursor(&id, &payload.into(), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get_str(&s, "activeId") != want {
            return;
        }
        js::set_str(&s, "cursor", &js::get_str(&v, "cursor"));
        // Re-read rather than trusting the local list: the rendered
        // conversation is now a different path through the tree.
        open_session_by_id(&want);
        load_branches(&want);
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

/// Pull the forest and keep the fork points: nodes with more than one child
/// are where the conversation actually diverged, and they are the only ones
/// worth offering as a branch switch.
fn load_branches(id: &str) {
    let want = id.to_string();
    api::tree(id, move |v| {
        let s = mjs::store_ref(STORE);
        if js::get_str(&s, "activeId") != want {
            return;
        }
        let nodes = js_sys::Array::from(&js::get(&v, "nodes"));
        let on_path = js_sys::Array::from(&js::get(&v, "onPath"));
        let mut counts: std::collections::HashMap<String, Vec<JsValue>> =
            std::collections::HashMap::new();
        for n in nodes.iter() {
            let parent = js::get_str(&n, "parentId");
            if parent.is_empty() {
                continue;
            }
            counts.entry(parent).or_default().push(n);
        }
        let on: Vec<String> = on_path.iter().map(|x| x.as_string().unwrap_or_default()).collect();
        let out = js_sys::Array::new();
        for n in nodes.iter() {
            let nid = js::get_str(&n, "id");
            let kids = match counts.get(&nid) {
                Some(k) if k.len() > 1 => k,
                _ => continue,
            };
            for (i, k) in kids.iter().enumerate() {
                let kid = js::get_str(k, "id");
                let b = js::obj();
                js::set_str(&b, "id", &kid);
                js::set_str(&b, "forkOf", &nid);
                js::set_num(&b, "n", (i + 1) as f64);
                js::set_num(&b, "of", kids.len() as f64);
                js::set_bool(&b, "current", on.contains(&kid));
                // Switching lands on the branch's TIP, not on the node where
                // it split off: a branch chip means "show me that version of
                // the conversation", and stopping at the fork would show one
                // message of it.
                js::set_str(&b, "tip", &branch_tip(k, &counts));
                // Label the branch with the first real message under it,
                // not with the node that happens to sit at the fork. A fork
                // often lands on a `reasoning` or `gen_info` node, and
                // "The user just said hi…" tells you nothing about which
                // branch you would be switching to.
                js::set_str(&b, "text", &helpers::clip(&first_message_under(k, &counts), 60));
                out.push(&b.into());
            }
        }
        js::set(&s, "branches", &out.into());
        js::set_bool(&s, "hasBranches", !counts.values().all(|v| v.len() < 2));
        mjs::redraw();
    });
}

/// `switchBranch(eventId)` — render a different branch from a fork point.
fn switch_branch(this: JsValue, args: js_sys::Array) -> JsValue {
    fork_at(this, args)
}

/// `handleOf(participantId)` — the `@handle` that authored a message.
fn handle_of(_this: JsValue, args: js_sys::Array) -> JsValue {
    let pid = js::arg_num(&args, 0, 0.0);
    let s = mjs::store_ref(STORE);
    let map = js::get(&s, "handles");
    let h = js::get_str(&map, &format!("{pid}"));
    JsValue::from_str(&h)
}


/// The text of the first message at or below `node`, following the eldest
/// child at each step.
///
/// Bounded by a step count rather than a visited set: the tree is acyclic by
/// construction (a parent's `seq` is always smaller than its child's), so the
/// only thing worth guarding against here is a pathologically deep branch.
fn first_message_under(
    node: &JsValue,
    children: &std::collections::HashMap<String, Vec<JsValue>>,
) -> String {
    let mut cur = node.clone();
    for _ in 0..64 {
        if js::get_str(&cur, "kind") == "context_window" {
            let text = js::get_str(&cur, "content");
            if !text.trim().is_empty() {
                return text;
            }
        }
        let id = js::get_str(&cur, "id");
        match children.get(&id).and_then(|k| k.first()) {
            Some(next) => cur = next.clone(),
            None => break,
        }
    }
    // Nothing said on this branch yet: name what is actually there.
    format!("({})", js::get_str(node, "kind"))
}


/// The deepest node on a branch, following the eldest child at each step.
///
/// Bounded by a step count rather than a visited set: the tree is acyclic by
/// construction, so this guards only against a pathologically long branch.
fn branch_tip(
    node: &JsValue,
    children: &std::collections::HashMap<String, Vec<JsValue>>,
) -> String {
    let mut cur = node.clone();
    for _ in 0..4096 {
        let id = js::get_str(&cur, "id");
        match children.get(&id).and_then(|k| k.first()) {
            Some(next) => cur = next.clone(),
            None => return id,
        }
    }
    js::get_str(&cur, "id")
}

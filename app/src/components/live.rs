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
        let meta = js::get(&v, "session");
        let model = js::get_str(&meta, "model");
        if !model.is_empty() {
            js::set_str(&mjs::store_ref("chat"), "model", &model);
            js::set_str(&mjs::store_ref("dropdown"), "model", &model);
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
        mjs::redraw();
        // The tree and the memory view are separate reads: neither is needed
        // to draw the transcript, and asking for both up front would make
        // opening a session wait on work most sessions never show.
        load_branches(&want);
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
    js::set(s, "handles", &js::obj().into());
    js::set(s, "accents", &js::obj().into());
    js::set_num(s, "agentCount", 0.0);
    js::set_bool(s, "isGroup", false);
    js::set_str(s, "cursor", "");
    js::set(s, "branches", &js_sys::Array::new().into());
    js::set_bool(s, "hasBranches", false);
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
    // Likewise the roster: the fetch below replaces it, and until it lands
    // the old session's agents would be shown as this one's.
    clear_roster(&s);
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
            // A new conversation is empty of agents. Without this the
            // previous session's roster stayed on screen — and since
            // nothing re-read it, those agents looked seated in a
            // conversation that had never heard of them.
            clear_roster(&s);
            // …and then read the real one back, so the new session's state
            // comes from the server rather than from what we assumed.
            open_session_by_id(&id);
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
    // A slash command is an instruction to the app, not a message to the
    // conversation — it never reaches a model and never enters the
    // transcript, so it is handled before anything else happens.
    if content.starts_with('/') {
        run_command(&s, &content);
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
    let model = js::get_str(&mjs::store_ref("chat"), "model");
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
                "error" => push_message(&s, "error", &text, ""),
                // The server explaining itself — e.g. that the conversation
                // has no agents in it yet.
                "system" => push_message(&s, "system", &text, ""),
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

/// Append a card from the wire, keeping every field its template reads.
///
/// `text_override` is for the assistant row, whose final text may be what was
/// streamed rather than what the sealed card carries.
fn push_card(s: &JsValue, card: &JsValue, kind: &str, text_override: Option<&str>) {
    let list = js_sys::Array::from(&js::get(s, "messages"));
    let m = js::obj();
    js::set_str(&m, "kind", kind);
    for key in ["text", "handle", "turn", "eventId", "name", "model", "error", "ts"] {
        let v = js::get_str(card, key);
        if !v.is_empty() {
            js::set_str(&m, key, &v);
        }
    }
    if let Some(t) = text_override {
        js::set_str(&m, "text", t);
    }
    for key in ["bytes", "historyTurns", "confidence", "round"] {
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
        // The roster bar's list: everyone except the reader.
        .val("others", &js_sys::Array::new().into())
        // participant id -> handle, so a message can be attributed without
        // searching the roster on every row.
        .val("handles", &js::obj().into())
        // handle -> accent colour, for tinting a row to its author.
        .val("accents", &js::obj().into())
        .num("agentCount", 0.0)
        .bool("isGroup", false)
        .bool("rosterOpen", false)
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
        .method("loadView", load_view)
        .method("compact", compact)
        .method("toggleThread", toggle_thread)
        .method("toggleThreadView", toggle_thread_view)
        .method("removeAgent", remove_agent)
        .method("forkAt", fork_at)
        .method("switchBranch", switch_branch)
        .method("handleOf", handle_of)
        .method("toggleCard", toggle_reasoning)
        .method("accentOf", accent_of);
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
    let chat = mjs::store_ref("chat");
    let dd = mjs::store_ref("dropdown");
    let payload = js::obj();
    js::set_str(&payload, "sessionId", &id);
    js::set_str(&payload, "model", &js::get_str(&chat, "model"));
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

/// `relAgo(iso)` — the same elapsed time as [`rel_time`], read as prose.
///
/// A byline is a sentence fragment ("fish, 24min ago"), not a column of
/// figures, so it gets words rather than the rail's compact `24m`. Both read
/// the same clock; only the rendering differs.
fn rel_ago(_this: JsValue, args: js_sys::Array) -> JsValue {
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
    // Who to show in the roster bar: everyone **except me**. A list that
    // includes the reader is a list of the room, not of who they are
    // talking to — and the one entry they can do nothing about.
    //
    // There is one human for now, so "me" is the `user` seat. When several
    // humans can share a conversation this becomes "the viewer", and the
    // other humans start appearing here, which is the intended shape.
    let others = js_sys::Array::new();
    // handle -> colour, so a row can be tinted without searching the roster
    // on every redraw. Keyed by handle rather than by participant id because
    // a streamed card names its speaker by handle.
    let accents = js::obj();
    for p in list.iter() {
        let id = js::get(&p, "id").as_f64().unwrap_or(-1.0);
        let handle = js::get_str(&p, "handle");
        js::set_str(&by_id, &format!("{id}"), &handle);
        let colour = js::get_str(&p, "accentColor");
        js::set_str(
            &accents,
            &handle,
            if colour.is_empty() { crate::components::agents::DEFAULT_ACCENT } else { &colour },
        );
        let kind = js::get_str(&p, "kind");
        if kind == "agent" {
            if js::get_bool(&p, "active") {
                agents += 1.0;
            }
            others.push(&p);
        }
    }
    js::set(s, "others", &others.into());
    js::set(s, "accents", &accents);
    js::set(s, "handles", &by_id);
    js::set_num(s, "agentCount", agents);
    // Two or more agents is what makes this a group conversation, which is
    // what the server uses to pick the group turn path.
    js::set_bool(s, "isGroup", agents > 1.0);
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


// ===========================================================================
// Slash commands
// ===========================================================================

/// Add a transient notice to the transcript.
///
/// Not persisted on purpose: it is the app answering you, not something
/// anyone said, and a reload should not replay it.
fn notice(s: &JsValue, text: &str) {
    push_message(s, "system", text, "");
    pin_to_bottom();
    mjs::redraw();
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
                "/invite @name — add an agent to this conversation\n\
                 /uninvite @name — remove one\n\
                 /help — this list",
            );
        }
        other => {
            // Left in the box: a typo should not cost you what you typed.
            notice(s, &format!("Unknown command {other}. Try /help."));
        }
    }
}

/// `/invite @a @b` — seat agents in the open conversation.
fn invite_command(s: &JsValue, names: &[String]) {
    let session = js::get_str(s, "activeId");
    if session.is_empty() {
        notice(s, "Open a conversation first, then /invite an agent into it.");
        return;
    }
    if names.is_empty() {
        notice(s, "Usage: /invite @name — the Agents rail lists what you can invite.");
        return;
    }

    // Check against the defined agents before calling: "no agent named X"
    // here is a better answer than a seat pointing at nothing.
    let defined = js_sys::Array::from(&js::get(&mjs::store_ref("agents"), "agents"));
    let known: Vec<String> = defined.iter().map(|a| js::get_str(&a, "name")).collect();
    let seated: Vec<String> = js_sys::Array::from(&js::get(s, "others"))
        .iter()
        .map(|p| js::get_str(&p, "handle"))
        .collect();

    let mut invited: Vec<String> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    let mut already: Vec<String> = Vec::new();
    for name in names {
        if seated.contains(name) {
            already.push(name.clone());
        } else if known.contains(name) {
            invited.push(name.clone());
        } else {
            unknown.push(name.clone());
        }
    }

    let mut said: Vec<String> = Vec::new();
    if !already.is_empty() {
        said.push(format!("already here: @{}", already.join(", @")));
    }
    if !unknown.is_empty() {
        said.push(format!(
            "no agent named @{} — define it with + in the Agents rail",
            unknown.join(", @")
        ));
    }
    if invited.is_empty() {
        if !said.is_empty() {
            notice(s, &said.join("; "));
        }
        return;
    }

    js::set_str(s, "draft", "");
    // One request per agent; the roster is re-read once at the end rather
    // than after each, so the bar does not flicker through intermediate
    // states on a multi-agent invite.
    let total = invited.len();
    let done = std::rc::Rc::new(std::cell::Cell::new(0usize));
    for name in invited.clone() {
        let payload = js::obj();
        js::set_str(&payload, "kind", "agent");
        js::set_str(&payload, "agentName", &name);
        js::set_str(&payload, "handle", &name);
        let session = session.clone();
        let done = std::rc::Rc::clone(&done);
        let invited = invited.clone();
        let said = said.clone();
        api::add_participant(&session.clone(), &payload.into(), move |v| {
            done.set(done.get() + 1);
            let s = mjs::store_ref(STORE);
            let err = js::get_str(&v, "error");
            if !err.is_empty() {
                notice(&s, &err);
            }
            if done.get() < total {
                return;
            }
            load_roster(&session);
            let mut msg = format!("Invited @{}", invited.join(", @"));
            if !said.is_empty() {
                msg.push_str(&format!(" ({})", said.join("; ")));
            }
            notice(&s, &msg);
        });
    }
}

/// `/uninvite @a` — the counterpart, so a mistaken invite is fixable from
/// the same place it was made.
fn uninvite_command(s: &JsValue, names: &[String]) {
    let session = js::get_str(s, "activeId");
    if session.is_empty() || names.is_empty() {
        notice(s, "Usage: /uninvite @name");
        return;
    }
    let seats = js_sys::Array::from(&js::get(s, "others"));
    let mut removed: Vec<String> = Vec::new();
    for name in names {
        let found = seats.iter().find(|p| &js::get_str(p, "handle") == name);
        let Some(p) = found else { continue };
        let pid = js::get(&p, "id").as_f64().unwrap_or(0.0);
        let session = session.clone();
        api::remove_participant(pid, move |_| {
            load_roster(&session);
            mjs::redraw();
        });
        removed.push(name.clone());
    }
    js::set_str(s, "draft", "");
    if removed.is_empty() {
        notice(s, &format!("@{} is not in this conversation.", names.join(", @")));
    } else {
        // An agent that has spoken keeps its seat, greyed: its messages are
        // in the transcript and have to keep an author.
        notice(s, &format!("Removed @{}", removed.join(", @")));
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


/// `accentOf(handle)` — the colour a row is drawn in.
///
/// Always a concrete colour, never empty: an empty custom property makes the
/// declaration that uses it invalid at computed-value time, which falls back
/// to *inherited*, not to the `var()` fallback — so the row would take its
/// neighbour's colour rather than the default.
fn accent_of(_this: JsValue, args: js_sys::Array) -> JsValue {
    let handle = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    let found = js::get_str(&js::get(&s, "accents"), &handle);
    JsValue::from_str(if found.is_empty() {
        crate::components::agents::DEFAULT_ACCENT
    } else {
        &found
    })
}

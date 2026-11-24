//! Talking to angela over HTTP.
//!
//! Reads and mutations go through the kit's JSON `fetch` helper. The chat
//! endpoint is different: it answers **NDJSON over a chunked body**, one row
//! per event, and the whole point is that rows arrive as the model produces
//! them — so it needs a streaming reader rather than `response.json()`.
//!
//! Everything here reaches `fetch`, `ReadableStream` and `TextDecoder`
//! through reflection on `globalThis`, matching the kit's own approach. That
//! keeps `web-sys` out of the dependency list for features we would use once.

use js_sys::{Array, Promise, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

use rustex1_kit::js;

/// `POST /api/abort` — ask the server to stop this session's turn.
///
/// This is the half that actually cancels anything expensive. The turn is
/// blocked inside an HTTP request to the model provider, and the server
/// drops that request when this arrives; closing *our* end of the NDJSON
/// stream would not, because a server waiting on a provider does not
/// notice a closed socket until it next tries to write.
pub fn abort<F: FnMut(JsValue) + 'static>(session_id: &str, then: F) {
    let payload = js::obj();
    js::set_str(&payload, "sessionId", session_id);
    rustex1_kit::api::post_json("/api/abort", &payload.into(), then);
}

/// Answer one. `allow` runs it once, `always` stops asking for this exact
/// call in this conversation, `deny` refuses and tells the model so.
pub fn approve<F: FnMut(JsValue) + 'static>(id: &str, decision: &str, then: F) {
    let payload = js::obj();
    js::set_str(&payload, "id", id);
    js::set_str(&payload, "decision", decision);
    rustex1_kit::api::post_json("/api/approve", &payload.into(), then);
}

/// `GET /api/sessions` — the project tree and the tab bar.
pub fn sessions<F: FnMut(JsValue) + 'static>(then: F) {
    rustex1_kit::api::get("/api/sessions", None, then);
}

/// `GET /api/projects` — the folders conversations are filed under, each
/// with how many are filed directly in it.
pub fn projects<F: FnMut(JsValue) + 'static>(then: F) {
    rustex1_kit::api::get("/api/projects", None, then);
}

/// `POST /api/projects/<action>` — `create`, `rename`, `delete`, `assign`.
///
/// Every one of them answers with the new listing as well as its own
/// result, so the sidebar and the composer's picker redraw from the same
/// response rather than racing a second request.
pub fn project_action<F: FnMut(JsValue) + 'static>(action: &str, payload: &JsValue, then: F) {
    rustex1_kit::api::post_json(&format!("/api/projects/{action}"), payload, then);
}

/// `GET /api/targets` — sandboxes, repos, branches and worktrees in one
/// response.
///
/// One call rather than four: the pickers are read together on every
/// redraw, and four round-trips would let the menus disagree about which
/// checkout exists.
pub fn targets<F: FnMut(JsValue) + 'static>(then: F) {
    rustex1_kit::api::get("/api/targets", None, then);
}

/// `POST /api/targets/<action>` — `repo-add`, `repo-remove`, `branch-add`,
/// `worktree-add`, `worktree-remove`, `sandbox-add`, `sandbox-remove`.
pub fn target_action<F: FnMut(JsValue) + 'static>(action: &str, payload: &JsValue, then: F) {
    rustex1_kit::api::post_json(&format!("/api/targets/{action}"), payload, then);
}

/// `GET /api/automations` — the scheduled prompts.
pub fn automations<F: FnMut(JsValue) + 'static>(then: F) {
    rustex1_kit::api::get("/api/automations", None, then);
}

/// `POST /api/automations/<action>` — `automation-save`, `automation-remove`,
/// `automation-toggle`.
pub fn automation_action<F: FnMut(JsValue) + 'static>(action: &str, payload: &JsValue, then: F) {
    rustex1_kit::api::post_json(&format!("/api/automations/{action}"), payload, then);
}

/// `GET /api/models` — model choices from AGL's user config.
pub fn models<F: FnMut(JsValue) + 'static>(then: F) {
    rustex1_kit::api::get("/api/models", None, then);
}

/// `GET /api/sessions/:id` — `{ session, messages }` for the transcript.
pub fn session<F: FnMut(JsValue) + 'static>(id: &str, then: F) {
    rustex1_kit::api::get(&format!("/api/sessions/{id}"), None, then);
}

/// `POST /api/session/new` — the tab bar's `+`.
pub fn new_session<F: FnMut(JsValue) + 'static>(payload: &JsValue, then: F) {
    rustex1_kit::api::post_json("/api/session/new", payload, then);
}

/// `DELETE /api/sessions/:id` — closing a tab.
pub fn delete_session<F: FnMut(JsValue) + 'static>(id: &str, then: F) {
    rustex1_kit::api::get(&format!("/api/sessions/{id}"), Some("DELETE"), then);
}

/// Delete a session only if it has no events on any branch.
pub fn delete_session_if_empty<F: FnMut(JsValue) + 'static>(id: &str, then: F) {
    let body = js::obj();
    js::set_str(&body, "id", id);
    rustex1_kit::api::post_json("/api/session/delete-if-empty", &body.into(), then);
}

/// `GET /api/sessions/:id/view` — the memory tree's view: the threads and
/// messages covering the whole conversation, oldest first.
pub fn memory_view<F: FnMut(JsValue) + 'static>(id: &str, then: F) {
    rustex1_kit::api::get(&format!("/api/sessions/{id}/view"), None, then);
}

/// `POST /api/sessions/:id/compact` — build the summary tree and fit the view.
pub fn compact<F: FnMut(JsValue) + 'static>(id: &str, payload: &JsValue, then: F) {
    rustex1_kit::api::post_json(&format!("/api/sessions/{id}/compact"), payload, then);
}

/// `GET /api/sessions/:id/zoom?id=&n=` — open one thread, one level.
///
/// One level per request is the point: a deep history is walked only where
/// someone actually looked, so opening the top of a thousand-message
/// conversation costs two lines, not a thousand.
pub fn zoom<F: FnMut(JsValue) + 'static>(session: &str, from: f64, n: f64, then: F) {
    rustex1_kit::api::get(
        &format!("/api/sessions/{session}/zoom?id={from}&n={n}"),
        None,
        then,
    );
}

/// `DELETE /api/participants/:id` — unseat one (deactivated if it has spoken).
pub fn remove_participant<F: FnMut(JsValue) + 'static>(pid: f64, then: F) {
    rustex1_kit::api::get(&format!("/api/participants/{pid}"), Some("DELETE"), then);
}

/// `POST /api/sessions/:id/cursor` — move the cursor, which is how a branch
/// is chosen and how a fork begins.
pub fn set_cursor<F: FnMut(JsValue) + 'static>(id: &str, payload: &JsValue, then: F) {
    rustex1_kit::api::post_json(&format!("/api/sessions/{id}/cursor"), payload, then);
}

/// `GET /api/sessions/:id/tree` — every branch, for the branch picker.
pub fn tree<F: FnMut(JsValue) + 'static>(id: &str, then: F) {
    rustex1_kit::api::get(&format!("/api/sessions/{id}/tree"), None, then);
}

/// `POST /api/chat`, consuming the NDJSON stream row by row.
///
/// `on_row` fires for every complete line as it arrives; `on_end` fires once,
/// whether the stream finished, errored, or the body ended mid-line. A
/// partial trailing line is flushed to `on_row` first, matching the server.
pub fn chat_stream<R, E>(payload: &JsValue, mut on_row: R, mut on_end: E)
where
    R: FnMut(JsValue) + 'static,
    E: FnMut(Option<String>) + 'static,
{
    let payload = payload.clone();
    spawn_local(async move {
        let out = read_stream(&payload, &mut on_row).await;
        match out {
            Ok(()) => on_end(None),
            Err(e) => {
                let msg = js::get_str(&e, "message");
                on_end(Some(if msg.is_empty() {
                    "request failed".to_string()
                } else {
                    msg
                }))
            }
        }
    });
}

async fn read_stream<R>(payload: &JsValue, on_row: &mut R) -> Result<(), JsValue>
where
    R: FnMut(JsValue),
{
    let global = js_sys::global();

    // fetch('/api/chat', { method, headers, body })
    let headers = js::obj();
    js::set_str(&headers, "content-type", "application/json");
    let init = js::obj();
    js::set_str(&init, "method", "POST");
    js::set(&init, "headers", &headers);
    let json = js::get(&global, "JSON");
    let text = js::call(&json, "stringify", &[payload.clone()]);
    js::set(&init, "body", &text);

    let resp = JsFuture::from(Promise::from(js::call(
        &global,
        "fetch",
        &[JsValue::from_str("/api/chat"), init.into()],
    )))
    .await?;

    let ok = js::get(&resp, "ok").as_bool().unwrap_or(false);
    if !ok {
        let status = js::get(&resp, "status").as_f64().unwrap_or(0.0);
        return Err(JsValue::from_str(&format!("HTTP {status}")));
    }

    let body = js::get(&resp, "body");
    if body.is_undefined() || body.is_null() {
        return Err(JsValue::from_str("no response body"));
    }
    let reader = js::call(&body, "getReader", &[]);
    let decoder = js_sys::Reflect::construct(
        &js::get(&global, "TextDecoder").into(),
        &Array::new(),
    )?;

    let mut buf = String::new();
    loop {
        let chunk = JsFuture::from(Promise::from(js::call(&reader, "read", &[]))).await?;
        if js::get(&chunk, "done").as_bool().unwrap_or(false) {
            break;
        }
        let value = js::get(&chunk, "value");
        if value.is_undefined() {
            continue;
        }
        // `{ stream: true }` so a multi-byte character split across chunks is
        // held back rather than mangled.
        let opts = js::obj();
        js::set(&opts, "stream", &JsValue::TRUE);
        let part = js::call(
            &decoder,
            "decode",
            &[Uint8Array::from(value).into(), opts.into()],
        );
        buf.push_str(&part.as_string().unwrap_or_default());

        while let Some(i) = buf.find('\n') {
            let line: String = buf[..i].trim().to_string();
            buf.drain(..=i);
            if line.is_empty() {
                continue;
            }
            if let Some(v) = json_parse(&line) {
                on_row(v);
            }
        }
    }
    // Flush a trailing partial line, as the server does.
    let tail = buf.trim().to_string();
    if !tail.is_empty() {
        if let Some(v) = json_parse(&tail) {
            on_row(v);
        }
    }
    Ok(())
}

/// `JSON.parse`, returning `None` on a malformed row rather than throwing —
/// one bad line must not abort a turn that is otherwise fine.
fn json_parse(text: &str) -> Option<JsValue> {
    let json = js::get(&js_sys::global(), "JSON");
    let v = js::call(&json, "parse", &[JsValue::from_str(text)]);
    if v.is_undefined() || v.is_null() {
        None
    } else {
        Some(v)
    }
}

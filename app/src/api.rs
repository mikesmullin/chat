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

/// `GET /api/sessions` — the history rail and the tab bar.
pub fn sessions<F: FnMut(JsValue) + 'static>(then: F) {
    rustex1_kit::api::get("/api/sessions", None, then);
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
        match read_stream(&payload, &mut on_row).await {
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

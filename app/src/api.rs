use js_sys::{Array, Promise, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

use rustex1_kit::js;

pub fn abort<F: FnMut(JsValue) + 'static>(session_id: &str, then: F) {
    let payload = js::obj();
    js::set_str(&payload, "sessionId", session_id);
    rustex1_kit::api::post_json("/api/abort", &payload.into(), then);
}

pub fn dequeue<F: FnMut(JsValue) + 'static>(session_id: &str, item_id: &str, then: F) {
    let payload = js::obj();
    js::set_str(&payload, "itemId", item_id);
    rustex1_kit::api::post_json(
        &format!("/api/sessions/{session_id}/dequeue"),
        &payload.into(),
        then,
    );
}

pub fn approve<F: FnMut(JsValue) + 'static>(id: &str, decision: &str, then: F) {
    let payload = js::obj();
    js::set_str(&payload, "id", id);
    js::set_str(&payload, "decision", decision);
    rustex1_kit::api::post_json("/api/approve", &payload.into(), then);
}

pub fn sessions<F: FnMut(JsValue) + 'static>(then: F) {
    rustex1_kit::api::get("/api/sessions", None, then);
}

pub fn projects<F: FnMut(JsValue) + 'static>(then: F) {
    rustex1_kit::api::get("/api/projects", None, then);
}

pub fn project_action<F: FnMut(JsValue) + 'static>(action: &str, payload: &JsValue, then: F) {
    rustex1_kit::api::post_json(&format!("/api/projects/{action}"), payload, then);
}

pub fn targets<F: FnMut(JsValue) + 'static>(then: F) {
    rustex1_kit::api::get("/api/targets", None, then);
}

pub fn target_action<F: FnMut(JsValue) + 'static>(action: &str, payload: &JsValue, then: F) {
    rustex1_kit::api::post_json(&format!("/api/targets/{action}"), payload, then);
}

pub fn automations<F: FnMut(JsValue) + 'static>(then: F) {
    rustex1_kit::api::get("/api/automations", None, then);
}

pub fn automation_action<F: FnMut(JsValue) + 'static>(action: &str, payload: &JsValue, then: F) {
    rustex1_kit::api::post_json(&format!("/api/automations/{action}"), payload, then);
}

pub fn models<F: FnMut(JsValue) + 'static>(then: F) {
    rustex1_kit::api::get("/api/models", None, then);
}

pub fn session<F: FnMut(JsValue) + 'static>(id: &str, then: F) {
    rustex1_kit::api::get(&format!("/api/sessions/{id}"), None, then);
}

pub fn new_session<F: FnMut(JsValue) + 'static>(payload: &JsValue, then: F) {
    rustex1_kit::api::post_json("/api/session/new", payload, then);
}

pub fn delete_session<F: FnMut(JsValue) + 'static>(id: &str, then: F) {
    rustex1_kit::api::get(&format!("/api/sessions/{id}"), Some("DELETE"), then);
}

pub fn delete_session_if_empty<F: FnMut(JsValue) + 'static>(id: &str, then: F) {
    let body = js::obj();
    js::set_str(&body, "id", id);
    rustex1_kit::api::post_json("/api/session/delete-if-empty", &body.into(), then);
}

pub fn memory_view<F: FnMut(JsValue) + 'static>(id: &str, then: F) {
    rustex1_kit::api::get(&format!("/api/sessions/{id}/view"), None, then);
}

pub fn compact<F: FnMut(JsValue) + 'static>(id: &str, payload: &JsValue, then: F) {
    rustex1_kit::api::post_json(&format!("/api/sessions/{id}/compact"), payload, then);
}

pub fn zoom<F: FnMut(JsValue) + 'static>(session: &str, from: f64, n: f64, then: F) {
    rustex1_kit::api::get(
        &format!("/api/sessions/{session}/zoom?id={from}&n={n}"),
        None,
        then,
    );
}

pub fn remove_participant<F: FnMut(JsValue) + 'static>(pid: f64, then: F) {
    rustex1_kit::api::get(&format!("/api/participants/{pid}"), Some("DELETE"), then);
}

pub fn set_cursor<F: FnMut(JsValue) + 'static>(id: &str, payload: &JsValue, then: F) {
    rustex1_kit::api::post_json(&format!("/api/sessions/{id}/cursor"), payload, then);
}

pub fn tree<F: FnMut(JsValue) + 'static>(id: &str, then: F) {
    rustex1_kit::api::get(&format!("/api/sessions/{id}/tree"), None, then);
}

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
    let tail = buf.trim().to_string();
    if !tail.is_empty() {
        if let Some(v) = json_parse(&tail) {
            on_row(v);
        }
    }
    Ok(())
}

fn json_parse(text: &str) -> Option<JsValue> {
    let json = js::get(&js_sys::global(), "JSON");
    let v = js::call(&json, "parse", &[JsValue::from_str(text)]);
    if v.is_undefined() || v.is_null() {
        None
    } else {
        Some(v)
    }
}

pub fn vcs_get<F: FnMut(JsValue) + 'static>(action: &str, query: &str, then: F) {
    let url = if query.is_empty() {
        format!("/api/vcs/{action}")
    } else {
        format!("/api/vcs/{action}?{query}")
    };
    rustex1_kit::api::get(&url, None, then);
}

pub fn vcs_post<F: FnMut(JsValue) + 'static>(action: &str, payload: &JsValue, then: F) {
    rustex1_kit::api::post_json(&format!("/api/vcs/{action}"), payload, then);
}

pub fn vcs_delete<F: FnMut(JsValue) + 'static>(action: &str, query: &str, then: F) {
    let url = if query.is_empty() {
        format!("/api/vcs/{action}")
    } else {
        format!("/api/vcs/{action}?{query}")
    };
    rustex1_kit::api::get(&url, Some("DELETE"), then);
}

pub fn jobs_get<F: FnMut(JsValue) + 'static>(action: &str, query: &str, then: F) {
    let url = if query.is_empty() {
        format!("/api/jobs/{action}")
    } else {
        format!("/api/jobs/{action}?{query}")
    };
    rustex1_kit::api::get(&url, None, then);
}

pub fn jobs_post<F: FnMut(JsValue) + 'static>(action: &str, payload: &JsValue, then: F) {
    rustex1_kit::api::post_json(&format!("/api/jobs/{action}"), payload, then);
}

pub fn vcs_text<F: FnMut(JsValue) + 'static>(action: &str, query: &str, then: F) {
    let url = if query.is_empty() {
        format!("/api/vcs/{action}")
    } else {
        format!("/api/vcs/{action}?{query}")
    };
    let mut then = then;
    spawn_local(async move {
        let global = js_sys::global();
        let Ok(response) = JsFuture::from(Promise::from(js::call(
            &global,
            "fetch",
            &[JsValue::from_str(&url)],
        )))
        .await
        else {
            return;
        };
        let body = js::call(&response, "text", &[]);
        let Ok(text) = JsFuture::from(Promise::from(body)).await else {
            return;
        };
        then(text);
    });
}

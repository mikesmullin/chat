use wasm_bindgen::JsValue;

use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::api;
use crate::helpers;

pub const STORE: &str = "live";

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

fn open_session_by_id(id: &str) {
    SESSION_LOADING.with(|v| *v.borrow_mut() = id.to_string());
    apply_cached_approvals();
    let want = id.to_string();
    api::session(id, move |v| {
        let s = mjs::store_ref(STORE);
        if js::get_str(&s, "activeId") != want {
            return;
        }
        let msgs = js_sys::Array::from(&js::get(&v, "messages"));
        for m in msgs.iter() {
            add_html(&m);
        }
        js::set(&s, "messages", &msgs.clone().into());
        js::set(&s, "session", &js::get(&v, "session"));
        let queue = js::get(&v, "queue");
        let empty = empty_queue();
        js::set(
            &s,
            "queue",
            if js::is_missing(&queue) || queue.is_null() {
                &empty
            } else {
                &queue
            },
        );
        let meta = js::get(&v, "session");
        let model = js::get_str(&meta, "model");
        if !model.is_empty() {
            js::set_str(&s, "model", &model);
            js::set_str(&mjs::store_ref("dropdown"), "model", &model);
        }
        js::set_str(&s, "projectId", &js::get_str(&meta, "project"));
        let config = js::get(&meta, "config");
        let reasoning = js::get_str(&config, "reasoningEffort");
        js::set_str(&mjs::store_ref("dropdown"), "level", if reasoning.is_empty() { "medium" } else { &reasoning });
        persist_composer_settings(JsValue::UNDEFINED, js_sys::Array::new());
        store_metrics(&s, &session_metrics(&msgs));
        let text = js::get_str(&v, "allowlist");
        js::set_str(&s, "allowlist", &text);
        let overridden = js::get_bool(&v, "allowlistOverridden");
        js::set_bool(&s, "allowlistOverridden", overridden);
        let unrestricted = js::get_str(&v, "allowlistMode") == "unrestricted";
        js::set_bool(&s, "unrestricted", unrestricted);
        js::set_str(
            &mjs::store_ref("dropdown"),
            "policy",
            if unrestricted { "Unrestricted" } else { "Allowlist" },
        );
        js::set_str(&s, "allowlistBaseline", "");
        clear_stream(&s);
        store_roster(&s, &js_sys::Array::from(&js::get(&v, "participants")));
        js::set_str(&s, "cursor", &js::get_str(&v, "cursor"));
        refresh_browser_sandbox(JsValue::UNDEFINED, js_sys::Array::new());
        SESSION_LOADING.with(|v| v.borrow_mut().clear());
        let turn_done = replay_active_turn(&want);
        mjs::redraw();
        if turn_done {
            open_session_by_id(&want);
            return;
        }
        load_view(JsValue::UNDEFINED, js_sys::Array::new());
    });
}

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

fn empty_queue() -> JsValue {
    let q = js::obj();
    js::set(&q, "items", &js_sys::Array::new().into());
    js::set_str(&q, "reason", "");
    q.into()
}

fn queue_locally(s: &JsValue, content: &str) {
    let queue = js::get(s, "queue");
    let items = js_sys::Array::from(&js::get(&queue, "items"));
    let item = js::obj();
    js::set_str(&item, "id", &format!("local-{}", items.length()));
    js::set_str(&item, "text", content);
    js::set_str(&item, "reason", "waiting to run");
    items.push(&item);
    let next = js::obj();
    js::set(&next, "items", &items.into());
    js::set_str(&next, "reason", "waiting to run");
    js::set(s, "queue", &next.into());
}

pub fn apply_queue(value: JsValue) {
    let s = mjs::store_ref(STORE);
    let active = js::get_str(&s, "activeId");
    let q = js::get(&js::get(&value, "queues"), &active);
    let empty = empty_queue();
    js::set(
        &s,
        "queue",
        if js::is_missing(&q) || q.is_null() {
            &empty
        } else {
            &q
        },
    );
    mjs::redraw();
}

fn dequeue(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let item = js::arg_str(&args, 0);
    let id = js::get_str(&s, "activeId");
    if id.is_empty() || item.is_empty() {
        return JsValue::UNDEFINED;
    }
    let item_id = item.clone();
    crate::api::dequeue(&id, &item, move |v| {
        let s = mjs::store_ref(STORE);
        if js::get(&v, "ok").as_bool() == Some(true) {
            let text = js::get_str(&v, "text");
            if !text.is_empty() {
                let draft = js::get_str(&s, "draft");
                let back = if draft.trim().is_empty() {
                    text
                } else {
                    format!("{text}\n{draft}")
                };
                js::set_str(&s, "draft", &back);
            }
            let queue = js::get(&s, "queue");
            let items = js_sys::Array::new();
            for row in js_sys::Array::from(&js::get(&queue, "items")).iter() {
                if js::get_str(&row, "id") != item_id {
                    items.push(&row);
                }
            }
            let next = js::obj();
            js::set(&next, "items", &items.into());
            js::set_str(&next, "reason", &js::get_str(&queue, "reason"));
            js::set(&s, "queue", &next.into());
        }
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

fn clear_stream(s: &JsValue) {
    STREAM_TEXT.with(|v| *v.borrow_mut() = StreamText::default());
    js::set_str(s, "streamText", "");
    js::set_str(s, "streamReasoning", "");
    js::set_bool(s, "streamReasoningOpen", false);
    js::set_str(s, "streamHandle", "");
}

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
    js::set_bool(&s, "compacted", false);
    js::set(&s, "threads", &js_sys::Array::new().into());
    clear_roster(&s);
    add_open_id(&id);
    rebuild_tabs(&s);
    set_active(&s, &id);
    js::set(&s, "messages", &js_sys::Array::new().into());
    clear_stream(&s);
    pin_to_bottom();
    mjs::redraw();
    open_session_by_id(&id);
    JsValue::UNDEFINED
}

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

fn pin_tab(_this: JsValue, args: js_sys::Array) -> JsValue {
    keep_tab(&js::arg_str(&args, 0));
    JsValue::UNDEFINED
}

fn keep_tab(id: &str) {
    if !id.is_empty() && read_preview_id() == id {
        write_preview_id("");
        js::set_str(&mjs::store_ref(STORE), "previewId", "");
        mjs::redraw();
    }
}

fn new_session(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let payload = js::obj();
    let current = mjs::store_ref(STORE);
    let project = if js::get_str(&current, "activeId").is_empty() { String::new() }
        else { js::get_str(&js::get(&current, "session"), "project") };
    js::set_str(&payload, "project", &project);
    api::new_session(&payload.into(), |v| {
        let s = mjs::store_ref(STORE);
        if js::get(&v, "ok").as_bool() == Some(false) {
            let error = js::get_str(&v, "error");
            notice(&s, &format!("Could not create session: {error}"));
            mjs::redraw();
            return;
        }
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
            clear_roster(&s);
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

fn pick_image(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let doc = js::get(&js::global(), "document");
    let el = js::call(&doc, "querySelector", &[JsValue::from_str(".composer-image-input")]);
    if !el.is_null() && !el.is_undefined() {
        js::call(&el, "click", &[]);
    }
    JsValue::UNDEFINED
}

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

fn send(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let draft = js::get_str(&s, "draft");
    let image = js::get_str(&s, "image");
    let mut content = draft.trim().to_string();
    if !image.is_empty() {
        if !content.is_empty() {
            content.push_str("\n\n");
        }
        content.push_str(&format!("![image]({image})"));
    }
    if content.is_empty() {
        return JsValue::UNDEFINED;
    }
    keep_tab(&js::get_str(&s, "activeId"));
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

    crate::components::composer::reset_history(&s);
    queue_locally(&s, &content);
    js::set_str(&s, "draft", "");
    js::set_str(&s, "image", "");
    js::set_bool(&s, "stopping", false);
    js::set_bool(&s, "stopUnconfirmed", false);
    js::set_bool(&s, "stopPending", false);
    js::set(&s, "approvals", &js_sys::Array::new().into());
    apply_cached_approvals();
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
    let row_boot = rustex1_kit::lifecycle::token();
    let done_boot = row_boot.clone();
    api::chat_stream(
        &payload.into(),
        move |row| {
            if rustex1_kit::lifecycle::am_current_token(&row_boot) { on_owned_row(&row_turn, &row); }
        },
        move |err| {
            if !rustex1_kit::lifecycle::am_current_token(&done_boot) { return; }
            let s = mjs::store_ref(STORE);
            let id = js::get_str(&turn, "sessionId");
            OWN_TURNS.with(|t| t.borrow_mut().retain(|other| !js_sys::Object::is(other, &turn)));
            if js::get_str(&s, "activeId") != id {
                load_sessions(JsValue::UNDEFINED, js_sys::Array::new());
                return;
            }
            let stopped = js::get_bool(&s, "stopping") || js::get_bool(&s, "stopUnconfirmed");
            if stopped && js::get_bool(&s, "waiting") {
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

fn stop(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    if !js::get_bool(&s, "waiting") || js::get_bool(&s, "stopping") {
        return JsValue::UNDEFINED;
    }
    js::set_bool(&s, "stopping", true);
    js::set_bool(&s, "stopUnconfirmed", false);
    let attempt = js::get(&s, "stopAttempt").as_f64().unwrap_or(0.0) + 1.0;
    js::set_num(&s, "stopAttempt", attempt);
    let id = js::get_str(&s, "activeId");
    if id.is_empty() {
        js::set_bool(&s, "stopPending", true);
    } else {
        crate::api::abort(&id, |_| {});
    }
    mjs::redraw();

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

#[derive(Default)]
struct StreamText {
    answer: String,
    reasoning: String,
    answer_html: String,
    reasoning_html: String,
    answer_dirty: bool,
    reasoning_dirty: bool,
    paint_scheduled: bool,
    last_scroll_ms: f64,
}

thread_local! {
    static STREAM_TEXT: std::cell::RefCell<StreamText> = std::cell::RefCell::new(StreamText::default());
}

fn stream_html() -> (String, String) {
    STREAM_TEXT.with(|v| {
        let mut text = v.borrow_mut();
        if text.answer_dirty {
            text.answer_html = helpers::markdown_to_html(&text.answer);
            text.answer_dirty = false;
        }
        if text.reasoning_dirty {
            text.reasoning_html = helpers::markdown_to_html(&text.reasoning);
            text.reasoning_dirty = false;
        }
        (text.answer_html.clone(), text.reasoning_html.clone())
    })
}

fn refresh_stream(_this: JsValue, args: js_sys::Array) -> JsValue {
    let (answer, reasoning) = stream_html();
    crate::components::session::refresh_stream_nodes(&js::arg(&args, 0), &answer, &reasoning);
    JsValue::UNDEFINED
}

fn schedule_stream_paint() {
    let scheduled = STREAM_TEXT.with(|v| {
        let mut text = v.borrow_mut();
        if text.paint_scheduled { true } else { text.paint_scheduled = true; false }
    });
    if scheduled { return; }
    let boot = rustex1_kit::lifecycle::token();
    let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
        if !rustex1_kit::lifecycle::am_current_token(&boot) { return; }
        STREAM_TEXT.with(|v| v.borrow_mut().paint_scheduled = false);
        let (answer, reasoning) = stream_html();
        paint_stream_node("answer", &answer);
        paint_stream_node("reasoning", &reasoning);
        pin_stream_if_due();
    });
    js::call(&js_sys::global(), "requestAnimationFrame", &[callback]);
}

fn paint_stream_node(kind: &str, html: &str) {
    let Some(nodes) = js::query_all(&format!("#home [data-live-stream='{kind}']")) else { return; };
    let count = js::get_num(&nodes, "length") as u32;
    for i in 0..count {
        let node = js_sys::Reflect::get(&nodes, &JsValue::from(i)).unwrap_or(JsValue::UNDEFINED);
        let messages = js::call(&node, "closest", &[JsValue::from_str(".messages")]);
        if js::is_missing(&messages) || js::is_missing(&js::get(&messages, "offsetParent")) { continue; }
        let rect = js::call(&messages, "getBoundingClientRect", &[]);
        if js::get_num(&rect, "height") <= 0.0 { continue; }
        if js::get_str(&node, "innerHTML") != html { js::set_str(&node, "innerHTML", html); }
    }
}

fn pin_stream_if_due() {
    if !js::get_bool(&mjs::store_ref(STORE), "stickBottom") { return; }
    let now = js_sys::Date::now();
    let due = STREAM_TEXT.with(|v| {
        let mut text = v.borrow_mut();
        if now - text.last_scroll_ms < 1500.0 { false } else { text.last_scroll_ms = now; true }
    });
    if !due { return; }
    let Some(nodes) = js::query_all("#home .messages") else { return; };
    for i in 0..js::get_num(&nodes, "length") as u32 {
        let node = js_sys::Reflect::get(&nodes, &JsValue::from(i)).unwrap_or(JsValue::UNDEFINED);
        if js::is_missing(&js::get(&node, "offsetParent")) { continue; }
        let rect = js::call(&node, "getBoundingClientRect", &[]);
        if js::get_num(&rect, "height") > 0.0 {
            js::set(&node, "scrollTop", &js::get(&node, "scrollHeight"));
            break;
        }
    }
}

thread_local! {
    static SESSION_LOADING: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    static OWN_TURNS: std::cell::RefCell<Vec<JsValue>> = const { std::cell::RefCell::new(Vec::new()) };
    static COMPOSER_DRAFTS: std::cell::RefCell<std::collections::HashMap<String, ComposerDraft>> = std::cell::RefCell::new(std::collections::HashMap::new());
    static TURNS: std::cell::RefCell<std::collections::HashMap<String, JsValue>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

pub(crate) fn receive_turn_snapshot(snapshot: JsValue) {
    let id = js::get_str(&snapshot, "sessionId");
    if id.is_empty() { return; }
    let turn_id = js::get_str(&snapshot, "turnId");
    for row in js_sys::Array::from(&js::get(&snapshot, "rows")).iter() {
        if js::get_str(&row, "type") == "done" && !js::get_bool(&row, "dequeued") {
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
    if js::get_str(&row, "type") == "done" && !js::get_bool(&row, "dequeued") {
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

fn replay_active_turn(id: &str) -> bool {
    let turn = OWN_TURNS.with(|t| t.borrow().iter().find(|turn| js::get_str(turn, "sessionId") == id).cloned())
        .or_else(|| TURNS.with(|t| t.borrow().get(id).cloned()));
    let Some(turn) = turn else { return false; };
    let s = mjs::store_ref(STORE);
    let baseline = js_sys::Array::from(&js::get(&turn, "baseline"));
    for row in baseline.iter() { add_html(&row); }
    js::set(&s, "messages", &baseline.into());
    clear_stream(&s);
    let mut done = false;
    let mut started = false;
    for row in js_sys::Array::from(&js::get(&turn, "rows")).iter() {
        done |= js::get_str(&row, "type") == "done";
        let card = js::get(&row, "card");
        started |= js::get_str(&row, "type") == "card" && js::get_str(&card, "kind") == "user";
        on_row(&row);
    }
    js::set_bool(&s, "waiting", started && !done);
    if done { TURNS.with(|t| { t.borrow_mut().remove(id); }); }
    done
}

fn owns_turn(id: &str) -> bool {
    OWN_TURNS.with(|t| t.borrow().iter().any(|turn| js::get_str(turn, "sessionId") == id))
}

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
        "session" => {
            let id = js::get_str(row, "sessionId");
            if !id.is_empty() {
                add_open_id(&id);
                set_active(&s, &id);
                load_roster(&id);
                if js::get_bool(&s, "stopPending") {
                    js::set_bool(&s, "stopPending", false);
                    crate::api::abort(&id, |_| {});
                }
            }
        }
        "speaking" => {
            js::set_str(&s, "streamHandle", &js::get_str(row, "handle"));
        }
        "assistant_delta" | "reasoning_delta" => {
            let reasoning = js::get_str(row, "type") == "reasoning_delta";
            let h = js::get_str(row, "handle");
            if !h.is_empty() && js::get_str(&s, "streamHandle") != h {
                js::set_str(&s, "streamHandle", &h);
            }
            let fragment = js::get_str(row, "text");
            if fragment.is_empty() { return; }
            let first = STREAM_TEXT.with(|v| {
                let mut text = v.borrow_mut();
                let target = if reasoning { &mut text.reasoning } else { &mut text.answer };
                let first = target.is_empty();
                target.push_str(&fragment);
                if reasoning { text.reasoning_dirty = true; } else { text.answer_dirty = true; }
                first
            });
            if first {
                js::set_str(&s, if reasoning { "streamReasoning" } else { "streamText" }, &fragment);
            }
            schedule_stream_paint();
        }
        "usage" => {
            let p = js::get(row, "prompt_tokens").as_f64().unwrap_or(0.0);
            let t = js::get(row, "total_tokens").as_f64().unwrap_or(0.0);
            js::set_num(&s, "promptTokens", p);
            js::set_num(&s, "totalTokens", t);
        }
        "card" => {
            let card = js::get(row, "card");
            let kind = js::get_str(&card, "kind");
            let text = js::get_str(&card, "text");
            match kind.as_str() {
                "assistant" => {
                    let streamed = STREAM_TEXT.with(|v| v.borrow().answer.clone());
                    let final_text = if text.is_empty() { streamed } else { text.clone() };
                    add_turn_metrics(&s, &js::get(&card, "telemetry"), &final_text);
                    if !final_text.trim().is_empty() {
                        push_card(&s, &card, "assistant", Some(&final_text));
                    }
                    STREAM_TEXT.with(|v| { let mut text = v.borrow_mut(); text.answer.clear(); text.answer_html.clear(); });
                    js::set_str(&s, "streamText", "");
                }
                "reasoning" => {
                    if !text.trim().is_empty() {
                        push_card(&s, &card, "reasoning", None);
                    }
                    STREAM_TEXT.with(|v| { let mut text = v.borrow_mut(); text.reasoning.clear(); text.reasoning_html.clear(); });
                    js::set_str(&s, "streamReasoning", "");
                }
                "tool_call" | "tool_result" | "system_prompt" | "agent_prompt"
                | "decision" => push_card(&s, &card, &kind, None),
                "user" => {
                    push_card(&s, &card, "user", None);
                    js::set_bool(&s, "waiting", true);
                }
                "error" => push_card(&s, &card, "error", None),
                "system" => push_message(&s, "system", &text, ""),
                _ => {}
            }
        }
        "done" => {
            js::set_bool(&s, "waiting", false);
            js::set_bool(&s, "stopping", false);
            js::set_bool(&s, "stopUnconfirmed", false);
            js::set_bool(&s, "stopPending", false);
            load_view(JsValue::UNDEFINED, js_sys::Array::new());
        }
        _ => {}
    }
}

fn push_card(s: &JsValue, card: &JsValue, kind: &str, text_override: Option<&str>) {
    let list = js_sys::Array::from(&js::get(s, "messages"));
    let m = js::obj();
    js::set_str(&m, "kind", kind);
    for key in ["text", "handle", "turn", "eventId", "name", "model", "error", "ts", "from"] {
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

fn push_message(s: &JsValue, kind: &str, text: &str, name: &str) {
    push_message_as(s, kind, text, name, "")
}

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
    js::set_str(
        &m,
        "ts",
        &js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default(),
    );
    js::set_str(&m, "id", &format!("local-{}", list.length()));
    add_html(&m.clone().into());
    js::set(&m, "visible", &JsValue::TRUE);
    let next = list.clone();
    next.push(&m.into());
    js::set(s, "messages", &next.into());
}


const MODEL_KEY: &str = "mjs-chat.model";
const DEFAULT_MODEL: &str = "llama-server:default";

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
        .val("tabs", &js_sys::Array::new().into())
        .str("previewId", "")
        .val("messages", &js_sys::Array::new().into())
        .val("session", &js::obj().into())
        .str("activeId", "")
        .str("activeProject", "")
        .bool("notifyEnabled", crate::components::notification_toggle::enabled())
        .val("unreadSessions", &crate::components::notification_toggle::unread())
        .val("sandboxes", &js_sys::Array::new().into())
        .val("browserSandboxes", &js_sys::Array::new().into())
        .str("browserSandboxId", "")
        .str("browserViewerUrl", "")
        .bool("viewerFetching", false)
        .val("sandboxGroups", &js_sys::Array::new().into())
        .val("sandboxOpen", &js::obj().into())
        .str("sboxConfirmId", "")
        .str("sboxError", "")
        .val("projects", &js_sys::Array::new().into())
        .val("worktrees", &js_sys::Array::new().into())
        .val("repoBranches", &js_sys::Array::new().into())
        .val("repos", &js_sys::Array::new().into())
        .val("automations", &js_sys::Array::new().into())
        .bool("sboxUp", false)
        .str("projectId", "")
        .str("draft", "")
        .str("image", "")
        .num("historyPosition", -1.0)
        .str("historyDraft", "")
        .str("streamText", "")
        .bool("streamReasoningOpen", false)
        .str("streamHandle", "")
        .str("streamReasoning", "")
        .str("projectRoot", "")
        .str("allowlist", "")
        .str("allowlistBaseline", "")
        .bool("allowlistOpen", false)
        .bool("allowlistOverridden", false)
        .bool("unrestricted", false)
        .bool("waiting", false)
        .val("queue", &empty_queue())
        .val("approvals", &js_sys::Array::new().into())
        .bool("stickBottom", true)
        .num("promptTokens", 0.0)
        .num("totalTokens", 0.0)
        .str("lastMetrics", "")
        .num("mTps", 0.0)
        .num("mDur", 0.0)
        .num("mIn", 0.0)
        .num("mOut", 0.0)
        .num("mBytes", 0.0)
        .val("threads", &js_sys::Array::new().into())
        .bool("compacted", false)
        .bool("compacting", false)
        .num("memoryHigh", 0.0)
        .val("participants", &js_sys::Array::new().into())
        .val("others", &js_sys::Array::new().into())
        .val("activeOthers", &js_sys::Array::new().into())
        .val("handles", &js::obj().into())
        .val("accents", &js::obj().into())
        .val("icons", &js::obj().into())
        .str("userName", "")
        .num("agentCount", 0.0)
        .bool("isGroup", false)
        .bool("rosterOpen", false)
        .bool("stopping", false)
        .bool("stopUnconfirmed", false)
        .num("stopAttempt", 0.0)
        .bool("stopPending", false)
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
        .method("refreshStream", refresh_stream)
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
        .method("openSessionRow", open_session_row)
        .method("openGroupEditor", crate::components::group_editor::open)
        .method("openTreeFolder", crate::components::group_editor::open_folder)
        .method("refreshBrowserSandbox", refresh_browser_sandbox)
        .method("loadViewerUrl", load_viewer_url)
        .method("pickBrowserSandbox", pick_browser_sandbox)
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
        .method("dequeue", dequeue)
        .method("decideApproval", decide_approval)
        .method("setModel", set_model)
        .method("refreshModels", refresh_models)
        .method("toggleSandboxGroup", crate::components::sandboxes_menu::toggle_group)
        .method("sboxRemove", sbox_remove);
    mjs::store(STORE, &d.finish());
}

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


fn add_html(msg: &JsValue) {
    let kind = js::get_str(msg, "kind");
    if kind != "user" && kind != "assistant" && kind != "reasoning" {
        return;
    }
    let text = js::get_str(msg, "text");
    if text.is_empty() {
        return;
    }
    js::set_str(msg, "html", &helpers::markdown_to_html(&text));
}

pub(crate) fn rel_ago(_this: JsValue, args: js_sys::Array) -> JsValue {
    let iso = js::arg_str(&args, 0);
    if iso.is_empty() {
        return JsValue::from_str("");
    }
    let then = js_sys::Date::new(&JsValue::from_str(&iso)).get_time();
    if then.is_nan() {
        return JsValue::from_str("");
    }
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

fn delete_session(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    crate::components::notification_toggle::begin_delete(&id);
    let keep = js_sys::Array::new();
    for row in js_sys::Array::from(&js::get(&s, "sessions")).iter() {
        if js::get_str(&row, "id") != id {
            keep.push(&row);
        }
    }
    js::set(&s, "sessions", &keep.into());
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

fn close_preview_on_leave(s: &JsValue, next: &str) {
    let preview = read_preview_id();
    if !preview.is_empty() && preview != next && js::get_str(s, "activeId") == preview {
        remove_open_id(&preview);
    }
}

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



const STICK_SLACK_PX: f64 = 48.0;

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
    if js::get_bool(&s, "stickBottom") != at_bottom {
        js::set_bool(&s, "stickBottom", at_bottom);
    }
    if at_bottom {
        let active_id = js::get_str(&s, "activeId");
        if !active_id.is_empty() {
            crate::components::notification_toggle::clear_session_unread(&s, &active_id);
        }
    }
    JsValue::UNDEFINED
}

fn keep_pinned(_this: JsValue, args: js_sys::Array) -> JsValue {
    let el = js::arg(&args, 0);
    if js::is_missing(&el) {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    if !js::get_bool(&s, "stickBottom") {
        return JsValue::UNDEFINED;
    }
    let el2 = el.clone();
    let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
        let h = js::get(&el2, "scrollHeight");
        js::set(&el2, "scrollTop", &h);
    });
    let g = js_sys::global();
    js::call(&g, "requestAnimationFrame", &[cb]);
    JsValue::UNDEFINED
}

fn pin_to_bottom() {
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "stickBottom", true);
}


pub(crate) fn load_projects(_this: JsValue, _args: js_sys::Array) -> JsValue {
    api::projects(|v| {
        apply_projects(&js::get(&v, "projects"));
    });
    JsValue::UNDEFINED
}

fn apply_projects(list: &JsValue) {
    if list.is_undefined() || list.is_null() {
        return;
    }
    js::set(&mjs::store_ref(STORE), "projects", list);
    mjs::redraw();
}

pub(crate) fn load_targets(_this: JsValue, _args: js_sys::Array) -> JsValue {
    api::targets(|v| apply_targets(&v));
    JsValue::UNDEFINED
}

fn load_automations(_this: JsValue, _args: js_sys::Array) -> JsValue {
    api::automations(|v| apply_automations(&v));
    JsValue::UNDEFINED
}

fn apply_automations(v: &JsValue) {
    let s = mjs::store_ref(STORE);
    let list = js::get(v, "automations");
    if !list.is_undefined() && !list.is_null() && !same_json(&js::get(&s, "automations"), &list) {
        js::set(&s, "automations", &list);
        mjs::redraw();
    }
}

fn refresh_browser_sandbox(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let active = js::get_str(&s, "activeId");
    let selected = js::get_str(&s, "browserSandboxId");
    let rows = js_sys::Array::new();
    let mut still = false;
    for r in js_sys::Array::from(&js::get(&s, "sandboxes")).iter() {
        let id = js::get_str(&r, "id");
        if js::get_str(&r, "session") != active
            || !js::get_str(&r, "image").contains("desktop")
            || js::get_str(&r, "state") != "ready"
        {
            continue;
        }
        if id == selected { still = true; }
        let row = js::obj();
        js::set_str(&row, "id", &id);
        js::set_str(&row, "name", &id);
        js::set_str(&row, "icon", "desktop");
        js::set_str(&row, "hint", &js::get_str(&r, "state"));
        rows.push(&row);
    }
    let first = js::get_str(&rows.get(0), "id");
    js::set(&s, "browserSandboxes", &rows.into());
    if !still {
        js::set_str(&s, "browserSandboxId", &first);
        js::set_str(&s, "browserViewerUrl", "");
    }
    if !js::get_str(&s, "browserSandboxId").is_empty()
        && js::get_str(&s, "browserViewerUrl").is_empty()
    {
        load_viewer_url(JsValue::UNDEFINED, js_sys::Array::new());
    }
    JsValue::UNDEFINED
}

fn viewer_url_off(url: &str) -> String {
    let Some((base, hash)) = url.split_once('#') else { return url.to_string() };
    let keep: Vec<&str> = hash.split('&').filter(|p| !p.starts_with("audio=")).collect();
    let mut parts = keep.join("&");
    if !parts.is_empty() { parts.push('&'); }
    parts.push_str("audio=off");
    format!("{base}#{parts}")
}

fn load_viewer_url(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let id = js::get_str(&s, "browserSandboxId");
    if id.is_empty() || js::get_bool(&s, "viewerFetching") {
        return JsValue::UNDEFINED;
    }
    js::set_bool(&s, "viewerFetching", true);
    let want = id.clone();
    rustex1_kit::api::get(&format!("/api/browser/{id}/viewer"), None, move |v| {
        let s = mjs::store_ref(STORE);
        js::set_bool(&s, "viewerFetching", false);
        if js::get_str(&s, "browserSandboxId") != want {
            return;
        }
        let url = js::get_str(&v, "url");
        if !url.is_empty() {
            js::set_str(&s, "browserViewerUrl", &viewer_url_off(&url));
        }
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

fn pick_browser_sandbox(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let id = js::arg_str(&args, 0);
    if id == js::get_str(&s, "browserSandboxId") {
        return JsValue::UNDEFINED;
    }
    js::set_str(&s, "browserSandboxId", &id);
    js::set_str(&s, "browserViewerUrl", "");
    js::call(&mjs::store_ref("picker"), "close", &[]);
    load_viewer_url(JsValue::UNDEFINED, js_sys::Array::new());
    mjs::redraw();
    JsValue::UNDEFINED
}

fn sbox_remove(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    if js::get_str(&s, "sboxConfirmId") != id {
        crate::components::sandboxes_menu::arm_row(STORE, &id);
        return JsValue::UNDEFINED;
    }
    js::set_str(&s, "sboxConfirmId", "");
    js::set_str(&s, "sboxError", "");
    mjs::redraw();
    let payload = js::obj();
    js::set_str(&payload, "id", &id);
    let callback = move |v: JsValue| {
        let s = mjs::store_ref(STORE);
        if js::get(&v, "ok").as_bool().unwrap_or(false) {
            apply_targets(&v);
            return;
        }
        let msg = js::get_str(&v, "error");
        let msg = if msg.trim().is_empty() { "That sandbox could not be destroyed.".to_string() } else { msg };
        crate::components::sandboxes_menu::arm_row(STORE, &id);
        js::set_str(&s, "sboxError", &msg);
        mjs::redraw();
    };
    api::target_action("sandbox-remove", &payload.into(), callback);
    JsValue::UNDEFINED
}

fn apply_targets(v: &JsValue) {
    let s = mjs::store_ref(STORE);
    for key in ["sandboxes", "repos", "repoBranches", "worktrees", "automations"] {
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
    let groups = crate::components::sandboxes_menu::group_rows(&js::get(&s, "sandboxes"));
    if !same_json(&js::get(&s, "sandboxGroups"), &groups) {
        js::set(&s, "sandboxGroups", &groups);
    }
    crate::components::target_form::sync_sandbox_status(&s);
    refresh_browser_sandbox(JsValue::UNDEFINED, js_sys::Array::new());
}

pub(crate) fn apply_sandboxes(value: &JsValue) { apply_targets(value); }

fn save_target(_this: JsValue, args: js_sys::Array) -> JsValue {
    let action = js::arg_str(&args, 0);
    let payload = args.get(1);
    let form = mjs::store_ref("tform");
    let request_id = js::get_num(&form, "requestId");
    js::set_bool(&form, "busy", true);
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
        api::sessions(|sv| {
            let s = mjs::store_ref(STORE);
            js::set(&s, "sessions", &js::get(&sv, "sessions"));
            mjs::redraw();
        });
    });
    JsValue::UNDEFINED
}

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

fn pick_target(_this: JsValue, args: js_sys::Array) -> JsValue {
    let field = match js::arg_str(&args, 0).as_str() {
        "project" => "projectId",
        _ => return JsValue::UNDEFINED,
    };
    let value = js::arg_str(&args, 1);
    let s = mjs::store_ref(STORE);
    js::set_str(&s, field, &value);
    js::call(&mjs::store_ref("picker"), "close", &[]);
    mjs::redraw();
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
    JsValue::UNDEFINED
}

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
    let s = mjs::store_ref(STORE);
    js::set_str(&s, "sboxConfirmId", "");
    js::set_str(&s, "sboxError", "");
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

fn set_policy(_this: JsValue, args: js_sys::Array) -> JsValue {
    let mode = js::arg_str(&args, 0);
    let unrestricted = mode == "Unrestricted";
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "unrestricted", unrestricted);
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
    js::set_str(
        &payload,
        "mode",
        if unrestricted { "unrestricted" } else { "allowlist" },
    );
    rustex1_kit::api::post_json("/api/allowlist", &payload.into(), |_| {});
    JsValue::UNDEFINED
}



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
        m.bytes += js::get_str(&row, "text").len() as f64;
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

fn store_metrics(s: &JsValue, m: &helpers::SessionMetrics) {
    js::set_num(s, "mTps", m.tok_per_sec);
    js::set_num(s, "mDur", m.duration_ms);
    js::set_num(s, "mIn", m.input_tokens);
    js::set_num(s, "mOut", m.output_tokens);
    js::set_num(s, "mBytes", m.bytes);
    js::set_str(s, "lastMetrics", &helpers::metrics_html(m));
}


fn thread_row(part: &JsValue, depth: f64) -> JsValue {
    let row = js::obj();
    let r = js::get_str(part, "ref");
    js::set_str(&row, "ref", &r);
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
    js::set_str(&row, "html", &helpers::markdown_to_html(&text));
    let msg = js::get(part, "message");
    if !msg.is_undefined() && !msg.is_null() {
        add_html(&msg);
        js::set(&row, "message", &msg);
    }
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

fn thread_index(rows: &js_sys::Array, key: &str) -> i32 {
    for (i, r) in rows.iter().enumerate() {
        if js::get_str(&r, "id") == key {
            return i as i32;
        }
    }
    -1
}

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
        let next = js_sys::Array::new();
        for (i, r) in rows.iter().enumerate() {
            next.push(&r);
            if i as i32 == at {
                for k in kids.iter() {
                    next.push(&k);
                }
            }
        }
        js::set(&s, "threads", &next.into());
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

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


fn store_roster(s: &JsValue, list: &js_sys::Array) {
    js::set(s, "participants", &list.clone().into());
    let by_id = js::obj();
    let mut agents = 0.0;
    let others = js_sys::Array::new();
    let active_others = js_sys::Array::new();
    let accents = js::obj();
    let icons = js::obj();
    for p in list.iter() {
        let id = js::get(&p, "id").as_f64().unwrap_or(-1.0);
        let handle = js::get_str(&p, "handle");
        js::set_str(&by_id, &format!("{id}"), &handle);
        let kind = js::get_str(&p, "kind");
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
    js::set_bool(s, "isGroup", agents > 1.0);
}

fn open_session_row(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    let home = mjs::store_ref("home");
    if js::get_str(&s, "activeId") == id {
        if js::get_bool(&mjs::store_ref("groupDraft"), "open") {
            open_session_editor(JsValue::UNDEFINED, js_sys::Array::new());
        } else if js::get_bool(&home, "asideOpen") {
            js::set_bool(&home, "asideOpen", false);
            let draft = mjs::store_ref("sessionDraft");
            js::set_bool(&draft, "open", false);
        } else {
            open_session_editor(JsValue::UNDEFINED, js_sys::Array::new());
        }
        mjs::redraw();
        return JsValue::UNDEFINED;
    }
    preview_session(JsValue::UNDEFINED, args)
}

fn open_session_editor(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let live = mjs::store_ref(STORE);
    let draft = mjs::store_ref("sessionDraft");
    crate::components::aside_panels::close();
    let meta = js::get(&live, "session");
    js::call(&draft, "seed", &[meta, JsValue::from_str(STORE)]);
    js::set(&draft, "inherited", &js::obj().into());
    let id = js::get_str(&live, "activeId");
    let expected = id.clone();
    rustex1_kit::api::get(&format!("/api/sessions/{id}/effective"), None, move |v| {
        if js::get_str(&mjs::store_ref(STORE), "activeId") == expected {
            js::set(&mjs::store_ref("sessionDraft"), "inherited", &js::get(&v, "inherited"));
            mjs::redraw();
        }
    });
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
    js::set_bool(&draft, "open", true);
    js::set_str(&mjs::store_ref("agents"), "selected", "");
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
    mjs::redraw();
    JsValue::UNDEFINED
}

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

fn handle_of(_this: JsValue, args: js_sys::Array) -> JsValue {
    let pid = js::arg_num(&args, 0, 0.0);
    let s = mjs::store_ref(STORE);
    let map = js::get(&s, "handles");
    let h = js::get_str(&map, &format!("{pid}"));
    JsValue::from_str(&h)
}



fn notice(s: &JsValue, text: &str) {
    push_message(s, "system", text, "");
    let rows = js_sys::Array::from(&js::get(s, "messages"));
    if rows.length() > 0 { js::set_bool(&rows.get(rows.length() - 1), "commandNotice", true); }
    pin_to_bottom();
    mjs::redraw();
}

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
            notice(s, &format!("Unknown command {other}. Try /help."));
        }
    }
}

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
    let total = invited.len();
    let results = std::rc::Rc::new(std::cell::RefCell::new((Vec::<String>::new(), Vec::<String>::new())));
    for name in invited.clone() {
        let payload = js::obj();
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



fn toggle_reasoning(_this: JsValue, args: js_sys::Array) -> JsValue {
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
    js::set(&s, "messages", &next.into());
    mjs::redraw();
    JsValue::UNDEFINED
}


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


fn defined_agent_field(handle: &str, field: &str) -> String {
    js_sys::Array::from(&js::get(&mjs::store_ref("agents"), "agents")).iter()
        .find(|agent| js::get_str(agent, "name") == handle)
        .map(|agent| js::get_str(&agent, field))
        .unwrap_or_default()
}

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

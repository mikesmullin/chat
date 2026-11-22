//! Session completion notifications and their shared bell control.
use crate::css;
use rustex1_kit::h::{m, Node};
use rustex1_kit::{js, lifecycle, mjs};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use wasm_bindgen::{prelude::*, JsCast};

const ENABLED_KEY: &str = "chat.notifications.enabled";
const UNREAD_KEY: &str = "chat.notifications.unread";
const SINCE_KEY: &str = "chat.notifications.since";
const SEEN_KEY: &str = "chat.notifications.seen";
const SOUND: &str = "/assets/sfx/idle.wav";
const REMINDER_MS: f64 = 60_000.0;

thread_local! {
    static SEEN: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static DELETED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static TIMER: Cell<u64> = const { Cell::new(0) };
}

pub const CSS: &str = r#"
.notify-toggle {
  width: 26px; height: 26px; padding: 0; border: 0; border-radius: 6px;
  display: inline-flex; align-items: center; justify-content: center;
  background: transparent; color: var(--muted); cursor: pointer;
}
.notify-toggle:hover { color: var(--text); background: var(--panel-2); }
.notify-toggle.is-on { color: var(--mjs-yellow); }
.notify-toggle:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
.notify-toggle .ph { font-size: 16px; }
"#;

/// Same builder for the app and the samples store.
pub fn button(st: &str) -> Node {
    m("button")
        .class("notify-toggle")
        .attr("type", "button")
        .bind("class", &format!("({st}.notifyEnabled ? 'is-on' : '')"))
        .bind(
            "aria-pressed",
            &format!("({st}.notifyEnabled ? 'true' : 'false')"),
        )
        .attr("aria-label", "Turn completion notifications")
        .bind(
            "title",
            &format!(
                "({st}.notifyEnabled ? 'Disable turn notifications' : 'Enable turn notifications')"
            ),
        )
        .on("click", &format!("{st}.toggleNotifications()"))
        .child(Node::icon_dyn(&format!(
            "({st}.notifyEnabled ? 'bell-ringing' : 'bell-slash')"
        )))
}

pub fn register() {
    css::inject("notification-toggle", CSS);
}

fn storage(name: &str) -> JsValue {
    js::get(&js::global(), name)
}
pub fn enabled() -> bool {
    match js::call(
        &storage("localStorage"),
        "getItem",
        &[JsValue::from_str(ENABLED_KEY)],
    )
    .as_string()
    .as_deref()
    {
        // A fresh visitor (nothing in localStorage) is on by default; only
        // an explicit "false" turns it off.
        None => true,
        Some("false") => false,
        Some(_) => true,
    }
}
pub fn unread() -> JsValue {
    let text = js::call(
        &storage("sessionStorage"),
        "getItem",
        &[JsValue::from_str(UNREAD_KEY)],
    )
    .as_string()
    .unwrap_or_default();
    js_sys::JSON::parse(&text)
        .ok()
        .filter(js_sys::Array::is_array)
        .unwrap_or_else(|| js_sys::Array::new().into())
}
fn save_unread(s: &JsValue) {
    if let Ok(text) = js_sys::JSON::stringify(&js::get(s, "unreadSessions")) {
        js::call(
            &storage("sessionStorage"),
            "setItem",
            &[JsValue::from_str(UNREAD_KEY), text.into()],
        );
    }
}

fn audio() -> JsValue {
    let global = js::global();
    let current = js::get(&global, "__chatNotifyAudio");
    if !js::is_missing(&current) {
        return current;
    }
    let ctor = js::get(&global, "Audio");
    let Ok(ctor) = ctor.dyn_into::<js_sys::Function>() else {
        return JsValue::UNDEFINED;
    };
    let args = js_sys::Array::new();
    args.push(&JsValue::from_str(SOUND));
    let Ok(audio) = js_sys::Reflect::construct(&ctor, &args) else {
        return JsValue::UNDEFINED;
    };
    let audio: JsValue = audio.into();
    js::set(&global, "__chatNotifyAudio", &audio);
    audio
}
fn ignore_rejection(promise: &JsValue) {
    if js::is_missing(promise) {
        return;
    }
    let reject =
        Closure::wrap(Box::new(|_err: JsValue| {}) as Box<dyn FnMut(JsValue)>).into_js_value();
    js::call(promise, "catch", &[reject]);
}
/// Prime the media element from the user's click for browsers that require a gesture.
fn prime_audio() {
    let player = audio();
    if js::is_missing(&player) {
        return;
    }
    js::set_bool(&player, "muted", true);
    let promise = js::call(&player, "play", &[]);
    let p = player.clone();
    let ready = Closure::wrap(Box::new(move |_v: JsValue| {
        js::call(&p, "pause", &[]);
        js::set_num(&p, "currentTime", 0.0);
        js::set_bool(&p, "muted", false);
    }) as Box<dyn FnMut(JsValue)>)
    .into_js_value();
    let p = player.clone();
    let failed = Closure::wrap(Box::new(move |_e: JsValue| {
        js::set_bool(&p, "muted", false);
    }) as Box<dyn FnMut(JsValue)>)
    .into_js_value();
    if !js::is_missing(&promise) {
        js::call(&promise, "then", &[ready, failed]);
    }
}
fn chime() {
    let player = audio();
    if js::is_missing(&player) {
        return;
    }
    js::set_bool(&player, "muted", false);
    js::set_num(&player, "currentTime", 0.0);
    ignore_rejection(&js::call(&player, "play", &[]));
}

/// Keep route titles intact when adding/removing the unread count.
fn sync_title() {
    let global = js::global();
    let doc = js::get(&global, "document");
    let current = js::get_str(&doc, "title");
    let last = js::get_str(&global, "__chatNotifyTitle");
    if current != last {
        js::set_str(&global, "__chatNotifyBaseTitle", &current);
    }
    let base = js::get_str(&global, "__chatNotifyBaseTitle");
    let count = js_sys::Array::from(&js::get(&mjs::store_ref("live"), "unreadSessions")).length();
    let title = if count == 0 {
        base
    } else {
        format!("{base} ({count})")
    };
    js::set_str(&global, "__chatNotifyTitle", &title);
    if current != title {
        js::set_str(&doc, "title", &title);
    }
}

fn schedule() {
    let generation = TIMER.with(|t| {
        let n = t.get() + 1;
        t.set(n);
        n
    });
    let s = mjs::store_ref("live");
    if !js::get_bool(&s, "notifyEnabled")
        || js_sys::Array::from(&js::get(&s, "unreadSessions")).length() == 0
    {
        return;
    }
    lifecycle::after(REMINDER_MS, move || {
        if TIMER.with(|t| t.get()) != generation {
            return;
        }
        let s = mjs::store_ref("live");
        if js::get_bool(&s, "notifyEnabled")
            && js_sys::Array::from(&js::get(&s, "unreadSessions")).length() > 0
        {
            chime();
            schedule();
        }
    });
}

pub fn start() {
    if enabled()
        && js::call(
            &storage("sessionStorage"),
            "getItem",
            &[JsValue::from_str(SINCE_KEY)],
        )
        .is_null()
    {
        js::call(
            &storage("sessionStorage"),
            "setItem",
            &[
                JsValue::from_str(SINCE_KEY),
                JsValue::from_str(&js_sys::Date::now().to_string()),
            ],
        );
    }
    sync_title();
    schedule();
    let global = js::global();
    let old = js::get(&global, "__chatNotifyTitleObserver");
    if !js::is_missing(&old) {
        js::call(&old, "disconnect", &[]);
    }
    let ctor = js::get(&global, "MutationObserver");
    let Ok(ctor) = ctor.dyn_into::<js_sys::Function>() else {
        return;
    };
    let token = lifecycle::token();
    let callback = Closure::wrap(Box::new(move |_records: JsValue, _observer: JsValue| {
        if lifecycle::am_current_token(&token) {
            sync_title();
        }
    }) as Box<dyn FnMut(JsValue, JsValue)>)
    .into_js_value();
    let args = js_sys::Array::new();
    args.push(&callback);
    let Ok(observer) = js_sys::Reflect::construct(&ctor, &args) else {
        return;
    };
    let document = js::get(&global, "document");
    let title = js::call(&document, "querySelector", &[JsValue::from_str("title")]);
    if js::is_missing(&title) {
        return;
    }
    let options = js::obj();
    js::set_bool(&options, "childList", true);
    js::set_bool(&options, "subtree", true);
    js::set_bool(&options, "characterData", true);
    js::call(&observer, "observe", &[title, options.into()]);
    js::set(&global, "__chatNotifyTitleObserver", &observer.into());
    attach_composer_focus_listener();
}

/// `focusin` on the active composer's draft box: the reader has grabbed
/// the conversation to type, so that session is being read. Bound at document
/// level (as the tab strip is) because the composer is a long-lived element
/// whose identity can change when the preview/active session swaps; we key
/// off `closest` so only the app's own `#home` composer counts.
fn attach_composer_focus_listener() {
    let global = js::global();
    let doc = js::get(&global, "document");
    let old = js::get(&global, "__chatNotifyComposerFocus");
    if !js::is_missing(&old) {
        js::call(
            &doc,
            "removeEventListener",
            &[JsValue::from_str("focusin"), old],
        );
    }
    let token = lifecycle::token();
    let handler = Closure::wrap(Box::new(move |event: JsValue| {
        if !lifecycle::am_current_token(&token) {
            return;
        }
        let target = js::get(&event, "target");
        if target.is_null() || target.is_undefined() || js::is_missing(&target) {
            return;
        }
        if !js::call(
            &target,
            "closest",
            &[JsValue::from_str("#home .composer .draft-ta")],
        )
        .as_bool()
        .unwrap_or(false)
        {
            return;
        }
        let s = mjs::store_ref("live");
        let active_id = js::get_str(&s, "activeId");
        if !active_id.is_empty() {
            clear_session_unread(&s, &active_id);
        }
    }) as Box<dyn FnMut(JsValue)>).into_js_value();
    js::call(
        &doc,
        "addEventListener",
        &[JsValue::from_str("focusin"), handler.clone()],
    );
    js::set(&global, "__chatNotifyComposerFocus", &handler);
}

pub fn toggle(this: JsValue, _args: js_sys::Array) -> JsValue {
    let next = !js::get_bool(&this, "notifyEnabled");
    js::set_bool(&this, "notifyEnabled", next);
    js::call(
        &storage("localStorage"),
        "setItem",
        &[
            JsValue::from_str(ENABLED_KEY),
            JsValue::from_str(if next { "true" } else { "false" }),
        ],
    );
    if next {
        js::call(
            &storage("sessionStorage"),
            "setItem",
            &[
                JsValue::from_str(SINCE_KEY),
                JsValue::from_str(&js_sys::Date::now().to_string()),
            ],
        );
        prime_audio();
    }
    schedule();
    JsValue::UNDEFINED
}

pub fn mark_read(this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    clear_session_unread(&this, &id);
    JsValue::UNDEFINED
}

/// Remove one session from the unread list (and re-sync). Also the plain
/// Rust path, so `live` can mark a session read the moment it is opened —
/// independent of any DOM click handler. A session you are looking at is by
/// definition not unread.
pub fn clear_session_unread(this: &JsValue, id: &str) {
    let old = js_sys::Array::from(&js::get(this, "unreadSessions"));
    if old.iter().all(|v| v.as_string().as_deref() != Some(id)) {
        return;
    }
    let next = js_sys::Array::new();
    for item in old.iter() {
        if item.as_string().as_deref() != Some(id) {
            next.push(&item);
        }
    }
    if old.length() != next.length() {
        js::set(this, "unreadSessions", &next.into());
        save_unread(this);
        sync_title();
        schedule();
    }
}

/// Prevent a late completion from reviving an unread deleted session.
pub fn begin_delete(id: &str) {
    DELETED.with(|ids| { ids.borrow_mut().insert(id.to_string()); });
    let id = id.to_string();
    // The API callback is not invoked on network errors. Avoid suppressing
    // notifications forever if a deletion request never gets a response.
    lifecycle::after(10_000.0, move || {
        if DELETED.with(|ids| ids.borrow_mut().remove(&id)) {
            crate::api::sessions(crate::components::live::apply_sessions);
        }
    });
}

pub fn finish_delete(this: &JsValue, id: &str, deleted: bool) {
    if deleted {
        clear_session_unread(this, id);
    } else {
        DELETED.with(|ids| { ids.borrow_mut().remove(id); });
    }
}

fn retained_unread(unread: &[String], existing: &HashSet<String>) -> Vec<String> {
    unread.iter().filter(|id| existing.contains(*id)).cloned().collect()
}

/// A complete session list is authoritative, including deletes made elsewhere.
pub fn reconcile_unread(this: &JsValue, sessions: &js_sys::Array) {
    let existing: HashSet<String> = sessions.iter().map(|s| js::get_str(&s, "id")).collect();
    let old = js_sys::Array::from(&js::get(this, "unreadSessions"));
    let ids: Vec<String> = old.iter().filter_map(|id| id.as_string()).collect();
    let retained = retained_unread(&ids, &existing);
    if retained.len() == old.length() as usize { return; }
    let next = js_sys::Array::new();
    for id in retained { next.push(&JsValue::from_str(&id)); }
    js::set(this, "unreadSessions", &next.into());
    save_unread(this);
    sync_title();
    schedule();
}

/// Exactly one alert per SSE turn, even when a snapshot and deltas overlap.
pub fn completed(id: &str, turn_id: &str, completed_at: Option<f64>) {
    if id.is_empty() || turn_id.is_empty() {
        return;
    }
    if DELETED.with(|ids| ids.borrow().contains(id)) { return; }
    let s = mjs::store_ref("live");
    if !js::get_bool(&s, "notifyEnabled") {
        return;
    }
    // A turn can finish before its session-list update arrives. Verify an
    // unknown id instead of reintroducing a deleted session as unread.
    let sessions = js_sys::Array::from(&js::get(&s, "sessions"));
    if !sessions.iter().any(|row| js::get_str(&row, "id") == id) {
        let session_id = id.to_string();
        let turn = turn_id.to_string();
        crate::api::sessions(move |v| {
            let rows = js::get(&v, "sessions");
            if !js_sys::Array::is_array(&rows) { return; }
            let rows = js_sys::Array::from(&rows);
            if rows.iter().any(|row| js::get_str(&row, "id") == session_id) {
                crate::components::live::apply_sessions(v);
                completed(&session_id, &turn, completed_at);
            }
        });
        return;
    }
    if let Some(at) = completed_at {
        let since = js::call(
            &storage("sessionStorage"),
            "getItem",
            &[JsValue::from_str(SINCE_KEY)],
        )
        .as_string()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or_else(js_sys::Date::now);
        if at < since {
            return;
        }
    }
    let saved = js::call(
        &storage("sessionStorage"),
        "getItem",
        &[JsValue::from_str(SEEN_KEY)],
    )
    .as_string()
    .unwrap_or_default();
    if saved.split(',').any(|v| v == turn_id)
        || !SEEN.with(|seen| seen.borrow_mut().insert(turn_id.to_string()))
    {
        return;
    }
    let mut ids: Vec<&str> = saved.split(',').filter(|v| !v.is_empty()).collect();
    ids.push(turn_id);
    if ids.len() > 256 {
        ids.drain(..ids.len() - 256);
    }
    js::call(
        &storage("sessionStorage"),
        "setItem",
        &[
            JsValue::from_str(SEEN_KEY),
            JsValue::from_str(&ids.join(",")),
        ],
    );
    let active_id = js::get_str(&s, "activeId");
    // A turn finishing in the conversation on screen is already visible
    // (it arrives live in the transcript) — only *other* sessions get
    // flagged.
    if !(active_id.is_empty() && active_id == id) {
        let old = js_sys::Array::from(&js::get(&s, "unreadSessions"));
        if !old.iter().any(|v| v.as_string().as_deref() == Some(id)) {
            let next = js_sys::Array::from(&old);
            next.push(&JsValue::from_str(id));
            js::set(&s, "unreadSessions", &next.into());
            save_unread(&s);
            sync_title();
        }
    }
    chime();
    schedule();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconciliation_keeps_only_unread_sessions_that_still_exist() {
        let unread = vec!["deleted".into(), "kept".into(), "also-deleted".into()];
        let existing = HashSet::from(["kept".to_string(), "read".to_string()]);
        assert_eq!(retained_unread(&unread, &existing), vec!["kept"]);
        assert!(retained_unread(&unread, &HashSet::new()).is_empty());
    }
}

//! Subscribe once per wasm boot. EventSource reconnects and receives a fresh snapshot.
use rustex1_kit::{js, lifecycle};
use wasm_bindgen::{prelude::*, JsCast};

thread_local! {
    static SESSIONS: std::cell::RefCell<JsValue> = const { std::cell::RefCell::new(JsValue::UNDEFINED) };
    static APPROVALS: std::cell::RefCell<JsValue> = const { std::cell::RefCell::new(JsValue::UNDEFINED) };
    static SANDBOXES: std::cell::RefCell<JsValue> = const { std::cell::RefCell::new(JsValue::UNDEFINED) };
}

fn session_delta(delta: &JsValue) {
    SESSIONS.with(|slot| {
        let old = slot.borrow().clone();
        let by_id = js::obj();
        for row in js_sys::Array::from(&js::get(&old, "sessions")).iter() {
            js::set(&by_id, &js::get_str(&row, "id"), &row);
        }
        for row in js_sys::Array::from(&js::get(delta, "changed")).iter() {
            js::set(&by_id, &js::get_str(&row, "id"), &row);
        }
        let rows = js_sys::Array::new();
        for id in js_sys::Array::from(&js::get(delta, "order")).iter() {
            if let Some(id) = id.as_string() {
                let row = js::get(&by_id, &id);
                if !js::is_missing(&row) {
                    rows.push(&row);
                }
            }
        }
        let next = js::obj();
        js::set(&next, "sessions", &rows.into());
        js::set(&next, "projectRoot", &js::get(delta, "projectRoot"));
        let next: JsValue = next.into();
        *slot.borrow_mut() = next.clone();
        crate::components::live::apply_sessions(next);
    });
}

fn sandbox_delta(delta: &JsValue) {
    SANDBOXES.with(|slot| {
        let old = slot.borrow().clone();
        let by_id = js::obj();
        for row in js_sys::Array::from(&js::get(&old, "sandboxes")).iter() {
            js::set(&by_id, &js::get_str(&row, "id"), &row);
        }
        for row in js_sys::Array::from(&js::get(delta, "changed")).iter() {
            js::set(&by_id, &js::get_str(&row, "id"), &row);
        }
        let rows = js_sys::Array::new();
        for id in js_sys::Array::from(&js::get(delta, "order")).iter() {
            if let Some(id) = id.as_string() {
                let row = js::get(&by_id, &id);
                if !js::is_missing(&row) { rows.push(&row); }
            }
        }
        let next = js::obj();
        js::set(&next, "sandboxes", &rows.into());
        js::set(&next, "sboxUp", &js::get(delta, "sboxUp"));
        let next: JsValue = next.into();
        *slot.borrow_mut() = next.clone();
        crate::components::live::apply_sandboxes(&next);
    });
}

fn approval_delta(delta: &JsValue) {
    APPROVALS.with(|slot| {
        let previous = slot.borrow().clone();
        let next = js::obj();
        if let Ok(old) = previous.dyn_into::<js_sys::Object>() {
            for key in js_sys::Object::keys(&old).iter() {
                if let Some(key) = key.as_string() {
                    js::set(&next, &key, &js::get(&old, &key));
                }
            }
        }
        for key in js_sys::Array::from(&js::get(delta, "removed")).iter() {
            let _ = js_sys::Reflect::delete_property(&next, &key);
        }
        let changed = js::get(delta, "changed");
        if let Ok(changed) = changed.dyn_into::<js_sys::Object>() {
            for key in js_sys::Object::keys(&changed).iter() {
                if let Some(key) = key.as_string() {
                    js::set(&next, &key, &js::get(&changed, &key));
                }
            }
        }
        let next: JsValue = next.into();
        *slot.borrow_mut() = next.clone();
        crate::components::live::apply_pending(next);
    });
}

pub fn start() {
    let global = js::global();
    let old = js::get(&global, "__chatEvents");
    if !js::is_missing(&old) {
        js::call(&old, "close", &[]);
    }
    let ctor = js::get(&global, "EventSource");
    let Ok(ctor) = ctor.dyn_into::<js_sys::Function>() else {
        return;
    };
    let args = js_sys::Array::new();
    args.push(&JsValue::from_str("/api/events"));
    let Ok(source) = js_sys::Reflect::construct(&ctor, &args) else {
        return;
    };
    let source: JsValue = source.into();
    js::set(&global, "__chatEvents", &source);
    for name in [
        "sessions",
        "sessions-delta",
        "models",
        "approvals",
        "approvals-delta",
        "sandboxes",
        "sandboxes-delta",
        "turn-snapshot",
        "turn-event",
        "turn-finished",
    ] {
        let token = lifecycle::token();
        let callback = Closure::wrap(Box::new(move |event: JsValue| {
            if !lifecycle::am_current_token(&token) {
                return;
            }
            let data = js::get_str(&event, "data");
            if let Ok(value) = js_sys::JSON::parse(&data) {
                match name {
                    "sessions" => {
                        SESSIONS.with(|s| *s.borrow_mut() = value.clone());
                        crate::components::live::apply_sessions(value);
                    }
                    "sessions-delta" => session_delta(&value),
                    "models" => crate::components::live::apply_models(value),
                    "approvals" => {
                        APPROVALS.with(|s| *s.borrow_mut() = value.clone());
                        crate::components::live::apply_pending(value);
                    }
                    "approvals-delta" => approval_delta(&value),
                    "sandboxes" => {
                        SANDBOXES.with(|s| *s.borrow_mut() = value.clone());
                        crate::components::live::apply_sandboxes(&value);
                    }
                    "sandboxes-delta" => sandbox_delta(&value),
                    "turn-snapshot" => crate::components::live::receive_turn_snapshot(value),
                    "turn-event" => crate::components::live::receive_turn_event(value),
                    "turn-finished" => crate::components::notification_toggle::completed(
                        &js::get_str(&value, "sessionId"), &js::get_str(&value, "turnId"), js::get(&value, "completedAt").as_f64()),
                    _ => {}
                }
            }
        }) as Box<dyn FnMut(JsValue)>)
        .into_js_value();
        js::call(
            &source,
            "addEventListener",
            &[JsValue::from_str(name), callback],
        );
    }
}

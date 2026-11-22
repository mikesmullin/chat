//! Global, two-step keyboard shortcuts for the session workspace.

use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::{closure::Closure, JsValue};
use rustex1_kit::{js, lifecycle, mjs};

#[derive(Debug, PartialEq)]
enum Action { None, Prefix, CloseTab, NewTab, DeleteSession, Cancel }

fn key_action(armed: bool, key: &str, ctrl: bool, alt: bool, meta: bool, shift: bool) -> Action {
    if armed {
        if !alt && !meta && !shift {
            match key.to_ascii_lowercase().as_str() {
                "w" => return Action::CloseTab,
                "n" => return Action::NewTab,
                "d" => return Action::DeleteSession,
                _ => {}
            }
        }
        Action::Cancel
    } else if ctrl && !alt && !meta && !shift && key.eq_ignore_ascii_case("s") {
        Action::Prefix
    } else {
        Action::None
    }
}

pub fn start() {
    let global = js::global();
    let doc: JsValue = js::document().into();
    for (event, property, target) in [
        ("keydown", "__chatShortcutKeyListener", &doc),
        ("blur", "__chatShortcutBlurListener", &global),
    ] {
        let old = js::get(&global, property);
        if !js::is_missing(&old) {
            js::call(target, "removeEventListener", &[JsValue::from_str(event), old, JsValue::TRUE]);
        }
    }
    let armed = Rc::new(Cell::new(false));
    let state = armed.clone();
    let token = lifecycle::token();
    let callback = Closure::wrap(Box::new(move |event: JsValue| {
        if !lifecycle::am_current_token(&token) { return; }
        // Session actions belong to the workspace, not the component guide.
        if js::get_str(&js::get(&js::global(), "location"), "pathname") != "/" {
            state.set(false);
            return;
        }
        if js::get_bool(&event, "repeat") || js::get_bool(&event, "isComposing") { return; }
        let action = key_action(
            state.get(), &js::get_str(&event, "key"), js::get_bool(&event, "ctrlKey"),
            js::get_bool(&event, "altKey"), js::get_bool(&event, "metaKey"), js::get_bool(&event, "shiftKey"),
        );
        match action {
            Action::None => return,
            Action::Cancel => { state.set(false); return; }
            Action::Prefix => state.set(true),
            Action::CloseTab | Action::NewTab | Action::DeleteSession => state.set(false),
        }
        js::call(&event, "preventDefault", &[]);
        js::call(&event, "stopImmediatePropagation", &[]);
        let live = mjs::store_ref("live");
        match action {
            Action::CloseTab => {
                let id = js::get_str(&live, "activeId");
                if !id.is_empty() { js::call(&live, "closeTab", &[JsValue::from_str(&id)]); }
            }
            Action::NewTab => { js::call(&live, "newSession", &[]); }
            Action::DeleteSession => {
                let id = js::get_str(&live, "activeId");
                if !id.is_empty() { js::call(&live, "deleteSession", &[JsValue::from_str(&id)]); }
            }
            _ => {}
        }
    }) as Box<dyn FnMut(JsValue)>).into_js_value();
    js::call(&doc, "addEventListener", &[JsValue::from_str("keydown"), callback.clone(), JsValue::TRUE]);
    js::set(&global, "__chatShortcutKeyListener", &callback);
    let blur = Closure::wrap(Box::new(move |_event: JsValue| armed.set(false)) as Box<dyn FnMut(JsValue)>).into_js_value();
    js::call(&global, "addEventListener", &[JsValue::from_str("blur"), blur.clone(), JsValue::TRUE]);
    js::set(&global, "__chatShortcutBlurListener", &blur);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_shortcuts_require_the_prefix_and_cancel_on_other_keys() {
        assert_eq!(key_action(false, "s", true, false, false, false), Action::Prefix);
        assert_eq!(key_action(false, "w", false, false, false, false), Action::None);
        assert_eq!(key_action(true, "w", false, false, false, false), Action::CloseTab);
        assert_eq!(key_action(true, "n", true, false, false, false), Action::NewTab);
        assert_eq!(key_action(true, "d", false, false, false, false), Action::DeleteSession);
        assert_eq!(key_action(true, "x", false, false, false, false), Action::Cancel);
        assert_eq!(key_action(true, "w", false, true, false, false), Action::Cancel);
        assert_eq!(key_action(false, "s", true, false, false, true), Action::None);
    }
}

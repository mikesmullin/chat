//! `identity switcher` — who you are, bottom-left (PLAN44 D4).
//!
//! The avatar-with-presence-dot, the name, and a chevron menu — the shape
//! every messenger's account chip has, because that is what "which account
//! am I" looks like. The menu answers three questions: which account this
//! is, how to see its profile (the Settings panel carries the devices and
//! the admin's shields), and how to leave.
//!
//! The admin's menu also **switches users** (D4): one row per account, and
//! switching mints that account's cookie while the device record keeps the
//! audit line (`masquerade:by:<admin>` — "alice (by Bob)"). In the dev
//! zone (loopback, no cookie) switching is a plain login instead.
//!
//! Non-admins never see the switch list: they get profile and logout, and
//! nothing else — exactly as decided.

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};
use wasm_bindgen::{JsCast, JsValue};

use crate::css;

const STORE: &str = "ident";
const ST: &str = "$store.ident";

pub const CSS: &str = r#"
:is(#sb, #home) .ident { position: relative; }
:is(#sb, #home) .ident-btn {
  display: flex; align-items: center; gap: 8px; width: 100%;
  padding: 6px 8px; border: 0; border-radius: 8px; background: transparent;
  color: var(--text); cursor: pointer; font: 500 12.5px/1.4 var(--sans);
  text-align: left;
}
:is(#sb, #home) .ident-btn:hover { background: var(--panel-2); }
:is(#sb, #home) .ident-avatar {
  position: relative; width: 22px; height: 22px; flex: none;
  border-radius: 99px; background: var(--accent-soft, var(--panel-2));
  display: inline-flex; align-items: center; justify-content: center;
  font: 600 10px/1 var(--sans); color: var(--accent);
}
:is(#sb, #home) .ident-dot {
  position: absolute; right: -1px; bottom: -1px; width: 8px; height: 8px;
  border-radius: 99px; background: var(--ok, #22c55e);
  border: 2px solid var(--panel, #0b0e14);
}
:is(#sb, #home) .ident-name {
  flex: 1 1 auto; min-width: 0; overflow: hidden;
  text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .ident-btn .ph { color: var(--gray-dim); font-size: 12px; }
:is(#sb, #home) .ident-menu {
  position: absolute; left: 0; bottom: calc(100% + 6px); z-index: 30;
  min-width: 190px; padding: 5px;
  background: var(--panel); border: 1px solid var(--line);
  border-radius: 10px; box-shadow: 0 12px 34px rgba(0,0,0,.5);
  display: flex; flex-direction: column; gap: 1px;
}
:is(#sb, #home) .ident-item {
  padding: 6px 9px; border: 0; border-radius: 6px; background: transparent;
  color: var(--text); font: 400 12.5px/1.4 var(--sans); cursor: pointer;
  text-align: left;
}
:is(#sb, #home) .ident-item:hover { background: var(--panel-2); }
:is(#sb, #home) .ident-label {
  padding: 5px 9px 2px; font: 500 10px/1.4 var(--mono);
  letter-spacing: .08em; text-transform: uppercase; color: var(--gray-dim);
}
:is(#sb, #home) .ident-sep {
  height: 1px; margin: 4px 2px; background: var(--line-soft, var(--line));
}

.ident-demo { width: min(560px, 100%); }
.ident-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// The bottom-left account chip and its menu.
pub fn widget() -> Node {
    m("div")
        .class("ident")
        .init(&format!("{ST}.load()"))
        .kids(vec![
            m("button")
                .class("ident-btn")
                .attr("type", "button")
                .on("click", &format!("{ST}.toggle()"))
                .kids(vec![
                    m("span").class("ident-avatar").kids(vec![
                        m("span").text(&format!("{ST}.initial")),
                        m("span").class("ident-dot"),
                    ]),
                    m("span").class("ident-name").text(&format!("{ST}.name")),
                    Node::icon("caret-up-down").class("ph ph-caret-up-down"),
                ]),
            m("div").class("ident-menu").show(&format!("{ST}.open")).kids(vec![
                m("button")
                    .class("ident-item")
                    .attr("type", "button")
                    .on("click", &format!("{ST}.openProfile()"))
                    .child(lit("Profile…")),
                // The switch list is the admin's alone (D4).
                m("div").class("ident-sep").show(&format!("{ST}.admin")),
                m("div")
                    .class("ident-label")
                    .show(&format!("{ST}.admin"))
                    .child(lit("switch to")),
                m("button")
                    .class("ident-item")
                    .attr("type", "button")
                    .show(&format!("{ST}.admin"))
                    .for_each(&format!("u in {ST}.users"))
                    .on("click", &format!("{ST}.switchUser(u.name)"))
                    .bind("class", &format!("(u.name === {ST}.name ? 'ident-item me' : 'ident-item')"))
                    .text("u.name"),
                m("div").class("ident-sep"),
                m("button")
                    .class("ident-item")
                    .attr("type", "button")
                    .show(&format!("{ST}.loggedIn"))
                    .on("click", &format!("{ST}.logout()"))
                    .child(lit("Log out")),
            ]),
        ])
}

/// standalone demo — the chip as the rail pins it.
pub fn demo() -> Node {
    m("div").class("ident-demo").child(widget())
}

pub fn register() {
    css::inject("identity-switcher", CSS);
    let mut d = Def::new().named("store ident");
    d.bool("open", false)
        .bool("loggedIn", false)
        .bool("admin", false)
        .bool("dev", false)
        .str("name", "…")
        .str("initial", "?")
        .num("userId", 0.0)
        .val("users", &js_sys::Array::new().into())
        .val("devices", &js_sys::Array::new().into())
        .val("bans", &js_sys::Array::new().into())
        .str("shields", "up")
        .bool("kernelFirewall", false)
        .method("load", load)
        .method("toggle", toggle)
        .method("openProfile", open_profile)
        .method("switchUser", switch_user)
        .method("logout", logout)
        .method("loadDevices", load_devices)
        .method("revoke", revoke)
        .method("toggleShields", toggle_shields)
        .method("unban", unban);
    mjs::store(STORE, &d.finish());
}

fn redraw() -> JsValue {
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `/api/me` — who this browser is — plus the user list the admin's switch
/// menu needs. The dev zone answers `{user: null, dev: true}` and the chip
/// says so rather than pretending to be someone.
fn load(_this: JsValue, _args: js_sys::Array) -> JsValue {
    rustex1_kit::api::get("/api/me", None, |v| {
        let s = mjs::store_ref(STORE);
        let u = js::get(&v, "user");
        let logged = !u.is_null() && !u.is_undefined();
        js::set_bool(&s, "loggedIn", logged);
        js::set_bool(&s, "dev", js::get_bool(&v, "dev"));
        if logged {
            let name = js::get_str(&u, "name");
            js::set_str(&s, "initial", &name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default());
            js::set_str(&s, "name", &name);
            js::set_num(&s, "userId", js::get_num(&u, "id"));
            js::set_bool(&s, "admin", js::get_bool(&u, "admin"));
        } else {
            js::set_str(&s, "initial", "D");
            js::set_str(&s, "name", "Dev");
            js::set_bool(&s, "admin", true);
        }
        mjs::redraw();
    });
    rustex1_kit::api::get("/api/users", None, |v| {
        let s = mjs::store_ref(STORE);
        js::set(&s, "users", &js::get(&v, "users"));
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

fn toggle(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "open", !js::get_bool(&s, "open"));
    redraw()
}

/// The profile is the Settings panel: it carries the personal fields, and
/// (below) this account's devices and the admin's shields.
fn open_profile(_this: JsValue, _args: js_sys::Array) -> JsValue {
    js::set_bool(&mjs::store_ref(STORE), "open", false);
    // The settings store owns the panel; its `openPanel` does the
    // one-panel-at-a-time dance this store must not duplicate.
    let settings = mjs::store_ref("settings");
    if let Ok(f) = js_sys::Reflect::get(&settings, &"openPanel".into()) {
        if f.is_function() {
            let f: &js_sys::Function = f.unchecked_ref();
            let _ = js_sys::Reflect::apply(f, &settings, &js_sys::Array::new());
        }
    }
    // Refresh what the profile shows — devices and shields move between
    // opens, and a list read once at boot is a list that lies.
    load_devices(JsValue::UNDEFINED, js_sys::Array::new());
    redraw()
}

/// Switch to another account (D4): the admin masquerades — the new cookie
/// is the target's, the device record keeps "by whom" — while the dev zone
/// simply logs in as that name. Reloads so every panel re-reads identity.
fn switch_user(_this: JsValue, args: js_sys::Array) -> JsValue {
    let name = js::arg_str(&args, 0);
    let s = mjs::store_ref(STORE);
    let dev = js::get_bool(&s, "dev");
    let path = if dev { "/login" } else { "/masquerade" };
    let body = js::obj();
    js::set_str(&body, "name", &name);
    js::set_bool(&s, "open", false);
    rustex1_kit::api::post_json(path, &body.into(), |_| {
        if let Some(w) = web_sys::window() {
            let _ = w.location().reload();
        }
    });
    JsValue::UNDEFINED
}

fn logout(_this: JsValue, _args: js_sys::Array) -> JsValue {
    js::set_bool(&mjs::store_ref(STORE), "open", false);
    rustex1_kit::api::post_json("/logout", &js::obj().into(), |_| {
        if let Some(w) = web_sys::window() {
            let _ = w.location().reload();
        }
    });
    JsValue::UNDEFINED
}

/// The devices list (PLAN44 A2/D10): every recognized login, its cookie
/// shown masked — the value is never stored — and a trash button that
/// force-expires it.
fn load_devices(_this: JsValue, _args: js_sys::Array) -> JsValue {
    rustex1_kit::api::get("/api/devices", None, |v| {
        let s = mjs::store_ref(STORE);
        js::set(&s, "devices", &js::get(&v, "devices"));
        mjs::redraw();
    });
    // Admin extras for the profile's shield row.
    rustex1_kit::api::get("/api/admin/perimeter", None, |v| {
        if js::get_bool(&v, "ok") {
            let s = mjs::store_ref(STORE);
            js::set_str(&s, "shields", &js::get_str(&v, "shields"));
            js::set(&s, "bans", &js::get(&v, "bans"));
            js::set_bool(&s, "kernelFirewall", js::get_bool(&v, "kernelFirewall"));
            mjs::redraw();
        }
    });
    JsValue::UNDEFINED
}

/// Trash one device: its cookie stops validating at once.
fn revoke(_this: JsValue, args: js_sys::Array) -> JsValue {
    let body = js::obj();
    js::set_num(&body, "deviceId", js::arg(&args, 0).as_f64().unwrap_or(0.0));
    rustex1_kit::api::post_json("/api/devices/revoke", &body.into(), |_| {
        load_devices(JsValue::UNDEFINED, js_sys::Array::new());
    });
    JsValue::UNDEFINED
}

/// The admin's Shields Up/Down button (D12).
fn toggle_shields(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let down = js::get_str(&s, "shields") == "up";
    let body = js::obj();
    js::set_bool(&body, "down", down);
    rustex1_kit::api::post_json("/api/admin/shields", &body.into(), |_| {
        load_devices(JsValue::UNDEFINED, js_sys::Array::new());
    });
    JsValue::UNDEFINED
}

fn unban(_this: JsValue, args: js_sys::Array) -> JsValue {
    let body = js::obj();
    js::set_str(&body, "ip", &js::arg_str(&args, 0));
    rustex1_kit::api::post_json("/api/admin/bans", &body.into(), |_| {
        load_devices(JsValue::UNDEFINED, js_sys::Array::new());
    });
    JsValue::UNDEFINED
}

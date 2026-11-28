use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};
use wasm_bindgen::{JsCast, JsValue};

use crate::css;

const STORE: &str = "share";
const ST: &str = "$store.share";

const PRESETS: &[(&str, &[&str])] = &[
    ("none", &[]),
    ("viewer", &["read"]),
    ("chatter", &["read", "post"]),
    ("operator", &["read", "post", "steer", "seat", "edit_meta"]),
    ("editor", &["read", "post", "steer", "seat", "edit_meta", "edit"]),
];

pub const CSS: &str = r#"
:is(#sb, #home) .share-panel {
  display: flex; flex-direction: column; gap: 14px;
  border-top: 1px solid #363636;
  margin: 14px 0;
  padding-top: 14px;
}
:is(#sb, #home) .share-head { display: flex; align-items: center; gap: 8px; }
:is(#sb, #home) .share-head .ph { color: var(--gray-dim); }
:is(#sb, #home) .share-title { font: 600 13px/1.4 var(--sans); }
:is(#sb, #home) .share-name {
  font: 400 11px/1.5 var(--mono); color: var(--gray-dim);
  overflow-wrap: anywhere;
}
:is(#sb, #home) .share-rows { display: flex; flex-direction: column; gap: 6px; }
:is(#sb, #home) .share-row {
  display: flex; align-items: center; gap: 8px;
}
:is(#sb, #home) .share-who {
  flex: 1 1 auto; min-width: 0; overflow: hidden;
  text-overflow: ellipsis; white-space: nowrap;
  font: 500 12px/1.4 var(--sans);
}
:is(#sb, #home) .share-role {
  padding: 4px 6px; border: 1px solid var(--line); border-radius: 6px;
  background: var(--panel-2); color: var(--text); font: 400 11px/1.4 var(--sans);
}
:is(#sb, #home) .share-add {
  display: flex; align-items: center; gap: 8px;
  padding-top: 8px; border-top: 1px solid var(--line-soft, var(--line));
}
:is(#sb, #home) .share-owner { font: 400 11px/1.4 var(--mono); color: var(--gray-dim); }
:is(#sb, #home) .share-err { color: var(--danger, #f87171); font-size: 11px; min-height: 14px; }

.share-demo { width: min(560px, 100%); }
.share-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub fn panel() -> Node {
    m("div")
        .class("share-panel")
        .show("$store.ident.panelOpen")
        .kids(vec![
            m("div").class("share-head").kids(vec![
                Node::icon("users-three").class("ph ph-users-three"),
                m("span").class("share-title").child(lit("Share")),
                m("span").class("share-name").text(&format!("{ST}.name")),
            ]),
            m("div").class("share-rows").kids(vec![
                m("div").class("share-row").kids(vec![
                    m("span").class("share-who").text(&format!("{ST}.ownerName")),
                    m("span").class("share-owner").child(lit("owner")),
                ]),
                m("div")
                    .class("share-row")
                    .for_each(&format!("g in {ST}.rows"))
                    .kids(vec![
                        m("span").class("share-who").text("g.name"),
                        m("select")
                            .class("share-role")
                            .model("g.preset")
                            .on("change", &format!("{ST}.setPreset(g.name)"))
                            .kids(vec![
                                m("option").attr("value", "none").child(lit("none")),
                                m("option").attr("value", "viewer").child(lit("viewer")),
                                m("option").attr("value", "chatter").child(lit("chatter")),
                                m("option").attr("value", "operator").child(lit("operator")),
                                m("option").attr("value", "editor").child(lit("editor")),
                            ]),
                    ]),
            ]),
            m("div").class("share-add").kids(vec![
                m("select")
                    .class("share-role")
                    .model(&format!("{ST}.who"))
                    .kids(vec![
                        m("option").attr("value", "").child(lit("invite someone…")),
                        m("option")
                            .for_each(&format!("u in {ST}.users"))
                            .attr("value", "u.name")
                            .text("u.name"),
                    ]),
                m("select")
                    .class("share-role")
                    .model(&format!("{ST}.preset"))
                    .kids(vec![
                        m("option").attr("value", "viewer").child(lit("viewer")),
                        m("option").attr("value", "chatter").child(lit("chatter")),
                        m("option").attr("value", "operator").child(lit("operator")),
                        m("option").attr("value", "editor").child(lit("editor")),
                    ]),
                m("button")
                    .class("ae-btn")
                    .attr("type", "button")
                    .on("click", &format!("{ST}.invite()"))
                    .child(lit("Share")),
            ]),
            m("p").class("share-err").text(&format!("{ST}.error")),
        ])
}

pub fn demo() -> Node {
    let s = mjs::store_ref(STORE);
    js::set_str(&s, "name", "session · demo-room");
    js::set_str(&s, "ownerName", "Bob");
    let rows = js_sys::Array::new();
    for (name, preset) in [("Alice", "chatter"), ("kid", "none")] {
        let row = js::obj();
        js::set_str(&row, "name", name);
        js::set_str(&row, "preset", preset);
        rows.push(&row);
    }
    js::set(&s, "rows", &rows.into());
    let users = js_sys::Array::new();
    for name in ["Bob", "Alice", "kid"] {
        let u = js::obj();
        js::set_str(&u, "name", name);
        users.push(&u);
    }
    js::set(&s, "users", &users.into());
    m("div").class("share-demo").child(panel())
}

pub fn register() {
    css::inject("share-dialog", CSS);
    let mut d = Def::new().named("store share");
    d.bool("open", false)
        .str("kind", "session")
        .str("name", "")
        .str("ownerName", "")
        .str("error", "")
        .str("who", "")
        .str("preset", "chatter")
        .val("rows", &js_sys::Array::new().into())
        .val("users", &js_sys::Array::new().into())
        .method("openForActive", open_for_active)
        .method("open", open)
        .method("close", close)
        .method("load", load)
        .method("setPreset", set_preset)
        .method("invite", invite);
    mjs::store(STORE, &d.finish());
}

fn redraw() -> JsValue {
    mjs::redraw();
    JsValue::UNDEFINED
}

fn open_for_active(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let live = mjs::store_ref("live");
    let id = js::get_str(&live, "activeId");
    if id.is_empty() {
        return JsValue::UNDEFINED;
    }
    open(_this, {
        let a = js_sys::Array::new();
        a.push(&JsValue::from_str("session"));
        a.push(&JsValue::from_str(&id));
        a
    })
}

fn open(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_str(&s, "kind", &js::arg_str(&args, 0));
    js::set_str(&s, "name", &js::arg_str(&args, 1));
    js::set_str(&s, "error", "");
    crate::components::aside_panels::close();
    js::set_str(&mjs::store_ref("agents"), "selected", "");
    js::set_bool(&mjs::store_ref("ident"), "panelOpen", true);
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
    load(JsValue::UNDEFINED, js_sys::Array::new());
    redraw()
}

fn close(_this: JsValue, _args: js_sys::Array) -> JsValue {
    js::set_bool(&mjs::store_ref(STORE), "open", false);
    redraw()
}

fn load(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let kind = js::get_str(&s, "kind");
    let name = js::get_str(&s, "name");
    rustex1_kit::api::get("/api/users", None, |v| {
        let s = mjs::store_ref(STORE);
        let rows = js_sys::Array::new();
        for u in js_sys::Array::from(&js::get(&v, "users")).iter() {
            let row = js::obj();
            js::set_str(&row, "name", &js::get_str(&u, "name"));
            js::set_str(&row, "preset", "none");
            rows.push(&row);
        }
        js::set(&s, "users", &js::get(&v, "users"));
        js::set(&s, "rows", &rows.into());
        mjs::redraw();
    });
    let url = format!("/api/acl?kind={kind}&name={name}");
    rustex1_kit::api::get(&url, None, |v| {
        let s = mjs::store_ref(STORE);
        let owner_id = js::get(&v, "ownerId").as_f64().unwrap_or(0.0);
        let mut owner_name = String::from("unknown");
        for u in js_sys::Array::from(&js::get(&s, "users")).iter() {
            if js::get_num(&u, "id") == owner_id {
                owner_name = js::get_str(&u, "name");
                break;
            }
        }
        js::set_str(&s, "ownerName", &owner_name);
        let mut by_user: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        for g in js_sys::Array::from(&js::get(&v, "grants")).iter() {
            by_user
                .entry(js::get_str(&g, "granteeName").to_lowercase())
                .or_default()
                .push(js::get_str(&g, "privilege"));
        }
        let rows = js_sys::Array::from(&js::get(&s, "rows"));
        let kept = js_sys::Array::new();
        for row in rows.iter() {
            let name = js::get_str(&row, "name");
            if name == owner_name {
                continue;
            }
            if let Some(p) = by_user.get(&name.to_lowercase()) {
                let preset = PRESETS
                    .iter()
                    .rev()
                    .find(|(_, want)| want.iter().all(|x| p.contains(&x.to_string())))
                    .map(|(n, _)| *n)
                    .unwrap_or("none");
                js::set_str(&row, "preset", preset);
            }
            kept.push(&row);
        }
        js::set(&s, "rows", &kept.into());
        mjs::redraw();
    });
    JsValue::UNDEFINED
}

fn set_preset(_this: JsValue, args: js_sys::Array) -> JsValue {
    let name = js::arg_str(&args, 0);
    let preset = if args.length() > 1 {
        js::arg_str(&args, 1)
    } else {
        preset_from_dom(&name)
    };
    let s = mjs::store_ref(STORE);
    let kind = js::get_str(&s, "kind");
    let res = js::get_str(&s, "name");
    js::set_str(&s, "error", "");
    let bundle = PRESETS
        .iter()
        .find(|(n, _)| *n == preset)
        .map(|(_, v)| *v)
        .unwrap_or(&[]);
    for p in [
        "read", "post", "steer", "seat", "edit_meta", "edit", "grant", "delete",
    ] {
        let body = js::obj();
        js::set_str(&body, "kind", &kind);
        js::set_str(&body, "name", &res);
        js::set_str(&body, "grantee", &name);
        js::set_str(&body, "privilege", p);
        rustex1_kit::api::post_json("/api/acl/revoke", &body.into(), |_| {});
    }
    for p in bundle {
        let body = js::obj();
        js::set_str(&body, "kind", &kind);
        js::set_str(&body, "name", &res);
        js::set_str(&body, "grantee", &name);
        js::set_str(&body, "privilege", p);
        rustex1_kit::api::post_json("/api/acl/grant", &body.into(), move |v| {
            if !js::get_bool(&v, "ok") {
                crate::components::validation_info::note("share", "error", &js::get_str(&v, "error"));
                mjs::redraw();
            }
        });
    }
    JsValue::UNDEFINED
}

fn preset_from_dom(name: &str) -> String {
    let store = mjs::store_ref(STORE);
    let names: Vec<String> = js_sys::Array::from(&js::get(&store, "rows"))
        .iter()
        .map(|r| js::get_str(&r, "name"))
        .collect();
    let Some(idx) = names.iter().position(|n| n == name) else {
        return "none".into();
    };
    let doc = js::get(&js::global(), "document");
    let f = js::get(&doc, "querySelectorAll");
    let Ok(f) = f.dyn_into::<js_sys::Function>() else {
        return "none".into();
    };
    let list = js_sys::Reflect::apply(
        &f,
        &doc,
        &js_sys::Array::of1(&JsValue::from_str(".share-rows select")),
    )
    .ok()
    .map(|v| js_sys::Array::from(&v))
    .unwrap_or_default();
    let sel = list.get(idx as u32);
    if sel.is_undefined() || sel.is_null() {
        return "none".into();
    }
    js::get_str(&sel, "value")
}

fn invite(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let who = js::get_str(&s, "who");
    if who.is_empty() {
        crate::components::validation_info::note("share", "error", "pick someone first");
        return redraw();
    }
    let preset = js::get_str(&s, "preset");
    set_preset(
        JsValue::UNDEFINED,
        {
            let a = js_sys::Array::new();
            a.push(&JsValue::from_str(&who));
            a.push(&JsValue::from_str(&preset));
            a
        },
    )
}

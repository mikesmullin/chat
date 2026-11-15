//! The one store: everything the chat UI reads, and the fetches that fill it.
//!
//! Rust port of `m-js-cmp-chat/src/store.js` (`createChatStore` + `wsTransport`
//! + `applyIncoming`), trimmed to the single-session HTTP contract this app
//! speaks first:
//!
//!   * `m-js-cmp-chat` (JS) talks WS (`wsTransport`) to the agent-desktop
//!     harness and owns the full transcript machine (deltas, cards, todos,
//!     allowlist, pie, tabs, history, goto/visibility).
//!   * this store keeps the same *fields and method names* so the Rust tree in
//!     `pages.rs` (a port of `templates.js`) reads identically, but the wire
//!     is angela's HTTP API (`POST /api/chat` NDJSON, `GET /api/agents`,
//!     `GET /api/sessions`) via the local `/api/*` forward — no WS yet.
//!   * when angela is down, `send()` falls back to the demo echo transport
//!     from `m-js-cmp-chat/demo/app.js`, so the UI is still demonstrable.
//!
//! Like sbox-a, nothing here deserializes into Rust structs: angela answers
//! JSON, `api::get` hands back a `JsValue`, and that value goes straight into
//! the store for m-js to read. A serde round trip would buy nothing.

use js_sys::Array;
use crate::api as chat_api;
use rustex1_kit::{api, js, mjs, mjs::Def};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// The one store. Named `chat` to match the `m-js-cmp-chat` default.
pub const STORE: &str = "chat";

const MODEL_KEY: &str = "mjs-chat.model";
const DEFAULT_MODEL: &str = "llama-server:default";

pub fn register() {
    let mut d = Def::new();
    d
        // ── transcript ─────────────────────────────────────────────
        .val("messages", &js::new_array().into())
        .bool("waiting", false)
        .bool("connected", true)
        .str("draft", "")
        .str("error", "")
        // ── identity ─────────────────────────────────────────────
        .str("model", &saved_model())
        .val("models", &default_models())
        .val("agents", &js::new_array().into())
        .str("agent", "lucy")
        .str("sessionId", "")
        .str("tabId", "1")
        // ── allowlist ────────────────────────────────────────────
        .str("allowlist", "")
        .str("allowlistBaseline", "")
        .bool("allowlistOverridden", false)
        .bool("allowlistOpen", false)
        .bool("allowlistInited", false)
        // ── tasks / context ──────────────────────────────────────
        .val("todos", &js::new_array().into())
        .bool("todosOpen", false)
        .num("contextWindow", 32768.0)
        .num("tokensUsed", 0.0)
        .str("elapsed", "")
        .str("statusLabel", "ready")
        .str("statusClass", "ready")
        // ── tabs / history ───────────────────────────────────────
        .val("tabList", &{
            let a = Array::new();
            let t = js::obj();
            js::set_num(&t, "id", 1.0);
            js::set_str(&t, "name", "Chat 1");
            a.push(&t.into());
            a
        }.into())
        .num("activeTabId", 1.0)
        .val("historyList", &js::new_array().into())
        .str("activeSessionId", "")
        .bool("historyOpen", true)
        .bool("sideOpen", true)
        .bool("isFullscreen", false)
        .val("welcomeStarters", &js::new_array().into())
        // ── scroll ───────────────────────────────────────────────
        .bool("_stick", true);

    getters(&mut d);
    methods(&mut d);
    mjs::store(STORE, &d.finish());
}

fn store_ref() -> JsValue {
    mjs::store_ref(STORE)
}

fn saved_model() -> String {
    // localStorage wins (JS createChatStore parity); empty/missing falls back
    // to the default so the model select is never an empty option.
    let g = js::global();
    let ls = js::get(&g, "localStorage");
    if !ls.is_undefined() && !ls.is_null() {
        let v = js::call(&ls, "getItem", &[JsValue::from_str(MODEL_KEY)]);
        if let Some(s) = v.as_string() {
            if !s.trim().is_empty() {
                return s;
            }
        }
    }
    DEFAULT_MODEL.to_string()
}

fn default_models() -> JsValue {
    let a = Array::new();
    a.push(&JsValue::from_str(DEFAULT_MODEL));
    a.into()
}

fn save_model(m: &str) {
    if m.trim().is_empty() {
        return;
    }
    let g = js::global();
    let ls = js::get(&g, "localStorage");
    if ls.is_undefined() || ls.is_null() {
        return;
    }
    js::call(&ls, "setItem", &[JsValue::from_str(MODEL_KEY), JsValue::from_str(m)]);
}

fn new_id() -> String {
    let g = js::global();
    let crypto = js::get(&g, "crypto");
    if !crypto.is_undefined() {
        let id = js::call(&crypto, "randomUUID", &[]);
        if let Some(s) = id.as_string() {
            return s;
        }
    }
    // fallback: time + random
    let t = js::call_global("Date", &[JsValue::from_str("now")]);
    let n = js::call_global("Math", &[]);
    let _ = n;
    format!("id-{}", js::describe(&t))
}

fn push_msg(msg: &JsValue) {
    let s = store_ref();
    let arr = Array::from(&js::get(&s, "messages"));
    arr.push(msg);
    js::set(&s, "messages", &arr.clone().into());
}

fn mk_msg(kind: &str, text: &str) -> JsValue {
    let o = js::obj();
    js::set_str(&o, "id", &new_id());
    js::set_str(&o, "kind", kind);
    js::set_str(&o, "text", text);
    js::set_bool(&o, "visible", true);
    o.into()
}

// ── computed views ──────────────────────────────────────────────────

fn getters(d: &mut Def) {
    d.getter("draftInput", |_this, _args| {
        js::get(&store_ref(), "draft")
    })
    .getter("allowlistText", |_this, _args| {
        js::get(&store_ref(), "allowlist")
    })
    .getter("activeMessages", |_this, _args| {
        // Cowork filters todo noise here; keep the same shape so the tree can
        // stay a line-for-line port of messageBlockTpl.
        let s = store_ref();
        let all = Array::from(&js::get(&s, "messages"));
        let out = Array::new();
        for m in all.iter() {
            let kind = js::get_str(&m, "kind");
            if kind == "tool_call" || kind == "tool_result" {
                let name = js::get_str(&m, "name");
                if name.to_lowercase().contains("todo") {
                    continue;
                }
            }
            out.push(&m);
        }
        out.into()
    })
    .getter("activeWaiting", |_this, _args| {
        JsValue::from_bool(js::get_bool(&store_ref(), "waiting"))
    })
    .getter("showWelcome", |_this, _args| {
        let n = Array::from(&js::get(&store_ref(), "messages")).length();
        JsValue::from_bool(n == 0)
    })
    .getter("modelOptions", |_this, _args| {
        let s = store_ref();
        let list = Array::from(&js::get(&s, "models"));
        let cur = js::get_str(&s, "model");
        if !cur.is_empty() {
            let mut found = false;
            for v in list.iter() {
                if v.as_string().unwrap_or_default() == cur {
                    found = true;
                    break;
                }
            }
            if !found {
                let out = Array::new();
                out.push(&JsValue::from_str(&cur));
                for v in list.iter() {
                    out.push(&v);
                }
                return out.into();
            }
        }
        list.into()
    })
    .getter("todoItems", |_this, _args| {
        js::get(&store_ref(), "todos")
    })
    .getter("todoProgress", |_this, _args| {
        let todos = Array::from(&js::get(&store_ref(), "todos"));
        if todos.length() == 0 {
            return JsValue::from_str("");
        }
        let mut done = 0u32;
        for t in todos.iter() {
            if js::get_str(&t, "status") == "completed" {
                done += 1;
            }
        }
        JsValue::from_str(&format!("Tasks {done}/{}", todos.length()))
    })
    .getter("todoCurrentLabel", |_this, _args| {
        let todos = Array::from(&js::get(&store_ref(), "todos"));
        for t in todos.iter() {
            let st = js::get_str(&t, "status");
            if st == "in_progress" || st == "pending" {
                return JsValue::from_str(&js::get_str(&t, "content"));
            }
        }
        JsValue::from_str("")
    })
    .getter("pieDash", |_this, _args| {
        let s = store_ref();
        let used = js::get_num(&s, "tokensUsed");
        let total = js::get_num(&s, "contextWindow");
        let frac = if total > 0.0 { (used / total).clamp(0.0, 1.0) } else { 0.0 };
        // circumference of r=15.5 ≈ 97.4
        JsValue::from_str(&format!("{:.1} 97.4", frac * 97.4))
    })
    .getter("pieUsedK", |_this, _args| {
        let used = js::get_num(&store_ref(), "tokensUsed");
        if used <= 0.0 {
            return JsValue::from_str("");
        }
        if used >= 1000.0 {
            JsValue::from_str(&format!("{:.0}k", used / 1000.0))
        } else {
            JsValue::from_str(&format!("{used:.0}"))
        }
    })
    .getter("contextTooltip", |_this, _args| {
        let s = store_ref();
        let used = js::get_num(&s, "tokensUsed");
        let total = js::get_num(&s, "contextWindow");
        JsValue::from_str(&format!("{used:.0} / {total:.0} tokens"))
    })
    .getter("tabListOut", |_this, _args| {
        js::get(&store_ref(), "tabList")
    });
}

// ── actions the template calls ──────────────────────────────────────

fn methods(d: &mut Def) {
    d.method("send", |_this, _args| {
        send_current();
        JsValue::UNDEFINED
    })
    .method("stop", |_this, _args| {
        let s = store_ref();
        js::set_bool(&s, "waiting", false);
        js::set_str(&s, "statusLabel", "ready");
        js::set_str(&s, "statusClass", "ready");
        // best-effort abort; angela also accepts POST /api/abort
        let payload = js::obj();
        js::set_str(&payload, "tabId", &js::get_str(&s, "tabId"));
        api::send("/api/abort", Some("POST"), Some(&payload.into()), |_| {});
        mjs::redraw();
        JsValue::UNDEFINED
    })
    .method("onKeydown", |_this, args| {
        let ev = js::arg(&args, 0);
        let key = js::get_str(&ev, "key");
        let shift = js::get_bool(&ev, "shiftKey");
        if key == "Enter" && !shift {
            js::call(&ev, "preventDefault", &[]);
            send_current();
        }
        JsValue::UNDEFINED
    })
    .method("onDraftFocus", |_this, _args| JsValue::UNDEFINED)
        .method("setModel", |_this, args| {
            let v = js::arg_str(&args, 0);
            if !v.is_empty() {
                let s = store_ref();
                js::set_str(&s, "model", &v);
                save_model(&v);
                let dd = mjs::store_ref("dropdown");
                js::set_str(&dd, "model", &v);
                mjs::redraw();
            }
            JsValue::UNDEFINED
        })
        .method("onModelChange", |_this, args| {
        let ev = js::arg(&args, 0);
        let target = js::get(&ev, "target");
        let v = js::get_str(&target, "value");
        if !v.is_empty() {
            let s = store_ref();
            js::set_str(&s, "model", &v);
            save_model(&v);
            mjs::redraw();
        }
        JsValue::UNDEFINED
    })
    .method("onAgentChange", |_this, args| {
        let ev = js::arg(&args, 0);
        let target = js::get(&ev, "target");
        let v = js::get_str(&target, "value");
        if !v.is_empty() {
            js::set_str(&store_ref(), "agent", &v);
            mjs::redraw();
        }
        JsValue::UNDEFINED
    })
    .method("toggleAllowlist", |_this, _args| {
        let s = store_ref();
        let open = !js::get_bool(&s, "allowlistOpen");
        js::set_bool(&s, "allowlistOpen", open);
        mjs::redraw();
        JsValue::UNDEFINED
    })
    .method("onAllowlistInput", |_this, args| {
        let ev = js::arg(&args, 0);
        let target = js::get(&ev, "target");
        let v = js::get_str(&target, "value");
        let s = store_ref();
        js::set_str(&s, "allowlist", &v);
        let base = js::get_str(&s, "allowlistBaseline");
        js::set_bool(&s, "allowlistOverridden", v != base);
        JsValue::UNDEFINED
    })
    .method("onAllowlistBlur", |_this, _args| {
        // persist override to angela (best effort)
        let s = store_ref();
        let payload = js::obj();
        js::set_str(&payload, "text", &js::get_str(&s, "allowlist"));
        js::set_bool(&payload, "overridden", js::get_bool(&s, "allowlistOverridden"));
        api::send("/api/allowlist", Some("POST"), Some(&payload.into()), |_| {});
        JsValue::UNDEFINED
    })
    .method("decideApproval", |_this, args| {
        let msg = js::arg(&args, 0);
        let decision = js::arg_str(&args, 1);
        if decision.is_empty() {
            return JsValue::UNDEFINED;
        }
        let aid = js::get_str(&msg, "approvalId");
        if aid.is_empty() {
            return JsValue::UNDEFINED;
        }
        js::set_str(&msg, "status", &decision);
        let payload = js::obj();
        js::set_str(&payload, "id", &aid);
        js::set_str(&payload, "decision", &decision);
        api::send("/api/approve", Some("POST"), Some(&payload.into()), |_| {});
        mjs::redraw();
        JsValue::UNDEFINED
    })
    .method("toggleMsgVisible", |_this, args| {
        let msg = js::arg(&args, 0);
        let vis = js::get_bool(&msg, "visible");
        // missing `visible` reads falsy → toggling from default-true hides
        let has = !js::get(&msg, "visible").is_undefined();
        let next = if has { !vis } else { false };
        js::set_bool(&msg, "visible", next);
        // persist to angela event meta (best effort)
        let s = store_ref();
        let payload = js::obj();
        js::set_str(&payload, "sessionId", &js::get_str(&s, "sessionId"));
        js::set_str(&payload, "eventId", &js::get_str(&msg, "eventId"));
        js::set_bool(&payload, "visible", next);
        api::send("/api/event/meta", Some("POST"), Some(&payload.into()), |_| {});
        mjs::redraw();
        JsValue::UNDEFINED
    })
    .method("toggleMsgGoto", |_this, args| {
        let msg = js::arg(&args, 0);
        let goto = js::get(&msg, "goto");
        let enabled = if goto.is_falsy() { false } else { js::get_bool(&goto, "enabled") };
        let next = js::obj();
        js::set_bool(&next, "enabled", !enabled);
        let label = if goto.is_falsy() { String::new() } else { js::get_str(&goto, "label") };
        js::set_str(&next, "label", &label);
        js::set(&msg, "goto", &next.clone().into());
        mjs::redraw();
        JsValue::UNDEFINED
    })
    .method("ensureGoto", |_this, _args| JsValue::UNDEFINED)
    .method("onGotoLabelInput", |_this, args| {
        let msg = js::arg(&args, 0);
        // write-through to the nested object is already done by x-model;
        // persist best-effort.
        let goto = js::get(&msg, "goto");
        let s = store_ref();
        let payload = js::obj();
        js::set_str(&payload, "sessionId", &js::get_str(&s, "sessionId"));
        js::set_str(&payload, "eventId", &js::get_str(&msg, "eventId"));
        js::set(&payload, "goto", &goto);
        api::send("/api/event/meta", Some("POST"), Some(&payload.into()), |_| {});
        JsValue::UNDEFINED
    })
    .method("isGotoLabelDuplicate", |_this, _args| JsValue::from_bool(false))
    .method("newTab", |_this, _args| {
        let s = store_ref();
        let tabs = Array::from(&js::get(&s, "tabList"));
        let id = js::call_global("Date", &[]);
        let _ = id;
        let nid = tabs.length() as f64 + 1.0;
        // keep ids dense for v1; cowork uses real tab ids, we just count.
        let t = js::obj();
        js::set_num(&t, "id", nid);
        js::set_str(&t, "name", &format!("Chat {}", nid as u64));
        tabs.push(&t.into());
        js::set(&s, "tabList", &tabs.clone().into());
        js::set_num(&s, "activeTabId", nid);
        // fresh transcript per tab (v1: clear; cowork keeps per-tab history)
        js::set(&s, "messages", &js::new_array().into());
        js::set_bool(&s, "waiting", false);
        js::set_str(&s, "error", "");
        js::set_str(&s, "sessionId", "");
        mjs::redraw();
        JsValue::UNDEFINED
    })
    .method("closeTab", |_this, args| {
        let id = js::arg(&args, 0).as_f64().unwrap_or(0.0);
        let s = store_ref();
        let tabs = Array::from(&js::get(&s, "tabList"));
        if tabs.length() <= 1 {
            return JsValue::UNDEFINED;
        }
        let kept = Array::new();
        for t in tabs.iter() {
            if js::get_num(&t, "id") != id {
                kept.push(&t);
            }
        }
        js::set(&s, "tabList", &kept.clone().into());
        if js::get_num(&s, "activeTabId") == id {
            let first = kept.get(0);
            js::set_num(&s, "activeTabId", js::get_num(&first, "id"));
            js::set(&s, "messages", &js::new_array().into());
            js::set_str(&s, "sessionId", "");
        }
        mjs::redraw();
        JsValue::UNDEFINED
    })
    .method("selectTab", |_this, args| {
        let id = js::arg(&args, 0).as_f64().unwrap_or(0.0);
        js::set_num(&store_ref(), "activeTabId", id);
        mjs::redraw();
        JsValue::UNDEFINED
    })
    .method("toggleHistory", |_this, _args| {
        let s = store_ref();
        js::set_bool(&s, "historyOpen", !js::get_bool(&s, "historyOpen"));
        mjs::redraw();
        JsValue::UNDEFINED
    })
    .method("toggleSide", |_this, _args| {
        let s = store_ref();
        js::set_bool(&s, "sideOpen", !js::get_bool(&s, "sideOpen"));
        mjs::redraw();
        JsValue::UNDEFINED
    })
    .method("toggleFullscreen", |_this, _args| {
        let s = store_ref();
        js::set_bool(&s, "isFullscreen", !js::get_bool(&s, "isFullscreen"));
        mjs::redraw();
        JsValue::UNDEFINED
    })
    .method("openSession", |_this, args| {
        let id = js::arg_str(&args, 0);
        if id.is_empty() {
            return JsValue::UNDEFINED;
        }
        open_session(&id);
        JsValue::UNDEFINED
    })
    .method("deleteSession", |_this, args| {
        let id = js::arg_str(&args, 0);
        if id.is_empty() {
            return JsValue::UNDEFINED;
        }
        api::send(&format!("/api/sessions/{id}"), Some("DELETE"), None, move |_| {
            refresh_sessions();
        });
        JsValue::UNDEFINED
    })
    .method("cleanAllSessions", |_this, _args| {
        api::send("/api/sessions", Some("DELETE"), None, move |_| {
            refresh_sessions();
        });
        JsValue::UNDEFINED
    })
    .method("useStarter", |_this, args| {
        let prompt = js::arg_str(&args, 0);
        if prompt.is_empty() {
            return JsValue::UNDEFINED;
        }
        js::set_str(&store_ref(), "draft", &prompt);
        send_current();
        JsValue::UNDEFINED
    })
    .method("onMessagesScroll", |_this, _args| JsValue::UNDEFINED)
    .method("toggleTodos", |_this, _args| {
        let s = store_ref();
        js::set_bool(&s, "todosOpen", !js::get_bool(&s, "todosOpen"));
        mjs::redraw();
        JsValue::UNDEFINED
    })
    .method("todoIcon", |_this, args| {
        let t = js::arg(&args, 0);
        let st = js::get_str(&t, "status");
        JsValue::from_str(match st.as_str() {
            "completed" => "ph-check-circle",
            "in_progress" => "ph-circle-notch",
            _ => "ph-circle",
        })
    })
    .method("renderMarkdown", |_this, args| {
        let text = js::arg_str(&args, 0);
        JsValue::from_str(&crate::helpers::render_markdown(&text))
    })
    .method("toolCallCode", |_this, args| {
        JsValue::from_str(&crate::helpers::tool_call_code(&js::arg(&args, 0)))
    })
    .method("toolCallExplanation", |_this, args| {
        JsValue::from_str(&crate::helpers::tool_call_explanation(&js::arg(&args, 0)))
    })
    .method("toolResultImageNote", |_this, args| {
        JsValue::from_str(&crate::helpers::tool_result_image_note(&js::arg(&args, 0)))
    })
    .method("formatTelemetryHtml", |_this, args| {
        JsValue::from_str(&telemetry_html(&js::arg(&args, 0), &js::arg(&args, 1)))
    })
    .method("shortModel", |_this, args| {
        JsValue::from_str(&crate::helpers::short_model(&js::arg_str(&args, 0)))
    })
    // select filler: native <select> + x-for is unreliable in m-js, so the
    // JS original fills options imperatively. Same here, called from x-effect.
    .method("fillModels", |_this, args| {
        let el = js::arg(&args, 0);
        if el.is_falsy() {
            return JsValue::UNDEFINED;
        }
        let s = store_ref();
        let list = Array::from(&js::get(&s, "models"));
        let cur = js::get_str(&s, "model");
        fill_select(&el, &list, &cur);
        JsValue::UNDEFINED
    })
    .method("fillAgents", |_this, args| {
        let el = js::arg(&args, 0);
        if el.is_falsy() {
            return JsValue::UNDEFINED;
        }
        let s = store_ref();
        let list = Array::from(&js::get(&s, "agents"));
        let cur = js::get_str(&s, "agent");
        // agents are {name} objects
        let sel = js::get(&el, "value").as_string().unwrap_or_default();
        let sig = format!("{}={cur}", list.length());
        if js::get_str(&el, "datasetSig") == sig && sel == cur {
            return JsValue::UNDEFINED;
        }
        js::set_str(&el, "datasetSig", &sig);
        js::set(&el, "innerHTML", &JsValue::from_str(""));
        let doc = js::document();
        if list.length() == 0 {
            let o = js::call(&doc, "createElement", &[JsValue::from_str("option")]);
            js::set_str(&o, "value", "lucy");
            js::set_str(&o, "textContent", "lucy");
            js::call(&el, "appendChild", &[o]);
            return JsValue::UNDEFINED;
        }
        for v in list.iter() {
            let name = if let Some(s) = v.as_string() {
                s
            } else {
                js::get_str(&v, "name")
            };
            let o = js::call(&doc, "createElement", &[JsValue::from_str("option")]);
            js::set_str(&o, "value", &name);
            js::set_str(&o, "textContent", &name);
            if name == cur {
                js::set_bool(&o, "selected", true);
            }
            js::call(&el, "appendChild", &[o]);
        }
        JsValue::UNDEFINED
    });
}

fn fill_select(el: &JsValue, items: &Array, cur: &str) {
    let sel = js::get(&el, "value").as_string().unwrap_or_default();
    let sig = format!("{}={cur}", items.length());
    if js::get_str(el, "datasetSig") == sig && sel == cur {
        return;
    }
    js::set_str(el, "datasetSig", &sig);
    js::set(el, "innerHTML", &JsValue::from_str(""));
    let doc = js::document();
    if items.length() == 0 {
        let o = js::call(&doc, "createElement", &[JsValue::from_str("option")]);
        js::set_str(&o, "value", cur);
        js::set_str(&o, "textContent", cur);
        js::call(el, "appendChild", &[o]);
        return;
    }
    for v in items.iter() {
        let name = v.as_string().unwrap_or_else(|| js::describe(&v));
        let o = js::call(&doc, "createElement", &[JsValue::from_str("option")]);
        js::set_str(&o, "value", &name);
        js::set_str(&o, "textContent", &name);
        if name == cur {
            js::set_bool(&o, "selected", true);
        }
        js::call(el, "appendChild", &[o]);
    }
}

fn telemetry_html(t: &JsValue, msg: &JsValue) -> String {
    if js::get_str(msg, "kind") == "reasoning" {
        return String::new();
    }
    let mut chips = String::new();
    if !t.is_falsy() {
        let tps = js::get_num(t, "tok_per_sec");
        if tps.is_finite() && tps > 0.0 {
            chips.push_str(&format!("<span class=\"metric-chip\"><i class=\"ph ph-gauge\"></i><span>{tps:.2} tps</span></span>"));
        }
        let ct = js::get_num(t, "completion_tokens");
        if ct > 0.0 {
            chips.push_str(&format!("<span class=\"metric-chip\"><i class=\"ph ph-text-aa\"></i><span>{ct:.0} tok</span></span>"));
        }
    }
    if js::get_str(msg, "kind") == "tool_result" {
        let full = js::get_num(msg, "full_chars");
        if full > 0.0 {
            chips.push_str(&format!("<span class=\"metric-chip\"><i class=\"ph ph-cloud-arrow-down\"></i><span>{full:.0} B</span></span>"));
        }
    }
    chips
}

// ── wire ────────────────────────────────────────────────────────────────

/// Composer Send: user row + POST /api/chat, NDJSON lines applied as they
/// arrive; angela down → echo fallback (demo transport parity).
pub fn send_current() {
    let s = store_ref();
    let draft = js::get_str(&s, "draft").trim().to_string();
    if draft.is_empty() || js::get_bool(&s, "waiting") {
        return;
    }
    // /new slash command: fresh session, keep the prompt typed after it
    if draft.to_lowercase() == "/new" || draft.to_lowercase().starts_with("/new ") {
        let rest = draft.strip_prefix("/new").unwrap_or("").trim().to_string();
        new_session_local();
        if !rest.is_empty() {
            js::set_str(&s, "draft", &rest);
            send_current();
        }
        return;
    }
    let user = mk_msg("user", &draft);
    push_msg(&user);
    js::set_str(&s, "draft", "");
    js::set_bool(&s, "waiting", true);
    js::set_str(&s, "statusLabel", "running");
    js::set_str(&s, "statusClass", "running");
    js::set_str(&s, "error", "");
    mjs::redraw();
    scroll_bottom();

    let model = js::get_str(&s, "model");
    let agent = js::get_str(&s, "agent");
    let session_id = js::get_str(&s, "sessionId");
    let tab_id = js::get_str(&s, "tabId");
    let allowlist = js::get_str(&s, "allowlist");
    let overridden = js::get_bool(&s, "allowlistOverridden");
    let content = draft.clone();

    spawn_local(async move {
        match post_chat(&content, &model, &agent, &session_id, &tab_id, &allowlist, overridden).await {
            Ok(_) => {}
            Err(e) => {
                // angela unreachable → echo fallback so the UI still moves
                js::warn(&format!("[chat] angela unreachable ({e}), echo fallback"));
                echo_fallback(&content);
            }
        }
    });
}

fn echo_fallback(content: &str) {
    let s = store_ref();
    push_msg(&mk_msg("assistant", &format!("echo: {content}")));
    js::set_bool(&s, "waiting", false);
    js::set_str(&s, "statusLabel", "ready");
    js::set_str(&s, "statusClass", "ready");
    mjs::redraw();
    scroll_bottom();
}

fn new_session_local() {
    let s = store_ref();
    js::set(&s, "messages", &js::new_array().into());
    js::set_str(&s, "sessionId", "");
    js::set_bool(&s, "waiting", false);
    js::set_str(&s, "error", "");
    js::set_str(&s, "statusLabel", "ready");
    js::set_str(&s, "statusClass", "ready");
    push_msg(&mk_msg("assistant", "New session."));
    mjs::redraw();
    scroll_bottom();
}

/// POST /api/chat and apply NDJSON lines. Returns Err when the harness is
/// unreachable or the body is not NDJSON.
async fn post_chat(
    content: &str,
    model: &str,
    agent: &str,
    session_id: &str,
    tab_id: &str,
    allowlist: &str,
    overridden: bool,
) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    let g = js::global();
    let fetch_fn = js::get(&g, "fetch");
    let fetch_fn: js_sys::Function = fetch_fn.dyn_into().map_err(|_| "no fetch()".to_string())?;

    let payload = js::obj();
    js::set_str(&payload, "content", content);
    js::set_str(&payload, "model", model);
    js::set_str(&payload, "agent", agent);
    js::set_str(&payload, "tabId", tab_id);
    if !session_id.is_empty() {
        js::set_str(&payload, "sessionId", session_id);
    }
    if !allowlist.is_empty() {
        js::set_str(&payload, "allowlist", allowlist);
        js::set_bool(&payload, "allowlistOverridden", overridden);
    }
    let text = js_sys::JSON::stringify(&payload).map_err(|_| "stringify".to_string())?;
    let init = js::obj();
    js::set_str(&init, "method", "POST");
    let headers = js::obj();
    js::set_str(&headers, "content-type", "application/json");
    js::set(&init, "headers", &headers.into());
    js::set(&init, "body", &text.into());

    let p = fetch_fn
        .call2(&JsValue::NULL, &JsValue::from_str("/api/chat"), &init.into())
        .map_err(|e| js::describe(&e))?;
    let resp = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(p))
        .await
        .map_err(|e| js::describe(&e))?;
    let status = js::get_num(&resp, "status");
    if status >= 400.0 {
        return Err(format!("HTTP {status}"));
    }
    // NDJSON arrives as text (streaming reader comes later; v1 buffers).
    let text_p = js::call(&resp, "text", &[]);
    let body = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(text_p))
        .await
        .map_err(|e| js::describe(&e))?;
    let body = body.as_string().ok_or("non-text body")?;
    if body.trim().is_empty() {
        return Err("empty body".into());
    }
    let mut applied = 0;
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Each line should be JSON; the JS store's applyIncoming handles the
        // same shapes (card/event/usage/gen_info/approval_needed/done/…).
        let v = js_sys::JSON::parse(line).unwrap_or(JsValue::UNDEFINED);
        if v.is_undefined() {
            continue;
        }
        apply_line(&v);
        applied += 1;
    }
    if applied == 0 {
        return Err("no NDJSON lines".into());
    }
    Ok(())
}

/// Minimal applyIncoming: covers the shapes angela's NDJSON actually emits
/// (see serve.mjs handleChatStream) plus the WS card shapes the JS store
/// handles. Streaming deltas land on live rows; cards finalize them.
pub fn apply_line(v: &JsValue) {
    let t = js::get_str(v, "type");
    let s = store_ref();
    match t.as_str() {
        "card" => {
            let card = js::get(v, "card");
            apply_card(&card);
        }
        "event" => {
            let ev = js::get(v, "event");
            apply_chat_event(&ev);
        }
        "usage" | "gen_info" => {
            let prompt = js::get(&v, "prompt_tokens");
            if !prompt.is_undefined() && !prompt.is_null() {
                if let Some(n) = prompt.as_f64() {
                    if n >= 0.0 {
                        js::set_num(&s, "tokensUsed", n);
                    }
                }
            }
            let ctx = js::get_num(v, "contextWindow");
            if ctx > 0.0 {
                js::set_num(&s, "contextWindow", ctx);
            }
            mjs::redraw();
        }
        "approval_needed" => {
            let m = js::obj();
            js::set_str(&m, "id", &new_id());
            js::set_str(&m, "kind", "approval_needed");
            js::set_str(&m, "approvalId", &js::get_str(v, "id"));
            js::set_str(&m, "name", &js::get_str(v, "tool"));
            js::set_str(&m, "command", &js::get_str(v, "command"));
            let args = js::get(v, "args");
            let empty: JsValue = js::obj().into();
            js::set(&m, "args", if args.is_undefined() { &empty } else { &args });
            js::set_str(&m, "status", "pending");
            js::set_bool(&m, "visible", true);
            push_msg(&m.into());
            mjs::redraw();
            scroll_bottom();
        }
        "done" | "aborted" => {
            // finalize streaming rows
            let all = Array::from(&js::get(&s, "messages"));
            for m in all.iter() {
                if js::get_bool(&m, "streaming") {
                    js::set_bool(&m, "streaming", false);
                }
            }
            js::set_bool(&s, "waiting", false);
            js::set_str(&s, "statusLabel", "ready");
            js::set_str(&s, "statusClass", "ready");
            mjs::redraw();
            scroll_bottom();
        }
        "error" => {
            let err = js::get_str(v, "error");
            js::set_str(&s, "error", &err);
            push_msg(&mk_msg("error", &format!("Error: {err}")).into());
            js::set_bool(&s, "waiting", false);
            js::set_str(&s, "statusLabel", "ready");
            js::set_str(&s, "statusClass", "ready");
            mjs::redraw();
            scroll_bottom();
        }
        _ => {
            // session/state/hello/status/model/models/todos/visible/…
            if t == "status" {
                let running = js::get_str(v, "phase") == "running" || js::get_bool(v, "waiting");
                js::set_bool(&s, "waiting", running);
                js::set_str(&s, "statusLabel", if running { "running" } else { "ready" });
                js::set_str(&s, "statusClass", if running { "running" } else { "ready" });
                mjs::redraw();
            }
        }
    }
}

fn apply_card(card: &JsValue) {
    if card.is_falsy() {
        return;
    }
    let kind = js::get_str(card, "kind");
    match kind.as_str() {
        "assistant" => {
            let text = js::get_str(card, "text");
            if text.trim().is_empty() {
                return;
            }
            // finalize any live assistant row, else append
            let s = store_ref();
            let all = Array::from(&js::get(&s, "messages"));
            for m in all.iter().rev() {
                if js::get_str(&m, "kind") == "assistant" && js::get_bool(&m, "streaming") {
                    js::set_str(&m, "text", &text);
                    js::set_bool(&m, "streaming", false);
                    mjs::redraw();
                    scroll_bottom();
                    return;
                }
            }
            push_msg(&mk_msg("assistant", &text));
            mjs::redraw();
            scroll_bottom();
        }
        "reasoning" => {
            let text = js::get_str(card, "text");
            if text.trim().is_empty() {
                return;
            }
            push_msg(&{
                let m = mk_msg("reasoning", &text);
                m
            });
            mjs::redraw();
            scroll_bottom();
        }
        "tool_call" => {
            let m = js::obj();
            js::set_str(&m, "id", &new_id());
            js::set_str(&m, "kind", "tool_call");
            js::set_str(&m, "name", &js::get_str(card, "name"));
            let args = js::get(card, "args");
            let empty: JsValue = js::obj().into();
            js::set(&m, "args", if args.is_undefined() { &empty } else { &args });
            js::set_str(&m, "explanation", &js::get_str(card, "explanation"));
            js::set_bool(&m, "visible", true);
            push_msg(&m.into());
            mjs::redraw();
            scroll_bottom();
        }
        "tool_result" => {
            let m = js::obj();
            js::set_str(&m, "id", &new_id());
            js::set_str(&m, "kind", "tool_result");
            js::set_str(&m, "name", &js::get_str(card, "name"));
            js::set_str(&m, "text", &js::get_str(card, "text"));
            js::set_bool(&m, "ok", js::get(card, "ok").is_truthy());
            js::set_bool(&m, "visible", true);
            push_msg(&m.into());
            mjs::redraw();
            scroll_bottom();
        }
        _ => {}
    }
}

fn apply_chat_event(ev: &JsValue) {
    if ev.is_falsy() {
        return;
    }
    let t = js::get_str(ev, "type");
    match t.as_str() {
        "assistant_delta" | "reasoning_delta" | "tool_delta" => {
            append_delta(&t, ev);
        }
        "tool_call" => {
            let m = js::obj();
            js::set_str(&m, "id", &new_id());
            js::set_str(&m, "kind", "tool_call");
            js::set_str(&m, "name", &js::get_str(ev, "name"));
            let args = js::get(ev, "args");
            let empty: JsValue = js::obj().into();
            js::set(&m, "args", if args.is_undefined() { &empty } else { &args });
            js::set_bool(&m, "streaming", true);
            js::set_bool(&m, "visible", true);
            push_msg(&m.into());
            mjs::redraw();
            scroll_bottom();
        }
        "tool_result" => {
            let m = js::obj();
            js::set_str(&m, "id", &new_id());
            js::set_str(&m, "kind", "tool_result");
            js::set_str(&m, "name", &js::get_str(ev, "name"));
            js::set_str(&m, "text", &js::get_str(ev, "text"));
            js::set_bool(&m, "visible", true);
            push_msg(&m.into());
            mjs::redraw();
            scroll_bottom();
        }
        _ => {}
    }
}

fn append_delta(kind: &str, ev: &JsValue) {
    let chunk = if !js::get_str(ev, "text").is_empty() {
        js::get_str(ev, "text")
    } else {
        js::get_str(ev, "output")
    };
    if chunk.is_empty() {
        return;
    }
    let (row_kind, name) = match kind {
        "assistant_delta" => ("assistant", ""),
        "reasoning_delta" => ("reasoning", ""),
        _ => ("tool_call", js::get_str(ev, "name").leak() as &str),
    };
    let _ = name;
    let s = store_ref();
    let all = Array::from(&js::get(&s, "messages"));
    for m in all.iter().rev() {
        if js::get_str(&m, "kind") == row_kind && js::get_bool(&m, "streaming") {
            let cur = js::get_str(&m, "text");
            if row_kind == "tool_call" {
                let prog = js::get_str(&m, "progress");
                js::set_str(&m, "progress", &(prog + &chunk));
            } else {
                js::set_str(&m, "text", &(cur + &chunk));
            }
            mjs::redraw();
            scroll_bottom();
            return;
        }
    }
    // no live row → start one
    let m = if row_kind == "tool_call" {
        let o = js::obj();
        js::set_str(&o, "id", &new_id());
        js::set_str(&o, "kind", "tool_call");
        js::set_str(&o, "name", &js::get_str(ev, "name"));
        js::set(&o, "args", &js::obj().into());
        js::set_str(&o, "progress", &chunk);
        js::set_bool(&o, "streaming", true);
        js::set_bool(&o, "visible", true);
        o.into()
    } else {
        let o = mk_msg(row_kind, &chunk);
        js::set_bool(&o, "streaming", true);
        o
    };
    push_msg(&m);
    mjs::redraw();
    scroll_bottom();
}

// ── boot pulls ──────────────────────────────────────────────────────

/// Agents + sessions + health, once at boot and on a slow poll.
pub fn refresh_all() {
    chat_api::models(|v| {
        let s = store_ref();
        let models = js::get(&v, "models");
        if models.is_undefined() || models.is_null() { return; }
        js::set(&s, "models", &models);
        let configured = js::get_str(&v, "default_model");
        let current = js::get_str(&s, "model");
        let arr = Array::from(&models);
        let valid = arr.iter().any(|x| x.as_string().as_deref() == Some(&current));
        if current.is_empty() || !valid {
            let selected = if !configured.is_empty() { configured } else { DEFAULT_MODEL.to_string() };
            js::set_str(&s, "model", &selected);
            js::set_str(&mjs::store_ref("dropdown"), "model", &selected);
            save_model(&selected);
        }
        mjs::redraw();
    });
    refresh_agents();
    refresh_sessions();
    api::get("/api/health", None, |v| {
        let s = store_ref();
        let ok = js::get_str(&v, "status") != "" || !js::get(&v, "error").is_truthy() || js::get_bool(&v, "ok");
        js::set_bool(&s, "connected", ok || js::get(&v, "ok").is_truthy() || js::get_str(&v, "status") == "ok");
        // /api/health shape varies; any 200 with JSON counts as reachable.
        js::set_bool(&s, "connected", true);
        mjs::redraw();
    });
}

fn refresh_agents() {
    api::get("/api/agents", None, |v| {
        let s = store_ref();
        let agents = js::get(&v, "agents");
        if agents.is_undefined() || agents.is_null() {
            return;
        }
        js::set(&s, "agents", &agents);
        // seed model list from agents
        let arr = Array::from(&agents);
        let models = Array::new();
        for a in arr.iter() {
            for m in Array::from(&js::get(&a, "models")).iter() {
                if let Some(str) = m.as_string() {
                    if !str.is_empty() {
                        models.push(&JsValue::from_str(&str));
                    }
                }
            }
            let single = js::get_str(&a, "model");
            if !single.is_empty() {
                models.push(&JsValue::from_str(&single));
            }
            let al = js::get_str(&a, "allowlist");
            if !al.is_empty() && !js::get_bool(&s, "allowlistInited") {
                js::set_str(&s, "allowlist", &al);
                js::set_str(&s, "allowlistBaseline", &al);
                js::set_bool(&s, "allowlistInited", true);
            }
        }
        if models.length() > 0 {
            js::set(&s, "models", &models.clone().into());
            let cur = js::get_str(&s, "model");
            let mut has = false;
            for m in models.iter() {
                if m.as_string().unwrap_or_default() == cur {
                    has = true;
                    break;
                }
            }
            if !has {
                if let Some(first) = models.get(0).as_string() {
                    js::set_str(&s, "model", &first);
                    save_model(&first);
                }
            }
        }
        mjs::redraw();
    });
}

fn refresh_sessions() {
    api::get("/api/sessions", None, |v| {
        let s = store_ref();
        let list = js::get(&v, "sessions");
        if list.is_undefined() || list.is_null() {
            return;
        }
        js::set(&s, "historyList", &list);
        mjs::redraw();
    });
}

fn open_session(id: &str) {
    api::get(&format!("/api/sessions/{id}"), None, move |v| {
        let s = store_ref();
        let msgs = js::get(&v, "messages");
        if !msgs.is_undefined() && !msgs.is_null() {
            js::set(&s, "messages", &msgs);
        }
        let meta = js::get(&v, "session");
        if !meta.is_undefined() && !meta.is_null() {
            let sid = js::get_str(&meta, "id");
            if !sid.is_empty() {
                js::set_str(&s, "sessionId", &sid);
                js::set_str(&s, "activeSessionId", &sid);
            }
            let m = js::get_str(&meta, "model");
            if !m.is_empty() {
                js::set_str(&s, "model", &m);
            }
        }
        js::set_bool(&s, "waiting", false);
        mjs::redraw();
        scroll_bottom();
    });
}

fn scroll_bottom() {
    let s = store_ref();
    if !js::get_bool(&s, "_stick") {
        return;
    }
    // Both demo panes share the store; keep each transcript pinned.
    for sel in ["#chat-messages", "#chat-messages-side"] {
        if let Some(el) = js::query(sel) {
            let h = js::get_num(&el, "scrollHeight");
            js::set_num(&el, "scrollTop", h);
        }
    }
}

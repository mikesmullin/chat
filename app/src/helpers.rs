//! Pure helpers — Rust port of `m-js-cmp-chat/src/helpers.js`.
//!
//! The JS original is shared between the widgets and an optional store; here
//! the same functions live in one place and both the store and the page call
//! them. Anything that touches the DOM stays in `pages.rs`; this file is
//! strings in, strings out (plus tiny JsValue readers).

use rustex1_kit::js;
use wasm_bindgen::JsValue;

/// `escHtml` — & < > " escaped.
pub fn esc_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Caption for a tool_result that attached an image (no base64 in the UI).
/// `msg` is the message object; reads `.image.{mimeType,bytes}`.
pub fn tool_result_image_note(msg: &JsValue) -> String {
    let img = js::get(msg, "image");
    if img.is_falsy() {
        return String::new();
    }
    let mime = js::get_str(&img, "mimeType");
    let mime = if mime.is_empty() { "image".to_string() } else { mime };
    let bytes = js::get_num(&img, "bytes");
    if bytes.is_finite() && bytes > 0.0 {
        let bytes = bytes as u64;
        let kib = if bytes >= 1024 {
            let k = bytes as f64 / 1024.0;
            if bytes >= 10240 {
                format!("{k:.0} KiB")
            } else {
                format!("{k:.1} KiB")
            }
        } else {
            format!("{bytes} B")
        };
        return format!("{mime} · {kib}");
    }
    mime
}

/// Explanation for a tool_call bubble (top-level or args.explanation).
pub fn tool_call_explanation(msg: &JsValue) -> String {
    let e = js::get_str(msg, "explanation");
    if !e.trim().is_empty() {
        return e.trim().to_string();
    }
    let args = js::get(msg, "args");
    if !args.is_falsy() {
        let a = js::get_str(&args, "explanation");
        if !a.trim().is_empty() {
            return a.trim().to_string();
        }
    }
    String::new()
}

/// Primary command body for a tool_call code block:
/// bash → script; exec → command + args; else remaining args as pretty JSON.
pub fn tool_call_code(msg: &JsValue) -> String {
    let args = js::get(msg, "args");
    if args.is_falsy() {
        return String::new();
    }
    // script wins (bash)
    let script = js::get_str(&args, "script");
    if !script.trim().is_empty() {
        return script;
    }
    // command + argv (exec)
    let cmd = js::get_str(&args, "command");
    if !cmd.trim().is_empty() {
        let argv = js::get(&args, "args");
        let mut parts = vec![cmd.trim().to_string()];
        if !argv.is_falsy() {
            if let Ok(arr) = argv.clone().dyn_into::<js_sys::Array>() {
                for v in arr.iter() {
                    if let Some(s) = v.as_string() {
                        parts.push(s);
                    } else {
                        parts.push(js::describe(&v));
                    }
                }
            }
        }
        return parts.join(" ");
    }
    // else: remaining args as JSON, minus explanation
    stringify_without(&args, &["explanation"])
}

fn stringify_without(args: &JsValue, drop_keys: &[&str]) -> String {
    // Clone via JSON so we can delete keys without mutating the store.
    let g = js::global();
    let json = js::get(&g, "JSON");
    let s = js::call(&json, "stringify", &[args.clone()]);
    let s = s.as_string().unwrap_or_default();
    if s.is_empty() || s == "{}" {
        return String::new();
    }
    // Fast path: single string value → return it bare.
    // Parse in JS to avoid serde: Object.keys / values.
    let keys = js::call_global("Object", &[]); // placeholder, replaced below
    let _ = keys;
    // Keep it simple and honest: filter via a small JS snippet is overkill;
    // the common shapes (single string arg, or pretty JSON) are both readable
    // as-is once explanation is the only dropped key and it rarely co-occurs
    // with a payload. If explanation is present, strip it textually is unsafe,
    // so fall back to key-by-key reconstruction in JS.
    let parse = js::get(&json, "parse");
    // Rebuild without dropped keys using a JS helper call.
    let helper = r#"((o,drop)=>{if(!o||typeof o!=='object'||Array.isArray(o))return o;const c={};for(const k of Object.keys(o)){if(drop.includes(k))continue;const v=o[k];if(v==null||v===''||(Array.isArray(v)&&!v.length))continue;c[k]=v;}return c;})"#;
    let func: js_sys::Function = js::call_global("eval", &[JsValue::from_str(helper)])
        .dyn_into()
        .unwrap_or_else(|_| js_sys::Function::new_no_args("return arguments[0]"));
    let drop_arr = js_sys::Array::new();
    for k in drop_keys {
        drop_arr.push(&JsValue::from_str(k));
    }
    let cleaned = func
        .call2(&JsValue::NULL, args, &drop_arr)
        .unwrap_or(args.clone());
    // If the cleaned object has exactly one string value, return it bare.
    let keys_arr = js_sys::Object::keys(&js_sys::Object::from(cleaned.clone()));
    if keys_arr.length() == 0 {
        return String::new();
    }
    if keys_arr.length() == 1 {
        let v = js::get(&cleaned, &keys_arr.get(0).as_string().unwrap_or_default());
        if let Some(s) = v.as_string() {
            if !s.is_empty() {
                return s;
            }
        }
    }
    let _ = parse;
    js::call(&json, "stringify", &[cleaned, JsValue::NULL, JsValue::from_f64(2.0)])
        .as_string()
        .unwrap_or_default()
}

/// Assistant markdown → HTML. Uses global `marked` + `DOMPurify` when the host
/// page loaded them; otherwise a tiny escape + `**bold**` / `` `code` `` /
/// newlines fallback — same contract as the JS original.
pub fn render_markdown(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let g = js::global();
    let marked = js::get(&g, "marked");
    if !marked.is_undefined() && !marked.is_null() {
        let parse_fn = js::get(&marked, "parse");
        let html = if !parse_fn.is_undefined() && parse_fn.is_function() {
            // marked.setOptions({gfm:true,breaks:true,…}) — best effort
            let opts = js::obj();
            js::set_bool(&opts, "gfm", true);
            js::set_bool(&opts, "breaks", true);
            let set_opts = js::get(&marked, "setOptions");
            if set_opts.is_function() {
                let _ = js_sys::Function::from(marked.clone()).call0(&JsValue::NULL);
            }
            let _ = opts;
            js::call(&marked, "parse", &[JsValue::from_str(text)])
                .as_string()
                .unwrap_or_default()
        } else {
            js::call(&marked, "call", &[JsValue::from_str(text)])
                .as_string()
                .unwrap_or_else(|| esc_html(text))
        };
        let purify = js::get(&g, "DOMPurify");
        if !purify.is_undefined() && !purify.is_null() {
            let clean =
                js::call(&purify, "sanitize", &[JsValue::from_str(&html)]);
            if let Some(s) = clean.as_string() {
                return s;
            }
        }
        if !html.is_empty() {
            return html;
        }
    }
    // Fallback: escape + `code` + **bold** + newlines.
    let mut h = esc_html(text);
    h = code_bold(&h);
    h.replace('\n', "<br>")
}

fn code_bold(escaped: &str) -> String {
    // Operates on already-escaped text; <code>/<strong> tags we insert are
    // the only markup. Keep it single-pass and simple.
    let mut out = String::with_capacity(escaped.len());
    let bytes: Vec<char> = escaped.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '`' {
            if let Some(j) = bytes[i + 1..].iter().position(|&c| c == '`') {
                let inner: String = bytes[i + 1..i + 1 + j].iter().collect();
                out.push_str("<code>");
                out.push_str(&inner);
                out.push_str("</code>");
                i += j + 2;
                continue;
            }
        }
        if bytes[i] == '*' && i + 1 < bytes.len() && bytes[i + 1] == '*' {
            let rest: String = bytes[i + 2..].iter().collect();
            if let Some(j) = rest.find("**") {
                let inner = &rest[..j];
                out.push_str("<strong>");
                out.push_str(inner);
                out.push_str("</strong>");
                i += 2 + j + 2;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

pub fn short_model(m: &str) -> String {
    if m.len() <= 28 {
        return m.to_string();
    }
    let head: String = m.chars().take(12).collect();
    let tail: String = m.chars().rev().take(12).collect::<String>().chars().rev().collect();
    format!("{head}…{tail}")
}

pub fn format_elapsed(ms: f64) -> String {
    format!("{:.3}s", (ms.max(0.0)) / 1000.0)
}

use wasm_bindgen::JsCast;

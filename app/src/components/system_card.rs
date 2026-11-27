use rustex1_kit::h::{m, Node};
use rustex1_kit::js;
use wasm_bindgen::JsValue;

use crate::components::{name_tag, samples, session};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .bubble-system {
  display: flex; gap: 7px; align-items: center;
  font-size: 12px; color: var(--gray-dim); line-height: 1.6;
  padding: 2px 0; white-space: pre-wrap;
}
:is(#sb, #home) .bubble-system i { line-height: 19px; }
:is(#sb, #home) .status-text { display: flex; align-items: center; flex-wrap: wrap; min-width: 0; font-size: 0; }
:is(#sb, #home) .status-fragment { display: contents; }
:is(#sb, #home) .turn-row .status-text .name-tag { margin-top: 0; align-self: center; }
:is(#sb, #home) .status-plain { font-size: 12px; white-space: pre-wrap; overflow-wrap: anywhere; min-width: 0; }

.system-demo { width: min(560px, 100%); }
.system-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

fn mention_parts(text: &str) -> Vec<(bool, String)> {
    let mut parts = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'@' || (i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || b"._%+-".contains(&bytes[i - 1]))) {
            i += 1;
            continue;
        }
        let mut end = i + 1;
        while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'-' || bytes[end] == b'_') { end += 1; }
        if end == i + 1 { i += 1; continue; }
        if start < i { parts.push((false, text[start..i].to_string())); }
        parts.push((true, text[i + 1..end].to_string()));
        start = end;
        i = end;
    }
    if start < text.len() { parts.push((false, text[start..].to_string())); }
    parts
}

pub(crate) fn status_parts(_this: JsValue, args: js_sys::Array) -> JsValue {
    let parts = js_sys::Array::new();
    for (mention, text) in mention_parts(&js::arg_str(&args, 0)) {
        let part = js::obj();
        js::set_bool(&part, "mention", mention);
        js::set_str(&part, "text", &text);
        parts.push(&part);
    }
    parts.into()
}

pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![m("div")
        .class("bubble-system")
        .iff(&format!("{v}.kind === 'system'"))
        .kids(vec![
            Node::icon("info").class("ph ph-info"),
            m("span").class("status-text")
                .kids(vec![
                    m("span").class("status-fragment").for_each(&format!("part in {st}.statusParts({v}.text)"))
                        .kids(vec![
                            m("span").class("status-plain").iff("!part.mention").text("part.text"),
                            name_tag::mention(st, "part.text").iff("part.mention"),
                        ]),
                ]),
            m("span").class("name-tag-when").show(&format!("{v}.commandNotice && {v}.ts"))
                .text(&format!("'· ' + {st}.relAgo({v}.ts)")),
        ])]
}

pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::row("system"));
    m("div").class("system-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(session::row(st, &v, parts(st, &v))),
    )
}

pub fn register() {
    css::inject("system-card", CSS);
}

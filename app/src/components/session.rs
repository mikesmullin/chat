use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::js;
use wasm_bindgen::JsValue;

use crate::components::{
    approval_card, assistant_card, decision_card, error_card, prompt_card, queued_card,
    reasoning_card, system_card,
    thinking, thread, tool_call_card, tool_response_card, user_card,
};
use crate::css;

pub const CSS: &str = r#"
.mjs-chat .messages {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 20px 24px 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  overscroll-behavior: contain;
}
.mjs-chat.mode-sidebar .messages { padding: 10px; }

:is(#sb, #home) .turn-row {
  display: flex;
  align-items: stretch;
  gap: 10px;
}
:is(#sb, #home) .turn-rail {
  flex: 0 0 auto;
  width: 18px;
  position: relative;
  display: flex;
  justify-content: center;
}
:is(#sb, #home) .turn-row { --turn-gap: 4px; --dot-center: 10px; }
:is(#sb, #home) .turn-row:has(.bubble-assistant) { --dot-center: 14px; }
:is(#sb, #home) .turn-row.in-turn .turn-rail::before {
  content: "";
  position: absolute;
  top: 0;
  bottom: calc(var(--turn-gap) * -1);
  left: 50%;
  width: 4px; margin-left: -2px;
  background: color-mix(in srgb, var(--agent-accent, #a78bfa) 45%, transparent);
}
:is(#sb, #home) .turn-row.turn-first .turn-rail::before {
  top: var(--dot-center); border-radius: 2px 2px 0 0;
}
:is(#sb, #home) .turn-row.turn-last .turn-rail::before {
  top: 0;
  bottom: auto;
  height: var(--dot-center);
  border-radius: 0 0 2px 2px;
}
:is(#sb, #home) .turn-row.turn-first.turn-last .turn-rail::before { display: none; }
:is(#sb, #home) .turn-dot {
  position: absolute;
  top: calc(var(--dot-center) - 9px); left: 50%;
  width: 18px; height: 18px; margin-left: -9px;
  box-sizing: border-box;
  border: 2px solid var(--panel, #161616);
  border-radius: 50%;
  background: var(--agent-accent, #a78bfa);
}
:is(#sb, #home) .turn-row.no-turn .turn-dot { display: none; }
:is(#sb, #home) .turn-row .msg-block { flex: 1 1 auto; min-width: 0; }
:is(#sb, #home) .turn-row .msg-block.msg-user {
  align-items: flex-end;
  flex: 0 1 auto;
  margin-left: auto;
}
:is(#sb, #home) .turn-row .msg-block.msg-user .bubble-user {
  width: auto;
  max-width: 100%;
  align-self: stretch;
}
:is(#sb, #home) .turn-row .msg-block.msg-user.msg-from-other {
  margin-left: 0;
  margin-right: auto;
  align-items: flex-start;
}
:is(#sb, #home) .turn-row .name-tag { align-self: flex-start; margin-top: 4px; }

:is(#sb, #home) .turn-row .bubble-assistant > div > :first-child { margin-top: 0; }
:is(#sb, #home) .turn-row .bubble-assistant > div > :last-child { margin-bottom: 0; }




.session-demo { width: min(640px, 100%); height: 560px; }
.session-demo .mjs-chat.mode-sidebar { border-left: 0; }
.session-demo .mjs-chat { height: 100%; }
"#;

pub fn transcript(st: &str) -> Node {
    m("section").class("main").child(messages(st))
}

pub fn session(st: &str) -> Node {
    m("div")
        .class("mjs-chat mode-sidebar")
        .attr("aria-label", "Session")
        .child(transcript(st))
}

pub fn demo() -> Node {
    m("div")
        .class("session-demo")
        .child(session(crate::components::samples::EXPR))
}

pub fn register() {
    css::inject("session", CSS);
}


fn card_kids(st: &str, v: &str) -> Vec<Node> {
    let mut out = Vec::new();
    out.extend(user_card::parts(st, v));
    out.extend(assistant_card::parts(st, v));
    out.extend(error_card::parts(st, v));
    out.extend(system_card::parts(st, v));
    out.extend(reasoning_card::parts(st, v));
    out.extend(prompt_card::system_parts(st, v));
    out.extend(prompt_card::user_parts(st, v));
    out.extend(decision_card::parts(st, v));
    out.extend(tool_call_card::parts(st, v));
    out.extend(tool_response_card::parts(st, v));
    out.extend(approval_card::parts(st, v));
    out
}

const SIMPLE_HIDDEN: &[&str] = &["system", "system_prompt", "agent_prompt", "reasoning", "tool_call", "tool_result"];

fn visible_event(v: &str) -> String {
    let kinds = SIMPLE_HIDDEN.iter().map(|kind| format!("'{kind}'")).collect::<Vec<_>>().join(", ");
    format!("($store.mode.advanced || {v}.commandNotice || ![{kinds}].includes({v}.kind))")
}

thread_local! {
    static SIMPLE_EDGES: std::cell::RefCell<Option<(JsValue, std::collections::HashMap<String, (bool, bool)>)>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn turn_edges(this: JsValue, args: js_sys::Array) -> JsValue {
    let row = js::arg(&args, 0);
    if js::arg(&args, 1).as_bool().unwrap_or(false) {
        return JsValue::from_str(&format!("{}{}", if js::get_bool(&row, "turnFirst") { " turn-first" } else { "" }, if js::get_bool(&row, "turnLast") { " turn-last" } else { "" }));
    }
    let messages = js::get(&this, "messages");
    let id = js::get_str(&row, "id");
    SIMPLE_EDGES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.as_ref().is_none_or(|(old, _)| !js_sys::Object::is(old, &messages)) {
            let mut edges = std::collections::HashMap::new();
            let mut group: Vec<String> = Vec::new();
            let flush = |group: &mut Vec<String>, edges: &mut std::collections::HashMap<String, (bool, bool)>| {
                let n = group.len();
                for (i, id) in group.drain(..).enumerate() { edges.insert(id, (i == 0, i + 1 == n)); }
            };
            for item in js_sys::Array::from(&messages).iter() {
                if !js::get(&item, "turn").is_truthy() { flush(&mut group, &mut edges); continue; }
                if js::get_bool(&item, "turnFirst") { flush(&mut group, &mut edges); }
                if !SIMPLE_HIDDEN.contains(&js::get_str(&item, "kind").as_str()) {
                    group.push(js::get_str(&item, "id"));
                }
                if js::get_bool(&item, "turnLast") { flush(&mut group, &mut edges); }
            }
            flush(&mut group, &mut edges);
            *cache = Some((messages, edges));
        }
        let (first, last) = cache.as_ref().and_then(|(_, edges)| edges.get(&id)).copied()
            .unwrap_or((js::get_bool(&row, "turnFirst"), js::get_bool(&row, "turnLast")));
        JsValue::from_str(&format!("{}{}", if first { " turn-first" } else { "" }, if last { " turn-last" } else { "" }))
    })
}

pub(crate) fn stream_edges(this: JsValue, args: js_sys::Array) -> JsValue {
    let reasoning = js::arg_str(&args, 0) == "reasoning";
    let advanced = js::arg(&args, 1).as_bool().unwrap_or(false);
    let rows = js_sys::Array::from(&js::get(&this, "messages"));
    let mut preceding = false;
    for row in rows.iter().collect::<Vec<_>>().into_iter().rev() {
        let kind = js::get_str(&row, "kind");
        if kind == "user" { break; }
        if js::get(&row, "turn").is_truthy() && (advanced || !SIMPLE_HIDDEN.contains(&kind.as_str())) {
            preceding = true;
            break;
        }
    }
    if js_sys::Array::from(&js::get(&this, "approvals")).length() > 0 { preceding = true; }
    if !reasoning && advanced && !js::get_str(&this, "streamReasoning").is_empty() { preceding = true; }
    let last = !reasoning || js::get_str(&this, "streamText").is_empty();
    JsValue::from_str(&format!("{}{}", if preceding { "" } else { " turn-first" }, if last { " turn-last" } else { "" }))
}

pub fn row(st: &str, v: &str, parts: Vec<Node>) -> Node {
    m("div")
        .class("turn-row")
        .bind(
            "class",
            &format!(
                "(({v}.turn ? 'in-turn' : 'no-turn') + {st}.turnEdges({v}, $store.mode.advanced))"
            ),
        )
        .bind("style", &format!("('--agent-accent: ' + {st}.accentOf({v}.handle || ''))"))
        .kids(vec![
            m("div").class("turn-rail").child(m("span").class("turn-dot")),
            m("div")
                .class("msg-block")
                .bind("class", &format!("(({v}.kind === 'user' ? 'msg-user' : '') + (({v}.from && {v}.from !== $store.ident.name) ? ' msg-from-other' : ''))"))
                .kids(parts),
        ])
}

fn streaming_answer() -> Node {
    m("div").class("bubble-assistant md-body streaming")
        .attr("data-live-stream", "answer").ignore()
}

pub(crate) fn refresh_stream_nodes(el: &JsValue, answer: &str, reasoning: &str) {
    if js::is_missing(el) { return; }
    for (kind, html) in [("answer", answer), ("reasoning", reasoning)] {
        let node = js::call(el, "querySelector", &[JsValue::from_str(&format!("[data-live-stream='{kind}']"))]);
        if !js::is_missing(&node) && js::get_str(&node, "innerHTML") != html {
            js::set_str(&node, "innerHTML", html);
        }
    }
}

pub fn messages_threaded(st: &str) -> Node {
    m("div")
        .class("messages")
        .effect(&format!(
            "({st}.streamText, {st}.compacted, {st}.treeOpen, {st}.waiting, {st}.refreshStream($el), {st}.keepPinned($el))"
        ))
        .on("scroll", &format!("{st}.onScroll($event)"))
        .on(
            "click",
            "$event.target.tagName === 'IMG' && $store.lightbox.open($event.target.src)",
        )
        .kids(vec![
            m("div")
                .class("thread-row")
                .for_each(&format!("t in {st}.threads"))
                .key("t.id")
                .show(&format!("(!t.leaf || !t.message || {})", visible_event("t.message")))
                .bind("style", "('--d: ' + t.depth)")
                .kids(vec![
                    m("div")
                        .iff("!t.leaf")
                        .class("thread-slot")
                        .child(thread::row(st)),
                    m("div")
                        .iff("t.leaf && t.message")
                        .bind(
                            "class",
                            "('msg-block ' + (t.message.kind === 'user' ? 'msg-user' : '') + ((t.message.from && t.message.from !== $store.ident.name) ? ' msg-from-other' : ''))",
                        )
                        .kids(card_kids(st, "t.message")),
                ]),
            m("div")
                .class("msg-block")
                .show(&format!("{st}.streamText"))
                .child(
                    streaming_answer(),
                ),
            m("div")
                .class("msg-block thinking-block")
                .show(&format!("{st}.waiting && !{st}.stopping && !{st}.stopUnconfirmed"))
                .child(thinking::thinking()),
            m("div").class("thinking").show(&format!("{st}.waiting && ({st}.stopping || {st}.stopUnconfirmed)"))
                .text(&format!("({st}.stopping ? 'Stopping…' : 'Stop not confirmed. Try again.')")),
        ]
        .into_iter()
        .chain(queued_card::queue_parts(st))
        .collect())
}

pub fn messages(st: &str) -> Node {
    m("div")
        .class("messages")
        .effect(&format!(
            "({st}.messages.length, {st}.streamText, {st}.streamReasoning, \
              {st}.compacted, {st}.treeOpen, $store.mode.advanced, {st}.waiting, \
              {st}.refreshStream($el), {st}.keepPinned($el))"
        ))
        .on("scroll", &format!("{st}.onScroll($event)"))
        .on(
            "click",
            "$event.target.tagName === 'IMG' && $store.lightbox.open($event.target.src)",
        )
        .kids(vec![
        row(st, "m", card_kids(st, "m"))
            .for_each(&format!("m in {st}.messages"))
            .key("m.id")
            .show(&visible_event("m")),

        row(st, "ap", approval_card::parts(st, "ap"))
            .for_each(&format!("ap in {st}.approvals"))
            .key("ap.id"),

        m("div")
            .class("turn-row in-turn")
            .bind("class", &format!("{st}.streamEdges('reasoning', $store.mode.advanced)"))
            .show(&format!("$store.mode.advanced && {st}.streamReasoning"))
            .bind("style", &format!("('--agent-accent: ' + {st}.accentOf({st}.streamHandle))"))
            .kids(vec![
            m("div").class("turn-rail").child(m("span").class("turn-dot")),
            m("div").class("msg-block").child(
                m("div")
                    .class("fold-card fold-reasoning")
                    .bind("class", &format!("({st}.streamReasoningOpen ? '' : 'is-collapsed')"))
                    .attr("title", "Click to expand or collapse")
                    .on("click", &format!("{st}.toggleCard('')"))
                    .kids(vec![
                    m("div").class("fold-head").kids(vec![
                        Node::icon("brain").class("ph ph-brain"),
                        m("span").class("fold-label").child(lit("Reasoning")),
                    ]),
                    m("div")
                        .class("fold-body md-body")
                        .attr("data-live-stream", "reasoning")
                        .ignore(),
                ]),
            )]),
        m("div")
            .class("turn-row in-turn")
            .bind("class", &format!("{st}.streamEdges('answer', $store.mode.advanced)"))
            .show(&format!("{st}.streamText"))
            .bind("style", &format!("('--agent-accent: ' + {st}.accentOf({st}.streamHandle))"))
            .kids(vec![
                m("div").class("turn-rail").child(m("span").class("turn-dot")),
                m("div").class("msg-block").child(
                    streaming_answer(),
                ),
            ]),
        m("div")
            .class("msg-block thinking-block")
            .show(&format!("{st}.waiting && !{st}.stopping && !{st}.stopUnconfirmed"))
            .child(thinking::thinking()),
        m("div").class("thinking").show(&format!("{st}.waiting && ({st}.stopping || {st}.stopUnconfirmed)"))
            .text(&format!("({st}.stopping ? 'Stopping…' : 'Stop not confirmed. Try again.')")),
    ]
    .into_iter()
    .chain(queued_card::queue_parts(st))
    .collect())
}

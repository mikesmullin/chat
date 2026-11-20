//! The conversation's complete node tree, used to choose the write cursor.
use std::collections::HashMap;
use wasm_bindgen::JsValue;
use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::{js, mjs};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .history-tree-wrap { display: flex; flex: 1 1 auto; min-height: 0; }
:is(#sb, #home) .history-tree { flex: 1 1 auto; min-height: 0; overflow: auto; padding: 10px; font: 12px/1.5 var(--mono, monospace); }
:is(#sb, #home) .history-tree-row { display: flex; align-items: baseline; width: 100%; min-width: 0; padding: 2px 5px; border: 0; border-radius: 4px; background: transparent; color: var(--muted); font: inherit; text-align: left; cursor: pointer; }
:is(#sb, #home) .history-tree-row:hover, :is(#sb, #home) .history-tree-row.is-selected { background: var(--accent-tint); }
:is(#sb, #home) .history-tree-row.is-user { color: var(--accent-soft); }
:is(#sb, #home) .history-tree-row.is-assistant { color: var(--pastel-teal); }
:is(#sb, #home) .history-tree-prefix { white-space: pre; color: var(--gray-dim); flex: 0 0 auto; }
:is(#sb, #home) .history-tree-kind { white-space: pre; color: inherit; flex: 0 0 auto; }
:is(#sb, #home) .history-tree-text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
:is(#sb, #home) .history-tree-current { color: var(--accent); padding-left: 6px; flex: 0 0 auto; }
:is(#sb, #home) .history-tree-status { color: var(--gray-dim); padding: 8px 5px; }
.history-tree-demo { width: min(680px, 100%); height: 380px; display: flex; }
"#;

/// Flatten the forest in depth-first order, indenting only at a fork so long
/// linear conversations don't disappear off the right edge of the panel.
pub fn flatten(nodes: &js_sys::Array, cursor: &str) -> JsValue {
    let mut children: HashMap<String, Vec<JsValue>> = HashMap::new();
    for n in nodes.iter() { children.entry(js::get_str(&n, "parentId")).or_default().push(n); }
    let mut stack: Vec<(JsValue, String, bool, bool)> = Vec::new();
    if let Some(roots) = children.get("") {
        for (i, n) in roots.iter().enumerate().rev() { stack.push((n.clone(), String::new(), i + 1 == roots.len(), roots.len() > 1)); }
    }
    let out = js_sys::Array::new();
    while let Some((node, prefix, last, forked)) = stack.pop() {
        let id = js::get_str(&node, "id");
        let kind = js::get_str(&node, "kind");
        let role = js::get_str(&node, "role");
        let tone = if kind == "context_window" && (role == "user" || role == "assistant") { role.as_str() } else { "other" };
        let label = if role.is_empty() { kind.replace('_', " ") } else { role.clone() };
        let content = js::get_str(&node, "content").split_whitespace().collect::<Vec<_>>().join(" ");
        let row = js::obj();
        js::set_str(&row, "id", &id);
        js::set_str(&row, "prefix", &format!("{}{}", prefix, if forked { if last { "└─ " } else { "├─ " } } else { "• " }));
        js::set_str(&row, "kind", &label);
        js::set_str(&row, "tone", tone);
        js::set_str(&row, "text", &content);
        js::set_bool(&row, "current", id == cursor);
        out.push(&row);
        if let Some(kids) = children.get(&id) {
            let continuation = if forked { format!("{}{}", prefix, if last { "   " } else { "│  " }) } else { prefix.clone() };
            for (i, child) in kids.iter().enumerate().rev() {
                stack.push((child.clone(), continuation.clone(), i + 1 == kids.len(), kids.len() > 1));
            }
        }
    }
    out.into()
}

pub fn tree(st: &str) -> Node {
    m("div").class("history-tree")
        .attr("role", "listbox")
        .attr("aria-label", "Conversation history tree")
        .on("keydown.up", &format!("$event.preventDefault(); {st}.treeMove(-1)"))
        .on("keydown.down", &format!("$event.preventDefault(); {st}.treeMove(1)"))
        .on("keydown.escape", &format!("$event.preventDefault(); {st}.treeClose()"))
        .kids(vec![
            m("div").class("history-tree-status").show(&format!("{st}.treeLoading")).child(lit("Loading history…")),
            m("div").class("history-tree-status").show(&format!("!{st}.treeLoading && !{st}.treeRows.length")).child(lit("No history yet.")),
            m("button").attr("type", "button").attr("role", "option")
                .for_each(&format!("(n, i) in {st}.treeRows")).key("n.id")
                .bind("class", &format!("('history-tree-row is-' + n.tone + (i === {st}.treeIndex ? ' is-selected' : ''))"))
                .bind("aria-selected", &format!("(i === {st}.treeIndex ? 'true' : 'false')"))
                .on("mouseenter", &format!("{st}.treeSelect(i)"))
                .on("click", &format!("{st}.treeChoose(n.id)"))
                .kids(vec![m("span").class("history-tree-prefix").text("n.prefix"), m("span").class("history-tree-kind").text("n.kind + ': '"), m("span").class("history-tree-text").text("n.text"), m("span").class("history-tree-current").show("n.current").child(lit("●"))]),
        ])
}

pub fn select(st: &JsValue, index: i64) {
    let rows = js_sys::Array::from(&js::get(st, "treeRows"));
    if rows.length() == 0 { return; }
    js::set_num(st, "treeIndex", index.clamp(0, rows.length() as i64 - 1) as f64);
    mjs::redraw();
    let document = js::get(&js_sys::global(), "document");
    let selected = js::call(&document, "querySelector", &[JsValue::from_str(".history-tree-row.is-selected")]);
    if !js::is_missing(&selected) {
        let options = js::obj(); js::set_str(&options, "block", "nearest");
        js::call(&selected, "scrollIntoView", &[options.into()]);
    }
}

pub fn focus_composer() {
    let document = js::get(&js_sys::global(), "document");
    let input = js::call(&document, "querySelector", &[JsValue::from_str(".composer .draft-ta")]);
    if !js::is_missing(&input) { js::call(&input, "focus", &[]); }
}

pub fn demo() -> Node { m("div").class("history-tree-demo").child(tree(crate::components::samples::EXPR)) }
pub fn register() { css::inject("history-tree", CSS); }

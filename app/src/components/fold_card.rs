use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .fold-card {
  width: 100%;
  margin: 4px 0 8px;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid color-mix(in srgb, var(--agent-accent, #a78bfa) 35%, transparent);
  background: color-mix(in srgb, var(--agent-accent, #a78bfa) 12%, transparent);
  color: color-mix(in srgb, var(--agent-accent, #a78bfa) 55%, #ffffff);
  cursor: pointer;
}
:is(#sb, #home) .fold-head {
  display: flex; align-items: center; gap: 6px;
  margin-bottom: 6px;
  font-size: 12px; font-weight: 600;
  letter-spacing: 0.02em; text-transform: uppercase;
  color: color-mix(in srgb, var(--agent-accent, #a78bfa) 72%, #ffffff);
}
:is(#sb, #home) .fold-note {
  font-weight: 400; text-transform: none; letter-spacing: 0;
  opacity: .75; flex: 0 0 auto;
}
:is(#sb, #home) .fold-body {
  font-size: 13px; line-height: 1.5;
  word-break: break-word;
}
:is(#sb, #home) .fold-pre {
  margin: 0; white-space: pre-wrap; font-family: var(--mono, monospace);
  font-size: 12px; max-height: 60vh; overflow: auto;
}
:is(#sb, #home) .fold-card.is-err {
  border-color: rgba(217, 83, 79, .4);
  background: rgba(217, 83, 79, .1);
}
:is(#sb, #home) .fold-card.is-err .fold-head { color: #e8908c; }
:is(#sb, #home) .fold-card.is-err .fold-body { color: #e8b4b1; }

:is(#sb, #home) .fold-card.is-collapsed {
  display: flex; align-items: baseline; gap: 6px;
  min-width: 0;
  margin: 2px 0; padding: 0;
  background: transparent; border: 0; border-radius: 0;
}
:is(#sb, #home) .fold-card.is-collapsed .fold-head {
  flex: 0 0 auto; margin-bottom: 0; min-width: 0;
}
:is(#sb, #home) .fold-card.is-collapsed .fold-body {
  flex: 1 1 auto; min-width: 0;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  max-height: none;
}
:is(#sb, #home) .fold-card.is-collapsed .fold-body * {
  display: inline; margin: 0; padding: 0;
  white-space: inherit; border: 0; background: transparent;
}
:is(#sb, #home) .fold-card.is-collapsed .fold-body br { display: none; }

.fold-card-demo { width: min(560px, 100%); }
.fold-card-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub fn head(icon: &str, label: &str, note: Option<&str>) -> Vec<Node> {
    let mut out = vec![
        Node::icon(icon).class(&format!("ph ph-{icon}")),
        m("span").class("fold-label").child(lit(label)),
    ];
    if let Some(expr) = note {
        out.push(m("span").class("fold-note").text(expr));
    }
    out
}

pub fn head_dyn(icon_expr: &str, label_expr: &str, note: Option<&str>) -> Vec<Node> {
    let mut out = vec![
        m("i").bind("class", icon_expr),
        m("span").class("fold-label").text(label_expr),
    ];
    if let Some(expr) = note {
        out.push(m("span").class("fold-note").text(expr));
    }
    out
}

pub fn fold(
    st: &str,
    v: &str,
    class: &str,
    icon: &str,
    label: &str,
    note: Option<&str>,
    body: Node,
) -> Node {
    fold_as(st, v, class, "", head(icon, label, note), body)
}

pub fn fold_as(st: &str, v: &str, class: &str, extra: &str, head: Vec<Node>, body: Node) -> Node {
    let collapse = format!("({v}.expanded ? '' : 'is-collapsed')");
    let state = if extra.is_empty() {
        collapse
    } else {
        format!("({collapse} + ' ' + {extra})")
    };
    m("div")
        .class(&format!("fold-card {class}"))
        .bind("class", &state)
        .attr("title", "Click to expand or collapse")
        .on("click", &format!("{st}.toggleCard({v}.id)"))
        .kids(vec![m("div").class("fold-head").kids(head), body])
}

pub fn pre(expr: &str) -> Node {
    m("pre").class("fold-body fold-pre").text(expr)
}

pub fn demo() -> Node {
    let st = crate::components::samples::EXPR;
    let v = crate::components::samples::row("reasoning");
    m("div").class("fold-card-demo").child(
        m("div").class("mjs-chat mode-sidebar").child(fold(
            st,
            &v,
            "fold-reasoning",
            "brain",
            "Reasoning",
            None,
            pre(&format!("{v}.text")),
        )),
    )
}

pub fn register() {
    css::inject("fold-card", CSS);
}

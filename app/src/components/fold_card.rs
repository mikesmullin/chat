//! `fold card` — a card that starts shut and opens on a click anywhere on it.
//!
//! The primitive every *evidence* row is built from: the system prompt, the
//! prompt as the agent received it, the engagement decision, the reasoning,
//! each tool call and each result. All of them answer "why did it say that?"
//! — essential when an answer needs explaining, noise the rest of the time.
//! So they collapse to one line, and the whole card is the toggle, because a
//! line that small has no room for a separate affordance.
//!
//! The cards that use it ([`crate::components::reasoning_card`],
//! [`crate::components::tool_call_card`],
//! [`crate::components::tool_response_card`]) supply a glyph, a label, an
//! optional note for the collapsed line, and a body. They do not restate the
//! collapse behaviour, the click target or the tint — this is the one place
//! those are written.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
/* ---- foldable evidence cards --------------------------------------------
   Tinted in the speaking agent's own colour, which the row sets as
   `--agent-accent`. */
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
/* Something went wrong: not in the agent's colour, because the point is the
   failure and a tint cannot say that. One modifier, worn by a denied or
   failed tool result and by an error row alike. */
:is(#sb, #home) .fold-card.is-err {
  border-color: rgba(217, 83, 79, .4);
  background: rgba(217, 83, 79, .1);
}
:is(#sb, #home) .fold-card.is-err .fold-head { color: #e8908c; }
:is(#sb, #home) .fold-card.is-err .fold-body { color: #e8b4b1; }

/* Waiting on the person: amber, the one card that is a question. */
:is(#sb, #home) .fold-card.is-ask {
  border-color: rgba(217, 119, 6, .55);
  background: rgba(120, 53, 15, .35);
}
:is(#sb, #home) .fold-card.is-ask .fold-head { color: #fcd34d; }

/* Collapsed: one line, no chrome at all — just the tinted text. */
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
/* The body may be rendered markdown, so it is full of block elements. One
   line only really is one line once they are inline and their own
   white-space rules (a <pre> keeps its newlines) give way to the clamp. */
:is(#sb, #home) .fold-card.is-collapsed .fold-body * {
  display: inline; margin: 0; padding: 0;
  white-space: inherit; border: 0; background: transparent;
}
/* A <br> would break the line regardless of white-space. */
:is(#sb, #home) .fold-card.is-collapsed .fold-body br { display: none; }

/* demo framing */
.fold-card-demo { width: min(560px, 100%); }
.fold-card-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// The head's three slots, written out: glyph, label, and the note that
/// rides the collapsed line beside it.
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

/// The same head, where the row decides the glyph and the label — a tool
/// result names the tool it ran and shows whether it worked.
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

/// A fold card over the row `v` in store `st`.
///
/// `class` is the card's own modifier (`fold-reasoning`, `fold-tool-call`),
/// `note` the expression shown on the collapsed line beside the label.
/// Whether it is drawn at all is the caller's business — a card shows itself
/// for one `kind`, and that condition belongs with the card, not here.
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

/// [`fold`] with the head written out, and an expression for the classes the
/// row's own state adds.
///
/// `extra` exists because the collapse state is bound here, and a `:class`
/// set twice on one node replaces rather than merges — so a card that is
/// also red when it failed, or amber while it waits, has to say so through
/// this rather than binding `class` again on the way out.
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

/// A `<pre>` body — the shape most evidence takes.
pub fn pre(expr: &str) -> Node {
    m("pre").class("fold-body fold-pre").text(expr)
}

/// standalone demo: the primitive itself, over the reasoning sample.
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

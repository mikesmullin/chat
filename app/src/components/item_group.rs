//! `item-group` — a labelled group: state-coloured name + inline counter,
//! then a list of rows.
//!
//! Each row = robot icon + bold name + truncated summary + relative time + a
//! per-state symbol — the shared ring of `session_state`: the state is its
//! colour, the session's context usage is its fill.
//!
//! `tone` is `done` / `working` / `idle`; it drives both the group's colour and
//! every row's symbol (a group is homogenous in the storybook).
//!
//! Owns its CSS; injected at boot by `components::register`.

use rustex1_kit::h::{lit, m, Node};
use crate::components::session_state;

/// the item group's styles (group + head + list + rows + state symbols' layout).
pub const CSS: &str = r#"
.item-group { color: var(--muted); margin-bottom: 18px; max-width: 480px; width: 100%; }
.item-group.idle { color: var(--gray-dim); }
.item-group.working { color: var(--pastel-amber); }
.item-group.done { color: var(--accent); }
.item-group:last-child { margin-bottom: 0; }

.item-head {
  display: flex; align-items: center; gap: 6px; margin-bottom: 6px;
  font-size: 12px;
}
.item-head-name { font-weight: 600; }
.item-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex; flex-direction: column; gap: 2px;
}

.item-row {
  display: flex; align-items: center; gap: 10px;
  padding: 6px 0; border-radius: 8px; color: var(--text);
  transition: background .13s;
}
.item-row:hover { background: var(--accent-glow); }
.item-icon { font-size: 16px; line-height: 1; color: var(--muted); flex: 0 0 auto; transition: color .13s; }
.item-row:hover .item-icon { color: var(--accent-soft); }
.item-name { font-weight: 600; flex: 0 0 auto; }
.item-summary {
  flex: 1 1 auto; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  color: var(--muted); font-size: 13px; padding-right: 4px;
}
.item-meta { display: flex; align-items: center; gap: 10px; flex: 0 0 auto; }
"#;

/// one agent row.
fn row(name: &str, summary: &str, reltime: &str, tone: &str) -> Node {
    // per-state symbol — the shared `uiSessionState` unit, so the row's
    // symbol is literally the same markup featured at /storybook/session-state.
    let symbol = session_state::symbol(tone);
    m("li")
        .class("item-row")
        .kids(vec![
            Node::icon("robot"),
            m("span").class("item-name").child(lit(name)),
            m("span").class("item-summary").child(lit(summary)),
            m("span").class("item-meta").kids(vec![
                m("span").class("relative-timestamp").child(lit(reltime)),
                symbol,
            ]),
        ])
}

/// the group header: state-coloured name + the inline counter widget.
fn head(label: &str, count: &str) -> Node {
    m("div")
        .class("item-head")
        .kids(vec![
            m("span").class("item-head-name").child(lit(label)),
            Node::host("uiCounter", &[]).bind("value", count),
        ])
}

/// build one item-group tree.
pub fn item_group(tone: &str, label: &str, count: &str, rows: &[(String, String, String)]) -> Node {
    m("div")
        .class(&format!("item-group {}", tone))
        .kids(vec![
            head(label, count),
            m("ul")
                .class("item-list")
                .kids(rows.iter().map(|r| row(&r.0, &r.1, &r.2, tone)).collect()),
        ])
}

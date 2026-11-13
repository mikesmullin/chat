//! `session history` — the left-sidebar session list: a `History` header
//! (trash-all on hover) plus one row per past session (title + agent · model
//! meta, per-row delete on hover, active row highlighted).
//!
//! Verbatim app classes inside `.mjs-chat.mode-sidebar` — the `--mjs-*`
//! tokens inherit the storybook charcoal through `#sb`, so the contract
//! holds with no restyle. Static showcase (buttons inert, like the tab
//! demo's close buttons).

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
/* demo framing: fixed panel size; the sidebar divider is an edge against a
   neighbouring column, not a property of the panel, so it is dropped here */
.session-history-demo { width: 260px; height: 420px; }
.session-history-demo .mjs-chat.mode-sidebar { border-left: 0; }
.session-history-demo .mjs-chat { height: 100%; }
"#;

pub struct SessionRow {
    pub title: &'static str,
    pub agent: &'static str,
    pub model: &'static str,
    pub active: bool,
}

pub const SAMPLES: &[SessionRow] = &[
    SessionRow { title: "Migrate auth service", agent: "cowork", model: "sonnet", active: true },
    SessionRow { title: "Fix session timeout", agent: "cowork", model: "opus", active: false },
    SessionRow { title: "Retry logic refactor", agent: "cowork", model: "haiku", active: false },
];

fn row(row: &SessionRow) -> Node {
    m("div")
        .class(if row.active {
            "history-item-row active"
        } else {
            "history-item-row"
        })
        .kids(vec![
            m("button")
                .class("history-item")
                .attr("type", "button")
                .kids(vec![
                    m("div").class("history-title").child(lit(row.title)),
                    m("div").class("history-meta").kids(vec![
                        m("span").child(lit(row.agent)),
                        m("span").child(lit(format!(" · {}", row.model).as_str())),
                    ]),
                ]),
            m("button")
                .class("trash-one")
                .attr("type", "button")
                .attr("title", "Delete this session")
                .child(Node::icon("trash")),
        ])
}

/// the panel: header + rows. `rows` keeps the shape reusable.
pub fn session_history(rows: &[SessionRow]) -> Node {
    m("div")
        .class("mjs-chat mode-sidebar")
        .attr("aria-label", "Session history")
        .child(
            m("aside").class("history").kids(vec![
                m("div").class("history-header").kids(vec![
                    lit("History"),
                    m("button")
                        .class("icon-btn trash-all")
                        .attr("type", "button")
                        .attr("title", "Delete all sessions")
                        .child(Node::icon("trash")),
                ]),
                m("div")
                    .class("history-list")
                    .kids(rows.iter().map(row).collect()),
            ]),
        )
}

/// standalone demo: three sessions, the first active.
pub fn demo() -> Node {
    m("div")
        .class("session-history-demo")
        .child(session_history(SAMPLES))
}

pub fn register() {
    css::inject("session-history", CSS);
}

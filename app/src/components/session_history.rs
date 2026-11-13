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
    css::inject("session-history-live", LIVE_CSS);
}

// ===========================================================================
// Live variant — the same classes, bound to `$store.live`.
// ===========================================================================

/// The history rail driven by the session list from angela.
///
/// Same class names as the static demo above (the contract), so the design
/// guide and the running app stay one thing. `@click` sits on the row and on
/// its direct child button, never inside a sub-component — the lesson from
/// the tab bar's close button.
pub fn session_history_live() -> Node {
    let st = "$store.live";
    m("div")
        .class("mjs-chat mode-sidebar")
        .attr("aria-label", "Session history")
        .child(
            m("aside").class("history").kids(vec![
                m("div").class("history-header").child(lit("History")),
                m("div").class("history-list").kids(vec![
                    m("div")
                        .class("history-empty")
                        .show(&format!("{st}.sessions.length === 0"))
                        .child(lit("No sessions yet")),
                    m("div")
                        .class("history-item-row")
                        .for_each(&format!("s in {st}.sessions"))
                        .key("s.id")
                        .bind("class", &format!("({st}.activeId === s.id ? 'active' : '')"))
                        .on("click", &format!("{st}.openSession(s.id)"))
                        .kids(vec![
                            m("button")
                                .class("history-item")
                                .attr("type", "button")
                                .kids(vec![
                                    m("div").class("history-title").text("s.title"),
                                    // When it was started, not what ran it:
                                    // the agent and model are the same on
                                    // every row, so they carried no signal.
                                    m("div")
                                        .class("history-meta")
                                        .child(m("span").class("relative-timestamp").text(
                                            &format!("{st}.relTime(s.createdAt)"),
                                        )),
                                ]),
                            m("button")
                                .class("trash-one")
                                .attr("type", "button")
                                .attr("title", "Delete this session")
                                .attr("aria-label", "Delete this session")
                                .on("click.stop", &format!("{st}.deleteSession(s.id)"))
                                .child(Node::icon("trash")),
                        ]),
                ]),
            ]),
        )
}


/// Live-rail styles: the trash button appears on hover (and on keyboard
/// focus, so it is reachable without a pointer).
pub const LIVE_CSS: &str = r#"
.history-item-row .trash-one {
  opacity: 0;
  transition: opacity .12s, color .12s;
  background: transparent;
  border: 0;
  color: var(--muted);
  cursor: pointer;
  padding: 0 6px;
  flex: 0 0 auto;
}
.history-item-row:hover .trash-one,
.history-item-row .trash-one:focus-visible { opacity: 1; }
.history-item-row .trash-one:hover { color: var(--pastel-red); }
.history-empty { color: var(--muted); font-size: 12px; padding: 10px 12px; }

/* The rail is one flat surface: the chat skin's sidebar divider and the
   header rule are edges against a neighbouring column, which this layout
   does not have. */
.home-rail .mjs-chat,
.home-rail .mjs-chat.mode-sidebar { border-left: 0; }
.home-rail .history-header { border-bottom: 0; }

/* Match the Agents menu title exactly — they are siblings in the same rail,
   so two different title treatments read as two different kinds of thing. */
.home-rail .history-header {
  font-size: 13px;
  font-weight: 400;
  letter-spacing: normal;
  text-transform: none;
  color: var(--gray-dim);
  padding: 10px 14px 8px;
}

/* The rail holds two sections; the session list is capped so a long history
   cannot push the agents menu off screen. ~5 rows, then it scrolls. The
   scrollbar is the thin arrow-less one from the global layer. */
.home-rail .history-list {
  max-height: 290px;
  overflow-y: auto;
  overflow-x: hidden;
}
"#;

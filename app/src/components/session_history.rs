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
/* ---- the history rail ----
   Out of `web/styles.css`, where these were the only rules dressing
   this component — a look kept in a global sheet outlives the markup
   it was written for. Copied unchanged, still scoped under
   `.mjs-chat`, so neither specificity nor the cascade moved with
   them. */
.mjs-chat .history {
  width: 260px;
  /* `min-width`, not `flex-shrink: 0`. The guard is about WIDTH — this sits
     beside the transcript and must not be squeezed — but `flex-shrink`
     applies to whichever axis the parent runs in, and in the app's left rail
     the parent is a column. There it read as "never be shorter than my
     content", so the list spilled out of its box and drew over whatever came
     after it in the rail. `min-width` says the same thing about the axis it
     actually means. */
  min-width: 260px;
  /* The flip side: let it be shorter than its content, so `.history-list`
     below can be the thing that scrolls. Without this the default
     `min-height: auto` keeps it at full content height whatever the parent
     offers. */
  min-height: 0;
  background: var(--mjs-panel);
  border-right: 1px solid var(--mjs-line);
  display: flex;
  flex-direction: column;
  transition: width 0.15s ease, opacity 0.15s ease;
}
.mjs-chat .history.collapsed {
  width: 0;
  /* The width guard above would otherwise hold the collapsed rail open. */
  min-width: 0;
  opacity: 0;
  overflow: hidden;
  border: 0;
}
.mjs-chat .history-header {
  padding: 12px 14px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--mjs-dim);
  border-bottom: 1px solid var(--mjs-line);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.mjs-chat .history-header .trash-all {
  opacity: 0;
  width: 28px;
  height: 28px;
  font-size: 13px;
  transition: opacity 0.12s ease;
}
.mjs-chat .history-header:hover .trash-all { opacity: 0.85; }
.mjs-chat .history-header .trash-all:hover { opacity: 1; border-color: var(--mjs-err); color: var(--mjs-err); }
.mjs-chat .history-list { flex: 1; overflow-y: auto; padding: 8px; }
.mjs-chat .history-item-row {
  display: flex;
  align-items: stretch;
  gap: 2px;
  margin-bottom: 4px;
  border-radius: 8px;
  border: 1px solid transparent;
}
.mjs-chat .history-item-row:hover { background: var(--mjs-panel-2); }
.mjs-chat .history-item-row.active {
  border-color: var(--mjs-accent);
  background: color-mix(in srgb, var(--mjs-accent) 12%, transparent);
}
.mjs-chat .history-item {
  flex: 1;
  min-width: 0;
  text-align: left;
  background: transparent;
  border: 0;
  color: var(--mjs-ink);
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 13px;
}
.mjs-chat .history-title { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.mjs-chat .history-meta {
  font-size: 11px;
  color: var(--mjs-dim);
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.mjs-chat .trash-one {
  opacity: 0;
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  margin: 4px 4px 4px 0;
  border: 0;
  background: transparent;
  color: var(--mjs-dim);
  border-radius: 6px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.mjs-chat .history-item-row:hover .trash-one { opacity: 0.85; }
.mjs-chat .trash-one:hover { opacity: 1 !important; color: var(--mjs-err); background: rgba(239, 68, 68, 0.12); }
@media (max-width: 720px) {
  .mjs-chat.mode-fullscreen .history { display: none; }
}

/* demo framing: fixed panel size; the sidebar divider is an edge against a
   neighbouring column, not a property of the panel, so it is dropped here */
.session-history-demo { width: 260px; height: 420px; }
.session-history-demo .mjs-chat.mode-sidebar { border-left: 0; }
.session-history-demo .mjs-chat { height: 100%; }
"#;

/// standalone demo — the rail the app draws, over the sample sessions.
pub fn demo() -> Node {
    m("div")
        .class("session-history-demo")
        .child(session_history(crate::components::samples::EXPR))
}

pub fn register() {
    css::inject("session-history", CSS);
    css::inject("session-history-live", LIVE_CSS);
}

/// The history rail, over whichever store holds the session list.
///
/// One definition: `/` passes the live store, the design guide passes the
/// samples. `@click` sits on the row and on its direct child button, never
/// inside a sub-component — the lesson from the tab bar's close button.
pub fn session_history(st: &str) -> Node {
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

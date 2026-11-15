//! `agents-menu` — the Agents rail: a title, an add button, and the agents
//! defined so far.
//!
//! An agent used to be a file you edited in a terminal. It is a row now, and
//! this list is where it is created, found and opened. Clicking one shows it
//! in the third column, which is an **inspector**: what it holds is whatever
//! is currently selected elsewhere on the page, not a fixed panel.
//!
//! The storybook variant below it (`agents_menu_demo`) is the old static
//! design-guide composition, kept because it is what the design guide
//! documents for the Done/Working/Idle grouping.

use rustex1_kit::h::{lit, m, Node};

use crate::components::item_group::item_group;
use crate::css;

pub const CSS: &str = r#"
/* the "agents" menu title: normal weight, dim gray (near #666) */
.agents-header {
  font-size: 13px;
  font-weight: 400;
  color: var(--gray-dim);
  margin-bottom: 14px;
}
/* …and the live variant, which has a button across from it */
:is(#sb, #home) .agents-head {
  display: flex; align-items: center; justify-content: space-between;
  margin-bottom: 10px;
}
:is(#sb, #home) .agents-head .agents-header { margin-bottom: 0; }
:is(#sb, #home) .agents-add {
  width: 22px; height: 22px;
  display: inline-flex; align-items: center; justify-content: center;
  border: 0; border-radius: 6px;
  background: transparent; color: var(--gray-dim);
  cursor: pointer; font: inherit; line-height: 1;
}
:is(#sb, #home) .agents-add:hover { color: var(--accent); background: var(--panel-2); }

:is(#sb, #home) .agent-list { display: flex; flex-direction: column; gap: 2px; }
:is(#sb, #home) .agent-row {
  display: flex; align-items: center; gap: 8px;
  width: 100%; text-align: left;
  border: 0; border-radius: 8px;
  background: transparent; color: var(--gray);
  padding: 6px 8px; cursor: pointer; font: inherit;
}
:is(#sb, #home) .agent-row:hover { background: var(--panel-2); color: var(--fg); }
:is(#sb, #home) .agent-row.is-selected { background: var(--panel-2); color: var(--fg); }
:is(#sb, #home) .agent-row i { color: var(--agent-accent, #a78bfa); font-size: 14px; line-height: 18px; }
:is(#sb, #home) .agent-row .nm {
  flex: 1; min-width: 0; font-size: 13px;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
/* How it engages, as a one-glyph tell rather than a word per row. */
:is(#sb, #home) .agent-row .eng {
  flex: 0 0 auto; font-size: 11px; color: var(--gray-dim); line-height: 18px;
}
:is(#sb, #home) .agent-empty {
  font-size: 12px; color: var(--gray-dim); padding: 6px 8px; line-height: 1.5;
}
"#;

/// The live rail: title, `+`, and the defined agents.
pub fn agents_menu_live() -> Node {
    let st = "$store.agents";
    m("div")
        .class("agents-menu")
        // Pull the list once the rail exists.
        .init(&format!("{st}.loadAgents()"))
        .kids(vec![
            m("div").class("agents-head").kids(vec![
                m("p").class("agents-header").child(lit("Agents")),
                m("button")
                    .class("agents-add")
                    .attr("type", "button")
                    .attr("title", "Define a new agent")
                    .attr("aria-label", "Define a new agent")
                    .on("click", &format!("{st}.newAgent()"))
                    .child(Node::icon("plus").class("ph ph-plus")),
            ]),
            m("div").class("agent-list").kids(vec![
                m("button")
                    .for_each(&format!("a in {st}.agents"))
                    .key("a.name")
                    .bind(
                        "class",
                        &format!(
                            "('agent-row ' + (a.name === {st}.selected ? 'is-selected' : ''))"
                        ),
                    )
                    .on("click", &format!("{st}.select(a.name)"))
                    .bind("title", &format!("{st}.engagementLabel(a.engagement)"))
                    .bind("style", "('--agent-accent: ' + (a.accentColor || '#a78bfa'))")
                    .kids(vec![
                        m("i").class("ph ph-robot"),
                        m("span").class("nm").text("a.name"),
                        // One glyph per mode: a word per row would crowd out
                        // the name, which is what you are actually scanning.
                        m("i").bind(
                            "class",
                            "'eng ph ' + (a.engagement === 'engaged' ? 'ph-chats-circle' \
                              : a.engagement === 'relevant' ? 'ph-funnel' : 'ph-at')",
                        ),
                    ]),
                m("p")
                    .class("agent-empty")
                    .show(&format!("{st}.agents.length === 0"))
                    .child(lit("No agents yet. Use + to define one.")),
            ]),
        ])
}

/// the storybook's static composition (Done / Working / Idle).
pub fn agents_menu() -> Node {
    m("div")
        .class("agents-menu")
        .kids(vec![
            m("p").class("agents-header").child(lit("Agents")),
            item_group(
                "done",
                "Done",
                "2",
                &[
                    ("Eve".into(), "Finished the onboarding copy and tone guide".into(), "1h".into()),
                    ("Dan".into(), "Fixed the session timeout bug".into(), "45m".into()),
                ],
            ),
            item_group(
                "working",
                "Working",
                "1",
                &[
                    ("Carol".into(), "Migrating the auth service to the new schema".into(), "12m".into()),
                ],
            ),
            item_group(
                "idle",
                "Idle",
                "3",
                &[
                    ("Alice".into(), "Refactored the retry logic in the fetch client".into(), "20m".into()),
                    ("Bob".into(), "Waiting on CI for the cache patch".into(), "3m".into()),
                    ("Fay".into(), "Tidying the session store after the migration".into(), "45m".into()),
                ],
            ),
        ])
}

pub fn register() {
    css::inject("agents-menu", CSS);
}

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
use crate::components::rail_head;
use crate::css;

pub const CSS: &str = r#"
/* The title row is `rail_head`, shared with the Projects section: one
   definition of "a section of the left rail", not one per section. */

:is(#sb, #home) .agent-list { display: flex; flex-direction: column; gap: 2px; }
:is(#sb, #home) .agent-row {
  display: flex; align-items: center; gap: 8px;
  width: 100%; text-align: left;
  border: 0; border-radius: 8px;
  background: transparent; color: var(--gray);
  padding: 5px 8px; cursor: pointer; font: inherit; line-height: 18px;
}
:is(#sb, #home) .agent-row:hover { background: var(--panel-2); color: var(--fg); }
:is(#sb, #home) .agent-row.is-selected { background: var(--panel-2); color: var(--fg); }
:is(#sb, #home) .agent-row i { color: var(--agent-accent, #a78bfa); font-size: 14px; line-height: 18px; }
:is(#sb, #home) .agent-row .nm {
  flex: 1; min-width: 0; font-size: 12px;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
/* How it engages, as a one-glyph tell rather than a word per row. */
:is(#sb, #home) .agent-row .eng {
  flex: 0 0 auto; font-size: 11px; color: var(--gray-dim); line-height: 18px;
}
:is(#sb, #home) .agent-empty {
  font-size: 12px; color: var(--gray-dim); padding: 6px 8px; line-height: 1.5;
}
/* Narrow shell (see home.rs breakpoints): the row is a centred glyph, which
   is what tells agents apart — each one owns its icon. The `title` still
   says how it engages. */
@media (max-width: 900px) {
  :is(#sb, #home) .agent-row { justify-content: center; padding-left: 0; padding-right: 0; }
  :is(#sb, #home) .agent-row .nm,
  :is(#sb, #home) .agent-row .eng,
  :is(#sb, #home) .agent-empty { display: none; }
}
"#;

/// The live rail: title, `+`, and the defined agents.
pub fn agents_menu_live() -> Node {
    let st = "$store.agents";
    m("div")
        .class("agents-menu")
        // Pull the list once the rail exists.
        .init(&format!("{st}.loadAgents()"))
        .bind("class", "($store.home.agentsOpen ? '' : 'is-collapsed')")
        .kids(vec![
            rail_head::head(
                "Agents",
                "robot",
                Some(("Define a new agent", &format!("{st}.newAgent()"), "$store.ident.can('agent.create')")),
                Some("agents"),
            ),
            m("div").class("agent-list").show("$store.home.agentsOpen").kids(vec![
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
                        // Each agent as its own glyph: in a rail of a dozen
                        // identical robots the name is doing all the work.
                        Node::icon_dyn("(a.icon || 'robot')"),
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
                    .child(lit("No agents yet.")),
            ]),
        ])
}

/// the storybook's static composition (Done / Working / Idle). The head
/// collapses against the guide's own `home` store, like the live rail.
pub fn agents_menu() -> Node {
    m("div")
        .class("agents-menu")
        .bind("class", "($store.home.agentsOpen ? '' : 'is-collapsed')")
        .kids(vec![
            rail_head::head("Agents", "robot", None, Some("agents")),
            m("div").class("agent-list").show("$store.home.agentsOpen").kids(vec![
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
            ]),
        ])
}

pub fn register() {
    css::inject("agents-menu", CSS);
}

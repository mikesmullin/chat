use rustex1_kit::h::{lit, m, Node};

use crate::components::item_group::item_group;
use crate::components::rail_head;
use crate::css;

pub const CSS: &str = r#"

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
:is(#sb, #home) .agent-row .eng {
  flex: 0 0 auto; font-size: 11px; color: var(--gray-dim); line-height: 18px;
}
:is(#sb, #home) .agent-empty {
  font-size: 12px; color: var(--gray-dim); padding: 6px 8px; line-height: 1.5;
}
@media (max-width: 900px) {
  :is(#sb, #home) .agent-row { justify-content: center; padding-left: 0; padding-right: 0; }
  :is(#sb, #home) .agent-row .nm,
  :is(#sb, #home) .agent-row .eng,
  :is(#sb, #home) .agent-empty { display: none; }
}
"#;

pub fn agents_menu_live() -> Node {
    let st = "$store.agents";
    m("div")
        .class("agents-menu")
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
                        Node::icon_dyn("(a.icon || 'robot')"),
                        m("span").class("nm").text("a.name"),
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

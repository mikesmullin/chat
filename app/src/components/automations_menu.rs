//! `automations menu` — the third left-rail section: one row per scheduled
//! automation, timer icon + name + schedule hint. Clicking a row opens the
//! automation in the inspector; the `+` opens the New automation form.
//!
//! Same brick as the sections above it: the title row is shared `rail_head`
//! (with collapse), the rows mirror the agent rows' shape, and the body
//! hides on the same `$store.home` flag.

use rustex1_kit::h::{lit, m, Node};

use crate::components::rail_head;
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .automation-list { display: flex; flex-direction: column; gap: 2px; }
:is(#sb, #home) .automation-row {
  display: flex; align-items: center; gap: 8px;
  width: 100%; text-align: left;
  border: 0; border-radius: 8px;
  background: transparent; color: var(--gray);
  padding: 5px 8px; cursor: pointer; font: inherit; line-height: 18px;
}
:is(#sb, #home) .automation-row:hover { background: var(--panel-2); color: var(--fg); }
:is(#sb, #home) .automation-row i { color: var(--gray-dim); font-size: 14px; line-height: 18px; }
:is(#sb, #home) .automation-row .nm {
  flex: 1; min-width: 0; font-size: 12px;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
/* The cron is the tell, the way engagement is on an agent row: what it is
   set to, not a word per row. A paused automation reads dimmed. */
:is(#sb, #home) .automation-row .sched {
  flex: 0 0 auto; font-size: 11px; color: var(--gray-dim); line-height: 18px;
  font-variant-numeric: tabular-nums;
}
:is(#sb, #home) .automation-row.is-off { opacity: .45; }
:is(#sb, #home) .automation-empty {
  font-size: 12px; color: var(--gray-dim); padding: 6px 8px; line-height: 1.5;
}
/* Narrow shell (see home.rs breakpoints): the row is a centred glyph. */
@media (max-width: 900px) {
  :is(#sb, #home) .automation-row { justify-content: center; padding-left: 0; padding-right: 0; }
  :is(#sb, #home) .automation-row .nm,
  :is(#sb, #home) .automation-row .sched,
  :is(#sb, #home) .automation-empty { display: none; }
}
"#;

/// The live rail: title, `+`, and the scheduled automations.
pub fn automations_menu_live() -> Node {
    let st = "$store.live";
    m("div")
        .class("automations-menu")
        .kids(vec![
            rail_head::head(
                "Automations",
                "timer",
                Some(("New automation", &format!("{st}.createTarget('automation')"), "$store.ident.can('automation.create')")),
                Some("automations"),
            ),
            m("div").class("automation-list").show("$store.home.automationsOpen").kids(vec![
                m("button")
                    .for_each(&format!("a in {st}.automations"))
                    .key("a.id")
                    .bind(
                        "class",
                        "('automation-row ' + (a.enabled === false ? 'is-off' : ''))",
                    )
                    .on("click", &format!("{st}.editTarget('automation', a.id)"))
                    .bind("title", "(a.prompt || a.cron || a.name)")
                    .kids(vec![
                        Node::icon("timer"),
                        m("span").class("nm").text("a.name"),
                        m("span").class("sched").text("a.cron"),
                    ]),
                m("p")
                    .class("automation-empty")
                    .show(&format!("{st}.automations.length === 0"))
                    .child(lit("No automations yet.")),
            ]),
        ])
}

/// The storybook's static composition: one enabled, one paused.
pub fn demo() -> Node {
    m("div")
        .class("automations-menu")
        .kids(vec![
            rail_head::head("Automations", "timer", None, Some("automations")),
            m("div").class("automation-list").show("$store.home.automationsOpen").kids(vec![
                m("div").class("automation-row").kids(vec![
                    Node::icon("timer"),
                    m("span").class("nm").child(lit("issue-triage")),
                    m("span").class("sched").child(lit("0 9 * * 1-5")),
                ]),
                m("div").class("automation-row is-off").kids(vec![
                    Node::icon("timer"),
                    m("span").class("nm").child(lit("nightly-sweep")),
                    m("span").class("sched").child(lit("0 2 * * *")),
                ]),
            ]),
        ])
}

pub fn register() {
    css::inject("automations-menu", CSS);
}

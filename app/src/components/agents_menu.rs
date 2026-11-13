//! `uiAgentsMenu` — the composite "Agents Menu": a dim "Agents" header followed
//! by three item groups in attention order (Done 2 / Working 1 / Idle 3). Each
//! group is built with the `item_group` component, which reuses the leaf
//! widgets (counter, pie ring, state symbols).

use rustex1_kit::h::{lit, m, Node};
use crate::components::item_group::item_group;

/// the agents menu's styles (the "Agents" header).
pub const CSS: &str = r#"
/* the "agents" menu title: normal weight, dim gray (near #666) */
.agents-header {
  font-size: 13px;
  font-weight: 400;
  color: var(--gray-dim);
  margin-bottom: 14px;
}
"#;

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

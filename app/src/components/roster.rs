//! `roster` — who is in the conversation.
//!
//! A chat of two shows two chips and is otherwise out of the way. Seating a
//! second agent is what turns the conversation into a group, and that is a
//! real change in how every turn is run: with one agent the provider-side
//! history *is* the transcript, while with two or more no shared history can
//! exist, so each agent gets its own context and reads the room as text.
//! The bar says which mode you are in, because the difference is otherwise
//! invisible until someone replies oddly.
//!
//! Branches live here too. A fork point is a node with more than one child,
//! which is the only place the conversation genuinely diverged and so the
//! only place worth offering a switch.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .roster {
  display: flex; gap: 6px; align-items: center; flex-wrap: wrap;
  padding: 6px 10px; font-size: 11px;
}
:is(#sb, #home) .seat {
  display: inline-flex; gap: 5px; align-items: center;
  background: var(--panel-2); border-radius: 999px;
  padding: 3px 9px; line-height: 18px; color: var(--gray);
}
:is(#sb, #home) .seat i { line-height: 18px; font-size: 12px; }
:is(#sb, #home) .seat.is-user { color: var(--gray-dim); }
:is(#sb, #home) .seat.is-agent i { color: var(--accent); }
/* A seat that has been unseated keeps its messages, so it keeps its chip —
   struck through rather than removed, because its words are still in the
   transcript and attributed to it. */
:is(#sb, #home) .seat.is-off { opacity: .45; text-decoration: line-through; }
:is(#sb, #home) .seat-drop {
  border: 0; background: transparent; color: var(--gray-dim);
  cursor: pointer; padding: 0; line-height: 18px; font: inherit;
  opacity: 0; transition: opacity 100ms ease;
}
:is(#sb, #home) .seat:hover .seat-drop { opacity: 1; }
:is(#sb, #home) .seat-drop:hover { color: var(--danger, #d9534f); }
:is(#sb, #home) .roster-add {
  display: inline-flex; gap: 4px; align-items: center;
}
:is(#sb, #home) .roster-add input {
  width: 92px; background: var(--panel-2); border: 0; border-radius: 999px;
  padding: 3px 9px; color: var(--fg); font: inherit; line-height: 18px;
}
:is(#sb, #home) .roster-add input:focus { outline: none; }
:is(#sb, #home) .roster-add input::placeholder { color: var(--gray-dim); }
:is(#sb, #home) .roster-add button {
  border: 0; background: transparent; color: var(--gray-dim);
  cursor: pointer; line-height: 18px; padding: 0 2px; font: inherit;
}
:is(#sb, #home) .roster-add button:hover { color: var(--accent); }
:is(#sb, #home) .roster-mode { color: var(--gray-dim); margin-left: auto; }
:is(#sb, #home) .roster-mode.is-group { color: var(--accent); }

:is(#sb, #home) .branches {
  display: flex; gap: 6px; align-items: center; flex-wrap: wrap;
  padding: 0 10px 6px; font-size: 11px; color: var(--gray-dim);
}
:is(#sb, #home) .branch {
  display: inline-flex; gap: 5px; align-items: center;
  background: var(--panel-2); border: 0; border-radius: 7px;
  padding: 3px 9px; color: var(--gray); cursor: pointer;
  font: inherit; line-height: 18px; max-width: 320px;
}
:is(#sb, #home) .branch:hover { color: var(--fg); }
:is(#sb, #home) .branch.is-current { color: var(--accent); }
:is(#sb, #home) .branch span.txt {
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
"#;

/// The seats, plus the control that adds one.
pub fn roster() -> Node {
    let st = "$store.live";
    m("div")
        .class("roster")
        .show(&format!("{st}.activeId"))
        .kids(vec![
            m("span")
                .for_each(&format!("p in {st}.participants"))
                .key("p.id")
                .bind(
                    "class",
                    "('seat ' + (p.kind === 'user' ? 'is-user' : 'is-agent') \
                      + (p.active ? '' : ' is-off'))",
                )
                .kids(vec![
                    m("i").bind(
                        "class",
                        "'ph ' + (p.kind === 'user' ? 'ph-user' : 'ph-robot')",
                    ),
                    m("span").text("'@' + p.handle"),
                    // The human seat is not optional.
                    m("button")
                        .class("seat-drop")
                        .iff("p.kind === 'agent'")
                        .attr("title", "Unseat")
                        .on("click", &format!("{st}.removeAgent(p.id)"))
                        .child(Node::icon("x").class("ph ph-x")),
                ]),
            m("span").class("roster-add").kids(vec![
                m("input")
                    .attr("type", "text")
                    .attr("placeholder", "add agent")
                    .model(&format!("{st}.newAgent"))
                    // Enter is the obvious gesture for a one-field form.
                    .on("keydown", &format!("($event.key === 'Enter') && {st}.addAgent()")),
                m("button")
                    .attr("title", "Seat another agent")
                    .on("click", &format!("{st}.addAgent()"))
                    .child(Node::icon("plus").class("ph ph-plus")),
            ]),
            // Which turn path the server will take, named out loud.
            m("span")
                .bind("class", &format!("('roster-mode ' + ({st}.isGroup ? 'is-group' : ''))"))
                .text(&format!(
                    "({st}.isGroup ? 'group · ' + {st}.agentCount + ' agents' : 'one agent')"
                )),
        ])
}

/// The fork points, when the conversation has any.
pub fn branches() -> Node {
    let st = "$store.live";
    m("div")
        .class("branches")
        .show(&format!("{st}.hasBranches"))
        .kids(vec![
            m("span").child(lit("Branches:")),
            m("button")
                .for_each(&format!("b in {st}.branches"))
                .key("b.id")
                .bind("class", "('branch ' + (b.current ? 'is-current' : ''))")
                .on("click", &format!("{st}.switchBranch(b.tip || b.id)"))
                .kids(vec![
                    Node::icon("git-branch").class("ph ph-git-branch"),
                    m("span").text("b.n + '/' + b.of"),
                    m("span").class("txt").text("b.text"),
                ]),
        ])
}

pub fn register() {
    css::inject("roster", CSS);
}

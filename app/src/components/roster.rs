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
:is(#sb, #home) .seat.is-agent { color: var(--agent-accent, #a78bfa); }
:is(#sb, #home) .seat.is-agent i { color: var(--agent-accent, #a78bfa); }
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
:is(#sb, #home) .roster-mode { color: var(--gray-dim); margin-left: auto; }
:is(#sb, #home) .roster-hint { color: var(--gray-dim); line-height: 18px; }
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
                .for_each(&format!("p in {st}.others"))
                .key("p.id")
                .bind(
                    "class",
                    "('seat ' + (p.kind === 'user' ? 'is-user' : 'is-agent') \
                      + (p.active ? '' : ' is-off'))",
                )
                // Each seat wears its own colour, so the roster and the
                // transcript name the same agent the same way.
                .bind("style", "('--agent-accent: ' + (p.accentColor || '#a78bfa'))")
                .kids(vec![
                    m("i").bind(
                        "class",
                        "'ph ' + (p.kind === 'user' ? 'ph-user' : 'ph-robot')",
                    ),
                    // No `@`: the sigil is command syntax, not part of the
                    // name. The icon and the colour say "participant".
                    m("span").text("p.handle"),
                    // The human seat is not optional.
                    m("button")
                        .class("seat-drop")
                        .iff("p.kind === 'agent'")
                        .attr("title", "Unseat")
                        .on("click", &format!("{st}.removeAgent(p.id)"))
                        .child(Node::icon("x").class("ph ph-x")),
                ]),
            // An empty bar with a text box in it does not tell you what to
            // do. The command that fixes it does.
            m("span")
                .class("roster-hint")
                .show(&format!("{st}.others.length === 0"))
                .child(lit("No agents yet — type /invite @name")),
            // Which turn path the server will take, named out loud.
            m("span")
                .bind("class", &format!("('roster-mode ' + ({st}.isGroup ? 'is-group' : ''))"))
                // Nothing to say when the room is empty: the hint beside it
                // already covers that case, and "one agent" would be a lie.
                .text(&format!(
                    "({st}.agentCount === 0 ? ''                       : {st}.isGroup ? 'group · ' + {st}.agentCount + ' agents' : 'one agent')"
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

//! `approval card` — a tool the allowlist does not permit, waiting on you.
//!
//! The one card that is a **question**. Everything else in the transcript
//! reports something that already happened; this one holds a turn still
//! while it waits for an answer, which is why it is the only row that opens
//! itself and wears amber rather than the agent's colour.
//!
//! Otherwise it is a [`fold_card`](crate::components::fold_card) like the
//! rest: same plate, same collapse, same one-line summary once it is
//! answered — because an approval you have already given is evidence, and
//! evidence folds away.
//!
//! The decision goes back through the store (`decideApproval`), not straight
//! to the server, so the guide can show the card working without a session
//! behind it.

use rustex1_kit::h::{lit, m, Node};

use crate::components::{fold_card, samples, session};
use crate::css;

pub const CSS: &str = r#"
/* The three answers. Tinted plates rather than solid buttons: the card is
   already a loud amber, and three saturated blocks inside it read as an
   alert rather than a question. Deny is not the dangerous one — carrying on
   is — so the emphasis sits on Allow, and Always carries a hint that it
   stops asking. */
:is(#sb, #home) .approval-actions {
  display: flex; flex-wrap: wrap; gap: 8px;
  margin-top: 10px;
}
:is(#sb, #home) .approval-actions button {
  appearance: none;
  border-radius: 8px;
  padding: 5px 12px;
  font: 600 12px/1.5 var(--sans, ui-sans-serif, system-ui, sans-serif);
  cursor: pointer;
  display: inline-flex; align-items: center; gap: 6px;
  transition: background 120ms ease, border-color 120ms ease;
}
:is(#sb, #home) .btn-allow {
  border: 1px solid rgba(74, 222, 128, .5);
  background: rgba(22, 101, 52, .35);
  color: #bbf7d0;
}
:is(#sb, #home) .btn-allow:hover { background: rgba(22, 101, 52, .6); }
:is(#sb, #home) .btn-always {
  border: 1px solid rgba(96, 165, 250, .45);
  background: rgba(30, 58, 95, .35);
  color: #bfdbfe;
}
:is(#sb, #home) .btn-always:hover { background: rgba(30, 58, 95, .6); }
:is(#sb, #home) .btn-deny {
  border: 1px solid rgba(248, 113, 113, .45);
  background: rgba(127, 29, 29, .3);
  color: #fecaca;
}
:is(#sb, #home) .btn-deny:hover { background: rgba(127, 29, 29, .55); }

/* What was decided, once it has been. Replaces the buttons in place rather
   than adding a line below them, so the card does not grow as it resolves. */
:is(#sb, #home) .approval-verdict {
  display: inline-flex; align-items: center; gap: 6px;
  margin-top: 10px;
  font-size: 12px; font-weight: 600;
  color: var(--gray-dim);
}
:is(#sb, #home) .approval-verdict.is-allowed { color: #86efac; }
:is(#sb, #home) .approval-verdict.is-denied { color: #fca5a5; }

/* An answered card is history: it collapses like any other evidence row, so
   the amber goes with it. */
:is(#sb, #home) .fold-approval.is-done:not(.is-collapsed) {
  border-color: color-mix(in srgb, var(--agent-accent, #a78bfa) 35%, transparent);
  background: color-mix(in srgb, var(--agent-accent, #a78bfa) 12%, transparent);
}

.approval-demo { width: min(560px, 100%); }
.approval-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// What this kind contributes to a message block.
///
/// `click.stop` on every button: the whole card is the fold toggle, so a
/// plain click would answer the question *and* shut the card on top of the
/// answer.
pub fn parts(st: &str, v: &str) -> Vec<Node> {
    let decide = |label: &str, class: &str, icon: &str, verdict: &str| {
        m("button")
            .class(class)
            .attr("type", "button")
            .on("click.stop", &format!("{st}.decideApproval({v}.id, '{verdict}')"))
            .kids(vec![
                Node::icon(icon).class(&format!("ph ph-{icon}")),
                m("span").child(lit(label)),
            ])
    };
    // The wrapper carries `fold-body` so the collapsed rules clamp the whole
    // card to one line — buttons, verdict and all. Without it the plate
    // folds but its contents stay stacked underneath, which reads as a card
    // that failed to close.
    let body = m("div").class("fold-body").kids(vec![
        // What it wants to run. The command line for a terminal tool, the
        // arguments for anything else — the same string the allowlist was
        // matched against, so "why was this not permitted?" is answerable
        // from the card.
        m("pre").class("fold-pre").text(&format!(
            "{v}.command || ({v}.args ? JSON.stringify({v}.args, null, 2) : '')"
        )),
        m("div")
            .class("approval-actions")
            .show(&format!("{v}.status === 'pending'"))
            .kids(vec![
                decide("Allow once", "btn-allow", "check", "allow"),
                decide("Always", "btn-always", "push-pin", "always"),
                decide("Deny", "btn-deny", "prohibit", "deny"),
            ]),
        m("div")
            .class("approval-verdict")
            .show(&format!("{v}.status && {v}.status !== 'pending'"))
            .bind(
                "class",
                &format!("({v}.status === 'deny' ? 'is-denied' : 'is-allowed')"),
            )
            .kids(vec![
                m("i").bind(
                    "class",
                    &format!(
                        "'ph ' + ({v}.status === 'deny' ? 'ph-prohibit' : 'ph-check-circle')"
                    ),
                ),
                m("span").text(&format!(
                    "({v}.status === 'always' ? 'Allowed — and added to this session\\'s rules' \
                      : {v}.status === 'allow' ? 'Allowed once' : 'Denied')"
                )),
            ]),
    ]);
    vec![fold_card::fold_as(
        st,
        v,
        "fold-approval",
        &format!("({v}.status === 'pending' ? 'is-ask' : 'is-done')"),
        fold_card::head("shield-warning", "Approval", Some(&format!("{v}.name"))),
        body,
    )
    .iff(&format!("{v}.kind === 'approval'"))]
}

/// standalone demo: one waiting, one already answered.
pub fn demo() -> Node {
    let st = samples::EXPR;
    let pending = samples::row("approval");
    let done = samples::row("ap-done");
    m("div").class("approval-demo").child(
        m("div").class("mjs-chat mode-sidebar").kids(vec![
            session::row(st, &pending, parts(st, &pending)),
            session::row(st, &done, parts(st, &done)),
        ]),
    )
}

pub fn register() {
    css::inject("approval-card", CSS);
}

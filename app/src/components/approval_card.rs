use rustex1_kit::h::{lit, m, Node};

use crate::components::{fold_card, samples, session};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .fold-card.fold-approval {
  border-color: color-mix(in srgb, #fb923c 38%, transparent);
  background: var(--panel-2);
  color: var(--muted);
}
:is(#sb, #home) .fold-card.fold-approval .fold-head { color: #fb923c; }

:is(#sb, #home) .approval-actions {
  display: flex; flex-wrap: wrap; gap: 8px;
  margin-top: 10px;
}
:is(#sb, #home) .approval-actions button {
  appearance: none;
  border-radius: 8px;
  padding: 5px 12px;
  font: 600 11.5px/1.5 var(--sans, ui-sans-serif, system-ui, sans-serif);
  cursor: pointer;
  display: inline-flex; align-items: center; gap: 6px;
  transition: background 120ms ease, border-color 120ms ease, color 120ms ease;
}
:is(#sb, #home) .approval-actions button:focus-visible {
  outline: 2px solid #fb923c; outline-offset: 2px;
}
:is(#sb, #home) .btn-allow {
  border: 1px solid color-mix(in srgb, #fb923c 40%, transparent);
  background: color-mix(in srgb, #fb923c 14%, transparent);
  color: #fb923c;
}
:is(#sb, #home) .btn-allow:hover {
  background: color-mix(in srgb, #fb923c 22%, transparent);
  border-color: color-mix(in srgb, #fb923c 60%, transparent);
}
:is(#sb, #home) .btn-always {
  border: 1px solid var(--border);
  background: transparent;
  color: var(--muted);
}
:is(#sb, #home) .btn-always:hover {
  color: var(--text);
  border-color: color-mix(in srgb, #fb923c 55%, transparent);
}
:is(#sb, #home) .btn-deny {
  border: 1px solid var(--border);
  background: transparent;
  color: var(--muted);
}
:is(#sb, #home) .btn-deny:hover {
  color: var(--danger, #d9534f);
  border-color: var(--danger, #d9534f);
}

:is(#sb, #home) .fold-card.fold-approval.is-collapsed .approval-actions,
:is(#sb, #home) .fold-card.fold-approval.is-collapsed .approval-verdict {
  margin-left: 0.25em;
}

:is(#sb, #home) .approval-verdict {
  display: inline-flex; align-items: center; gap: 6px;
  margin-top: 10px;
  font-size: 12px; font-weight: 600;
  color: var(--muted);
}
:is(#sb, #home) .approval-verdict .ph { color: #fb923c; }

.approval-demo { width: min(560px, 100%); }
.approval-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

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
    let body = m("div").class("fold-body").kids(vec![
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

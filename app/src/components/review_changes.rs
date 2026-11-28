use rustex1_kit::h::{lit, m, Node};

use crate::components::vcs;
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .review-changes {
  border: 1px solid var(--border); border-radius: 12px;
  background: var(--panel-2); padding: 11px 12px;
  display: flex; flex-direction: column; gap: 9px;
}
:is(#sb, #home) .review-changes h5 { margin: 0; font-size: 12px; font-weight: 700; color: var(--fg); }
:is(#sb, #home) .review-changes textarea {
  width: 100%; box-sizing: border-box;
  min-height: 54px; resize: vertical;
  background: var(--panel); color: var(--fg);
  border: 1px solid var(--border); border-radius: 8px;
  padding: 7px 9px; font: inherit; font-size: 12px;
}
:is(#sb, #home) .rc-opts { display: flex; flex-direction: column; gap: 4px; }
:is(#sb, #home) .rc-opt {
  display: flex; gap: 8px; align-items: flex-start;
  padding: 6px 8px; border-radius: 8px; cursor: pointer;
  border: 1px solid transparent;
}
:is(#sb, #home) .rc-opt:hover { background: var(--panel); }
:is(#sb, #home) .rc-opt.on { background: var(--accent-tint); border-color: color-mix(in srgb, var(--accent) 40%, transparent); }
:is(#sb, #home) .rc-opt .rad {
  width: 13px; height: 13px; border-radius: 50%; margin-top: 2px;
  border: 1.5px solid var(--gray-dim); flex: 0 0 auto;
}
:is(#sb, #home) .rc-opt.on .rad { border-color: var(--accent); background: radial-gradient(circle, var(--accent) 45%, transparent 50%); }
:is(#sb, #home) .rc-opt-main { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
:is(#sb, #home) .rc-opt-label { display: flex; align-items: center; gap: 5px; font-size: 12px; color: var(--fg); }
:is(#sb, #home) .rc-opt-main small { color: var(--gray-dim); font-size: 10.5px; line-height: 1.45; }
:is(#sb, #home) .rc-actions { display: flex; justify-content: flex-end; }
:is(#sb, #home) .rc-submit {
  border: 0; border-radius: 8px; padding: 6px 13px;
  background: var(--accent); color: #fff; font: inherit; font-size: 12px; cursor: pointer;
}
.review-changes-demo { width: min(420px, 100%); }
"#;

const OPTS: &[(&str, &str, &str, &str)] = &[
    ("comment", "Comment", "chat-circle-text", "Submit general feedback without an explicit decision."),
    ("approve", "Approve", "seal-check", "Submit feedback and approve merging these changes."),
    ("request_changes", "Request changes", "pencil-simple", "Submit feedback that must be addressed before merging."),
];

pub fn parts(st: &str) -> Vec<Node> {
    vec![m("div").class("review-changes").kids(vec![
        m("h5").child(lit("Finish your review")),
        m("textarea")
            .attr("rows", "3")
            .attr("maxlength", "5000")
            .attr("placeholder", "Leave a comment")
            .model(&format!("{st}.reviewComment")),
        m("div")
            .class("rc-opts")
            .attr("role", "radiogroup")
            .attr("aria-label", "review decision")
            .kids(
                OPTS.iter()
                    .map(|(key, label, icon, hint)| {
                        m("div")
                            .class("rc-opt")
                            .attr("role", "radio")
                            .bind("class", &format!("({st}.reviewDecision === '{key}' ? 'rc-opt on' : 'rc-opt')"))
                            .bind("aria-checked", &format!("({st}.reviewDecision === '{key}' ? 'true' : 'false')"))
                            .on("click", &format!("{st}.setReviewDecision('{key}')"))
                            .kids(vec![
                                m("span").class("rad"),
                                m("div").class("rc-opt-main").kids(vec![
                                    m("span").class("rc-opt-label").kids(vec![
                                        Node::icon(icon),
                                        m("span").child(lit(label)),
                                    ]),
                                    m("small").child(lit(hint)),
                                ]),
                            ])
                    })
                    .collect(),
            ),
        m("div").class("rc-actions").child(
            m("button")
                .class("rc-submit")
                .attr("type", "button")
                .on("click", &format!("{st}.submitReview()"))
                .child(lit("Submit review")),
        ),
    ])]
}

pub fn demo() -> Node {
    m("div").class("review-changes-demo").kids(parts(vcs::EXPR))
}

pub fn register() {
    css::inject("review-changes", CSS);
}

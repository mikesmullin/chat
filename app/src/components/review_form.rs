use rustex1_kit::h::{lit, m, Node};

use crate::components::vcs;
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .review-form {
  display: flex; flex-direction: column; gap: 8px;
  border: 1px solid var(--border); border-radius: 12px;
  background: var(--panel-2); padding: 11px 12px;
}
:is(#sb, #home) .review-form h3 {
  margin: 0; font-size: 12px; font-weight: 700; letter-spacing: .01em;
  display: flex; align-items: center; gap: 6px; color: var(--fg);
}
:is(#sb, #home) .review-form textarea {
  width: 100%; box-sizing: border-box;
  min-height: 64px; resize: vertical;
  background: var(--panel); color: var(--fg);
  border: 1px solid var(--border); border-radius: 8px;
  padding: 7px 9px; font: inherit; font-size: 12px;
}
:is(#sb, #home) .review-form textarea:focus-visible {
  outline: 2px solid var(--accent); outline-offset: 1px;
}
:is(#sb, #home) .review-decisions { display: flex; gap: 6px; flex-wrap: wrap; }
:is(#sb, #home) .review-dec {
  display: inline-flex; align-items: center; gap: 5px;
  padding: 4px 10px; border-radius: 999px;
  border: 1px solid var(--border); background: var(--panel);
  color: var(--gray); font-size: 11.5px; cursor: pointer;
}
:is(#sb, #home) .review-dec:hover { color: var(--fg); border-color: var(--accent); }
:is(#sb, #home) .review-dec.on { color: var(--accent-soft); border-color: var(--accent); background: var(--accent-tint); }
:is(#sb, #home) .review-form-actions { display: flex; justify-content: flex-end; }
:is(#sb, #home) .review-submit {
  display: inline-flex; align-items: center; gap: 6px;
  border: 0; border-radius: 8px; padding: 6px 13px;
  background: var(--accent); color: #fff; font: inherit; font-size: 12px;
  cursor: pointer;
}
:is(#sb, #home) .review-submit:disabled { opacity: .5; cursor: default; }
.review-form-demo { width: min(420px, 100%); }
"#;

pub fn parts(st: &str) -> Vec<Node> {
    vec![m("div").class("review-form").kids(vec![
        m("h3").kids(vec![Node::icon("chat-circle-text"), m("span").child(lit("Add review"))]),
        m("div").class("review-decisions").kids(vec![
            decision(st, "comment", "Comment", "chat-circle-text"),
            decision(st, "approve", "Approve", "seal-check"),
            decision(st, "request_changes", "Request changes", "pencil-simple"),
            decision(st, "reject", "Reject", "prohibit"),
        ]),
        m("textarea")
            .attr("rows", "4")
            .attr("maxlength", "5000")
            .attr("placeholder", "optional — say what you saw")
            .model(&format!("{st}.reviewComment")),
        m("div").class("review-form-actions").child(
            m("button")
                .class("review-submit")
                .attr("type", "button")
                .on("click", &format!("{st}.submitReview()"))
                .kids(vec![Node::icon("paper-plane-tilt"), m("span").child(lit("submit review"))]),
        ),
    ])]
}

fn decision(st: &str, key: &str, label: &str, icon: &str) -> Node {
    m("button")
        .class("review-dec")
        .attr("type", "button")
        .bind("class", &format!("({st}.reviewDecision === '{key}' ? 'review-dec on' : 'review-dec')"))
        .on("click", &format!("{st}.setReviewDecision('{key}')"))
        .kids(vec![Node::icon(icon), m("span").child(lit(label))])
}

pub fn demo() -> Node {
    m("div").class("review-form-demo").kids(parts(vcs::EXPR))
}

pub fn register() {
    css::inject("review-form", CSS);
}

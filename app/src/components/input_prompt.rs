use rustex1_kit::h::{lit, m, Node};

use crate::components::jobs;
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .input-prompt {
  display: flex; flex-direction: column; gap: 8px;
  border: 1px solid color-mix(in srgb, var(--pastel-amber, #f5d8a6) 45%, transparent);
  border-radius: 12px; background: color-mix(in srgb, var(--pastel-amber, #f5d8a6) 8%, transparent);
  padding: 11px 12px;
}
:is(#sb, #home) .input-prompt-head {
  display: flex; align-items: center; gap: 6px;
  font-size: 11px; font-weight: 700; color: var(--pastel-amber, #f5d8a6);
  letter-spacing: .04em; text-transform: uppercase;
}
:is(#sb, #home) .input-prompt-q {
  margin: 0; font-size: 12.5px; color: var(--fg); line-height: 1.5;
}
:is(#sb, #home) .input-prompt textarea {
  width: 100%; box-sizing: border-box; min-height: 54px; resize: vertical;
  background: var(--panel); color: var(--fg);
  border: 1px solid var(--border); border-radius: 8px;
  padding: 7px 9px; font: inherit; font-size: 12px;
}
:is(#sb, #home) .input-prompt-actions { display: flex; gap: 7px; justify-content: flex-end; }
:is(#sb, #home) .ip-btn {
  display: inline-flex; align-items: center; gap: 5px;
  border: 1px solid var(--border); border-radius: 8px;
  padding: 5px 11px; background: var(--panel); color: var(--gray);
  font: inherit; font-size: 11.5px; cursor: pointer;
}
:is(#sb, #home) .ip-btn:hover { color: var(--fg); border-color: var(--accent); }
:is(#sb, #home) .ip-btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
.input-prompt-demo { width: min(420px, 100%); }
"#;

pub fn parts(st: &str) -> Vec<Node> {
    vec![m("div")
        .class("input-prompt")
        .show(&format!("{st}.pendingQuestion() && {st}.pendingQuestion().question"))
        .kids(vec![
            m("div").class("input-prompt-head").kids(vec![
                Node::icon("pause-circle"),
                m("span").child(lit("the run is waiting on you")),
            ]),
            m("p").class("input-prompt-q").text(&format!("{st}.pendingQuestion().question")),
            m("textarea")
                .attr("rows", "3")
                .attr("placeholder", "answer the run")
                .model(&format!("{st}.answerDraft")),
            m("div").class("input-prompt-actions").kids(vec![
                m("button")
                    .class("ip-btn primary")
                    .attr("type", "button")
                    .on("click", &format!("{st}.answerRun()"))
                    .kids(vec![Node::icon("paper-plane-tilt"), m("span").child(lit("answer"))]),
            ]),
        ])]
}

pub fn demo() -> Node {
    m("div").class("input-prompt-demo").kids(parts(jobs::EXPR))
}

pub fn register() {
    css::inject("input-prompt", CSS);
}

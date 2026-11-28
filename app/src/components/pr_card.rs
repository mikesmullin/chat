use rustex1_kit::h::{m, Node};

use crate::components::{pr_state_badge, samples};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .pr-card {
  display: flex; flex-direction: column; gap: 5px;
  padding: 9px 11px; border-radius: 10px;
  border: 1px solid var(--border); background: var(--panel-2);
  color: var(--fg); cursor: pointer; text-decoration: none;
  transition: border-color .13s, background-color .13s;
}
:is(#sb, #home) .pr-card:hover { border-color: var(--accent); background: var(--panel); }
:is(#sb, #home) .pr-card-head { display: flex; align-items: center; gap: 7px; min-width: 0; }
:is(#sb, #home) .pr-card-num { color: var(--gray-dim); font-size: 11px; font-variant-numeric: tabular-nums; }
:is(#sb, #home) .pr-card-title {
  flex: 1 1 auto; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  font-size: 12.5px; font-weight: 600;
}
:is(#sb, #home) .pr-card-branches {
  display: flex; align-items: center; gap: 5px;
  font-size: 11px; color: var(--gray-dim); min-width: 0;
}
:is(#sb, #home) .pr-card-branches code {
  background: var(--panel); border: 1px solid var(--border); border-radius: 5px;
  padding: 0 5px; max-width: 45%;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.pr-card-demo { display: flex; flex-direction: column; gap: 8px; width: 340px; }
"#;

pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![m("a")
        .class("pr-card")
        .attr("href", "#")
        .on("click", &format!("$event.preventDefault(); {st}.openPr({v}.repo, '' + {v}.number)"))
        .kids(vec![
            m("div").class("pr-card-head").kids(vec![
                m("span").class("pr-card-num").text(&format!("'#' + {v}.number")),
                m("span").class("pr-card-title").text(&format!("{v}.title")),
                Node::host(pr_state_badge::NAME, &[]).bind("state", &format!("{v}.state")),
            ]),
            m("div").class("pr-card-branches").kids(vec![
                m("code").text(&format!("{v}.sourceBranch")),
                Node::icon("arrow-right").class("ph ph-arrow-right"),
                m("code").text(&format!("{v}.targetBranch")),
            ]),
        ])]
}

pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::sample("pr"));
    m("div").class("pr-card-demo").kids(parts(st, &v))
}

pub fn register() {
    css::inject("pr-card", CSS);
}

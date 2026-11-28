use rustex1_kit::h::{m, Node};

use crate::components::samples;
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .repo-card {
  display: flex; align-items: center; gap: 6px;
  width: 100%; text-align: left;
  border: 0; border-radius: 7px;
  background: transparent; color: var(--gray);
  padding: 4px 8px; cursor: pointer;
  font: inherit; font-size: 12px; line-height: 18px;
  text-decoration: none;
  user-select: none;
}
:is(#sb, #home) .repo-card:hover { background: var(--panel-2); color: var(--fg); }
:is(#sb, #home) .repo-card i { color: var(--gray-dim); font-size: 13px; }
:is(#sb, #home) .repo-name {
  flex: 1 1 auto; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.repo-card-demo { width: min(480px, 100%); }
"#;

pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![m("a")
        .class("repo-card")
        .attr("href", "#")
        .on("click", &format!("$event.preventDefault(); {st}.openRepo({v}.name)"))
        .kids(vec![
            Node::icon("github-logo"),
            m("span").class("repo-name").text(&format!("{v}.name")),
        ])
    ]
}

pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::sample("repo"));
    m("div").class("repo-card-demo").kids(parts(st, &v))
}

pub fn register() {
    css::inject("repo-card", CSS);
}

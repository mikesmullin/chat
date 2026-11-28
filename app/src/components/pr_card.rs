use rustex1_kit::h::{m, Node};

use crate::components::samples;
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .pr-num { flex: 0 0 auto; color: var(--gray-dim); font-variant-numeric: tabular-nums; }
.pr-card-demo { display: flex; flex-direction: column; gap: 1px; width: 300px; }
"#;

pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![
        m("button")
            .class("wt-row")
            .attr("type", "button")
            .bind("title", &format!("{v}.title"))
            .on("click", &format!("{st}.openPr({v}.repo, '' + {v}.number)"))
            .kids(vec![
                Node::icon("git-pull-request"),
                m("span").class("pr-num").text(&format!("'#' + {v}.number")),
                m("span").class("wt-name").text(&format!("{v}.title")),
            ]),
    ]
}

pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::sample("pr"));
    m("div").class("pr-card-demo").kids(parts(st, &v))
}

pub fn register() {
    css::inject("pr-card", CSS);
}

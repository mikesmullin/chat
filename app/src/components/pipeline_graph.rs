use rustex1_kit::h::{m, Node};

use crate::components::{run_state_badge, samples};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .pipe-graph {
  display: flex; align-items: stretch; gap: 0; flex-wrap: wrap;
  row-gap: 8px;
}
:is(#sb, #home) .pipe-node {
  display: flex; flex-direction: column; align-items: center; gap: 4px;
  min-width: 84px; padding: 7px 10px;
  border: 1px solid var(--border); border-radius: 10px; background: var(--panel);
}
:is(#sb, #home) .pipe-node.pn-active { border-color: var(--accent); background: var(--accent-tint); }
:is(#sb, #home) .pipe-node.pn-done { border-color: color-mix(in srgb, var(--pastel-green, #22c55e) 45%, transparent); }
:is(#sb, #home) .pipe-node.pn-bad { border-color: color-mix(in srgb, var(--danger, #d9534f) 45%, transparent); }
:is(#sb, #home) .pipe-node-name { font-size: 11px; font-weight: 600; color: var(--fg); text-align: center; }
:is(#sb, #home) .pipe-edge {
  align-self: center;
  width: 22px; height: 2px; background: var(--border); flex: 0 0 auto;
}
:is(#sb, #home) .pipe-fork {
  display: flex; flex-direction: column; gap: 6px;
  padding: 7px 9px; border: 1px dashed var(--border); border-radius: 10px;
}
:is(#sb, #home) .pipe-fork-label { font-size: 10px; color: var(--gray-dim); letter-spacing: .04em; text-transform: uppercase; }
:is(#sb, #home) .pipe-branch { display: flex; align-items: center; gap: 0; }
.pipe-graph-demo { width: min(560px, 100%); }
"#;

fn node(st: &str, v: &str) -> Node {
    m("div")
        .class("pipe-node")
        .bind("class", &format!(
            "('pipe-node ' + ({v}.state === 'success' ? 'pn-done' : ({v}.state === 'failure' || {v}.state === 'aborted') ? 'pn-bad' : ({v}.state === 'running' || {v}.state === 'provisioning' || {v}.state === 'blocked') ? 'pn-active' : ''))"
        ))
        .kids(vec![
            m("span").class("pipe-node-name").text(&format!("{v}.name")),
            Node::host(run_state_badge::NAME, &[]).bind("state", &format!("{v}.state")),
        ])
        .on("click", &format!("{st}.openRun('' + {st}.runId)"))
}

pub fn parts(st: &str, stages: &str) -> Vec<Node> {
    vec![m("div")
        .class("pipe-graph")
        .for_each(&format!("s in {stages}"))
        .key("s.name")
        .kids(vec![
            node(st, "s"),
            m("span")
                .class("pipe-edge")
                .show(&format!("s.position < {stages}.length - 1"))
                .attr("aria-hidden", "true"),
        ])]
}

pub fn demo() -> Node {
    m("div").class("pipe-graph-demo").kids(parts(
        samples::EXPR,
        &format!("{}.runSample.stages", samples::EXPR),
    ))
}

pub fn register() {
    css::inject("pipeline-graph", CSS);
}

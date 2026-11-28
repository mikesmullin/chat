use rustex1_kit::h::{m, Node};

use crate::components::{job_run_chip, sandbox_chip_state, samples};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .stage-timeline { display: flex; flex-direction: column; }
:is(#sb, #home) .st-row {
  display: flex; align-items: center; gap: 8px;
  padding: 6px 4px; border-top: 1px solid var(--border);
  font-size: 11.5px; color: var(--gray);
}
:is(#sb, #home) .st-row:first-child { border-top: 0; }
:is(#sb, #home) .st-dot {
  width: 9px; height: 9px; border-radius: 50%; flex: 0 0 auto;
  background: var(--gray-dim);
}
:is(#sb, #home) .st-row.st-running .st-dot { background: var(--accent); box-shadow: 0 0 6px var(--accent); }
:is(#sb, #home) .st-row.st-success .st-dot { background: var(--pastel-green, #22c55e); }
:is(#sb, #home) .st-row.st-failure .st-dot, :is(#sb, #home) .st-row.st-aborted .st-dot { background: var(--danger, #d9534f); }
:is(#sb, #home) .st-row.st-blocked .st-dot { background: var(--pastel-amber, #f5d8a6); }
:is(#sb, #home) .st-name {
  flex: 1 1 auto; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .st-group {
  font-size: 10px; color: var(--gray-dim);
  background: var(--panel-2); border: 1px solid var(--border); border-radius: 5px;
  padding: 0 5px;
}
:is(#sb, #home) .st-link {
  border: 0; background: transparent; color: var(--accent-soft);
  font: inherit; font-size: 11px; cursor: pointer; padding: 0;
}
:is(#sb, #home) .st-link:hover { text-decoration: underline; }
:is(#sb, #home) .st-art {
  display: inline-flex; align-items: center; gap: 4px;
  border: 1px solid var(--border); border-radius: 999px;
  padding: 1px 8px; font-size: 10px; color: var(--gray);
  background: var(--panel-2); cursor: pointer;
}
.stage-timeline-demo { width: min(440px, 100%); }
"#;

pub fn parts(st: &str, stages: &str) -> Vec<Node> {
    let _ = st;
    vec![m("div")
        .class("stage-timeline")
        .for_each(&format!("s in {stages}"))
        .key("s.name")
        .kids(vec![
            m("div")
                .class("st-row")
                .bind("class", "('st-row st-' + s.state)")
                .kids(vec![
                    m("span").class("st-dot"),
                    m("span").class("st-name").text("s.name"),
                    m("span").class("st-group").show("s.group").text("s.group"),
                    Node::host(sandbox_chip_state::NAME, &[]).show("s.sandboxId").bind("state", "'ready'").bind("id", "s.sandboxId"),
                    Node::host(job_run_chip::NAME, &[]).bind("state", "s.state"),
                    m("button")
                        .class("st-link")
                        .attr("type", "button")
                        .show("s.sessionId")
                        .attr("title", "open the run session")
                        .on("click", &format!("$store.live.openSession(s.sessionId)"))
                        .text("'session'"),
                    m("button")
                        .class("st-art")
                        .attr("type", "button")
                        .for_each("a in s.artifacts")
                        .key(&format!("s.name + (a.number || '')"))
                        .on("click", "$event.stopPropagation(); $store.vcs.openPr(a.repo, '' + a.number)")
                        .text("'PR #' + a.number"),
                ]),
        ])]
}

pub fn demo() -> Node {
    m("div").class("stage-timeline-demo").kids(parts(
        samples::EXPR,
        &format!("{}.runSample.stages", samples::EXPR),
    ))
}

pub fn register() {
    css::inject("stage-timeline", CSS);
}

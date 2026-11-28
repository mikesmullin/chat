use rustex1_kit::h::{lit, m, Node};

use crate::components::{
    input_prompt, job_run_chip, jobs, journal_tail, pipeline_graph, stage_timeline,
};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .run-panel { display: flex; flex-direction: column; gap: 10px; }
:is(#sb, #home) .runp-head { display: flex; align-items: center; gap: 8px; min-width: 0; }
:is(#sb, #home) .runp-title {
  flex: 1 1 auto; min-width: 0;
  font-size: 13px; font-weight: 700; color: var(--fg);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .runp-btn {
  display: inline-flex; align-items: center; gap: 5px;
  border: 1px solid var(--border); border-radius: 8px;
  padding: 4px 10px; background: var(--panel-2); color: var(--gray);
  font: inherit; font-size: 11px; cursor: pointer;
}
:is(#sb, #home) .runp-btn:hover { color: var(--fg); border-color: var(--accent); }
:is(#sb, #home) .runp-btn.danger:hover { color: var(--danger, #d9534f); border-color: var(--danger, #d9534f); }
:is(#sb, #home) .runp-meta { display: flex; gap: 8px; flex-wrap: wrap; font-size: 11px; color: var(--gray-dim); }
:is(#sb, #home) .runp-error {
  margin: 0; padding: 8px 10px; border-radius: 8px;
  border: 1px solid color-mix(in srgb, var(--danger, #d9534f) 35%, transparent);
  background: color-mix(in srgb, var(--danger, #d9534f) 8%, transparent);
  color: var(--danger, #d9534f); font-size: 11.5px;
}
.run-panel-demo { width: min(560px, 100%); }
"#;

pub fn panel() -> Node {
    let st = jobs::EXPR;
    m("div")
        .class("agent-editor run-panel")
        .show(&format!("{st}.runOpen"))
        .kids(vec![
            m("div").class("agent-editor-head").kids(vec![
                Node::icon("path"),
                m("span").class("agent-editor-title").text(&format!("{st}.runLabel({st}.run)")),
                Node::host(job_run_chip::NAME, &[]).bind("state", &format!("{st}.run.state")),
                m("button")
                    .class("runp-btn")
                    .attr("type", "button")
                    .attr("title", "refresh")
                    .on("click", &format!("{st}.pollRun()"))
                    .child(Node::icon("arrows-clockwise")),
                m("button")
                    .class("runp-btn danger")
                    .attr("type", "button")
                    .show(&format!("{st}.runActive({st}.run)"))
                    .on("click", &format!("{st}.stopRun('' + {st}.runId)"))
                    .kids(vec![Node::icon("stop"), m("span").child(lit("stop"))]),
                m("button")
                    .class("runp-btn")
                    .attr("type", "button")
                    .attr("title", "close")
                    .on("click", &format!("{st}.closeRun()"))
                    .child(Node::icon("x")),
            ]),
            m("div").class("runp-meta").kids(vec![
                m("span").text(&format!("'job ' + {st}.run.jobName")),
                m("span").text(&format!("'run #' + {st}.run.number")),
                m("span").text(&format!("{st}.run.triggerSource")),
            ]),
            m("p").class("runp-error").show(&format!("{st}.run.error")).text(&format!("{st}.run.error")),
            panel_body(st),
        ])
}

fn panel_body(st: &str) -> Node {
    let stages = format!("{st}.stagesOfRun()");
    let mut kids = input_prompt::parts(st);
    kids.extend(pipeline_graph::parts(st, &stages));
    kids.extend(stage_timeline::parts(st, &stages));
    kids.extend(journal_tail::parts(st));
    m("div").class("runp-body").kids(kids)
}

pub fn demo() -> Node {
    panel()
}

pub fn register() {
    css::inject("run-panel", CSS);
}

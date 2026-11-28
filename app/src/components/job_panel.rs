use rustex1_kit::h::{lit, m, Node};

use crate::components::{job_card, jobs, run_state_badge};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .job-panel { display: flex; flex-direction: column; gap: 10px; }
:is(#sb, #home) .jobp-head { display: flex; align-items: center; gap: 8px; min-width: 0; }
:is(#sb, #home) .jobp-title {
  flex: 1 1 auto; min-width: 0;
  font-size: 13px; font-weight: 700; color: var(--fg);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .jobp-btn {
  display: inline-flex; align-items: center; gap: 5px;
  border: 1px solid var(--border); border-radius: 8px;
  padding: 4px 10px; background: var(--panel-2); color: var(--gray);
  font: inherit; font-size: 11px; cursor: pointer;
}
:is(#sb, #home) .jobp-btn:hover { color: var(--fg); border-color: var(--accent); }
:is(#sb, #home) .jobp-runs { display: flex; flex-direction: column; }
:is(#sb, #home) .jobp-run {
  display: flex; align-items: center; gap: 7px;
  padding: 5px 4px; border-top: 1px solid var(--border);
  background: transparent; border-left: 0; border-right: 0; border-bottom: 0;
  color: var(--gray); font: inherit; font-size: 11.5px;
  cursor: pointer; text-align: left; width: 100%;
}
:is(#sb, #home) .jobp-run:hover { color: var(--fg); }
:is(#sb, #home) .jobp-run-name {
  flex: 1 1 auto; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .jobp-form {
  display: flex; flex-direction: column; gap: 8px;
  border: 1px solid var(--border); border-radius: 12px;
  background: var(--panel-2); padding: 11px 12px;
}
:is(#sb, #home) .jobp-form h3 { margin: 0; font-size: 12px; font-weight: 700; color: var(--fg); }
:is(#sb, #home) .jobp-form input {
  width: 100%; box-sizing: border-box;
  background: var(--panel); color: var(--fg);
  border: 1px solid var(--border); border-radius: 8px;
  padding: 7px 9px; font: inherit; font-size: 12px;
}
:is(#sb, #home) .jobp-form input:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
:is(#sb, #home) .jobp-error {
  margin: 0; font-size: 11px; color: var(--danger, #d9534f);
}
:is(#sb, #home) .jobp-actions { display: flex; justify-content: flex-end; }
:is(#sb, #home) .jobp-submit {
  display: inline-flex; align-items: center; gap: 6px;
  border: 0; border-radius: 8px; padding: 6px 13px;
  background: var(--accent); color: #fff; font: inherit; font-size: 12px; cursor: pointer;
}
.job-panel-demo { width: min(480px, 100%); }
"#;

pub fn panel() -> Node {
    let st = jobs::EXPR;
    m("div")
        .class("agent-editor job-panel")
        .show(&format!("{st}.jobOpen || {st}.jobFormOpen"))
        .kids(vec![
            m("div").class("jobp-head").kids(vec![
                Node::icon("wrench"),
                m("span").class("jobp-title").text(&format!("{st}.jobName || 'Track a Jobfile'")),
                m("button")
                    .class("jobp-btn")
                    .attr("type", "button")
                    .attr("title", "close")
                    .on("click", &format!("{st}.jobOpen = false; {st}.jobFormOpen = false"))
                    .child(Node::icon("x")),
            ]),
            m("div").show(&format!("{st}.jobOpen")).kids(job_card::parts(st)),
            m("div").class("jobp-runs").show(&format!("{st}.jobOpen")).kids(vec![
                m("button")
                    .class("jobp-run")
                    .for_each(&format!("run in {st}.runsForJob({st}.jobName)"))
                    .key("run.id")
                    .on("click", &format!("{st}.openRun('' + run.id)"))
                    .kids(vec![
                        Node::host(run_state_badge::NAME, &[]).bind("state", "run.state"),
                        m("span").class("jobp-run-name").text(&format!("{st}.runLabel(run)")),
                        Node::host(crate::components::reltime::NAME, &[]).bind("at", "run.createdAt"),
                    ]),
            ]),
            m("div").class("jobp-form").show(&format!("{st}.jobFormOpen")).kids(vec![
                m("h3").child(lit("Track a Jobfile")),
                m("input")
                    .attr("type", "text")
                    .attr("spellcheck", "false")
                    .attr("placeholder", "/path/to/Jobfile.yaml")
                    .model(&format!("{st}.formPath")),
                m("p").class("jobp-error").show(&format!("{st}.formError")).text(&format!("{st}.formError")),
                m("div").class("jobp-actions").child(
                    m("button")
                        .class("jobp-submit")
                        .attr("type", "button")
                        .on("click", &format!("{st}.registerJob()"))
                        .kids(vec![Node::icon("plus"), m("span").child(lit("track it"))]),
                ),
            ]),
        ])
}

pub fn demo() -> Node {
    panel()
}

pub fn register() {
    css::inject("job-panel", CSS);
}

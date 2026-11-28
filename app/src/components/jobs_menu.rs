use rustex1_kit::h::{lit, m, Node};

use crate::components::{job_row, jobs, rail_head};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .jobs-list { display: flex; flex-direction: column; gap: 1px; }
:is(#sb, #home) .jobs-group {
  padding: 6px 8px 2px; font-size: 10px; letter-spacing: .05em;
  text-transform: uppercase; color: var(--gray-dim);
}
:is(#sb, #home) .jobs-prompt-row {
  display: flex; align-items: center; gap: 6px;
  width: 100%; text-align: left;
  border: 0; border-radius: 7px;
  background: transparent; color: var(--gray);
  padding: 4px 8px; cursor: pointer; font: inherit; font-size: 12px; line-height: 18px;
}
:is(#sb, #home) .jobs-prompt-row:hover { background: var(--panel-2); color: var(--fg); }
:is(#sb, #home) .jobs-prompt-row.is-off { opacity: .45; }
:is(#sb, #home) .jobs-prompt-row i { color: var(--gray-dim); font-size: 13px; }
:is(#sb, #home) .jobs-prompt-name {
  flex: 1 1 auto; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .jobs-prompt-cron {
  flex: 0 0 auto; font-size: 10.5px; color: var(--gray-dim);
  font-variant-numeric: tabular-nums;
}
:is(#sb, #home) .jobs-empty { margin: 0; padding: 10px 6px; color: var(--gray-dim); font-size: 11px; }
.jobs-menu-demo { width: 300px; }
"#;

pub fn menu(st: &str) -> Node {
    let js = jobs::EXPR;
    m("div")
        .class("jobs-menu")
        .kids(vec![
            rail_head::head(
                "Jobs",
                "wrench",
                Some((
                    "Track Jobfile",
                    &format!("{js}.openJobForm()"),
                    "$store.ident.can('job.manage')",
                )),
                Some("jobs"),
            ),
            m("div")
                .class("jobs-list")
                .show("$store.home.jobsOpen")
                .kids(vec![
                    m("div").for_each(&format!("b in {js}.basedirs()")).key(&format!("'job' + b")).kids(vec![
                        m("div")
                            .class("jobs-group")
                            .show(&format!("{js}.jobsForBasedir(b).length > 0"))
                            .text("b"),
                        m("div")
                            .class("jobs-group-rows")
                            .kids(vec![m("div")
                                .for_each(&format!("j in {js}.jobsForBasedir(b)"))
                                .key("j.id")
                                .kids(job_row::parts(js, "j"))]),
                    ]),
                    m("div").class("jobs-group").show(&format!("{st}.automations.length > 0")).child(lit("prompt automations")),
                    m("button")
                        .class("jobs-prompt-row")
                        .for_each(&format!("a in {st}.automations"))
                        .key("a.id")
                        .bind("class", "('jobs-prompt-row ' + (a.enabled === false ? 'is-off' : ''))")
                        .on("click", &format!("{st}.editTarget('automation', a.id)"))
                        .bind("title", "(a.prompt || a.cron || a.name)")
                        .kids(vec![
                            Node::icon("timer"),
                            m("span").class("jobs-prompt-name").text("a.name"),
                            m("span").class("jobs-prompt-cron").text("a.cron"),
                        ]),
                    m("p")
                        .class("jobs-empty")
                        .show(&format!("{js}.jobs.length === 0 && {st}.automations.length === 0"))
                        .child(lit("No jobs yet.")),
                ]),
        ])
}

pub fn demo() -> Node {
    menu(crate::components::samples::EXPR)
}

pub fn register() {
    css::inject("jobs-menu", CSS);
}

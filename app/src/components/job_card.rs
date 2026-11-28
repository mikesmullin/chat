use rustex1_kit::h::{lit, m, Node};

use crate::components::jobs;
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .job-card {
  display: flex; flex-direction: column; gap: 8px;
  border: 1px solid var(--border); border-radius: 12px;
  background: var(--panel-2); padding: 11px 12px;
}
:is(#sb, #home) .job-card-head { display: flex; align-items: center; gap: 8px; }
:is(#sb, #home) .job-card-head h3 {
  margin: 0; flex: 1 1 auto; min-width: 0;
  font-size: 13px; font-weight: 700; color: var(--fg);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .job-chip {
  display: inline-flex; align-items: center; gap: 4px;
  padding: 1px 8px; border-radius: 999px; font-size: 10.5px; font-weight: 600;
  border: 1px solid var(--border); background: var(--panel); color: var(--gray);
  white-space: nowrap;
}
:is(#sb, #home) .job-desc { margin: 0; font-size: 11.5px; color: var(--gray-dim); }
:is(#sb, #home) .job-meta { display: flex; flex-wrap: wrap; gap: 8px; font-size: 10.5px; color: var(--gray-dim); }
:is(#sb, #home) .job-meta code {
  background: var(--panel); border: 1px solid var(--border); border-radius: 5px; padding: 0 5px;
}
:is(#sb, #home) .job-stages { margin: 0; font-size: 11px; color: var(--gray); display: flex; align-items: center; gap: 5px; flex-wrap: wrap; }
:is(#sb, #home) .job-stages code {
  background: var(--panel); border: 1px solid var(--border); border-radius: 5px; padding: 0 5px;
}
:is(#sb, #home) .job-card-actions { display: flex; gap: 7px; flex-wrap: wrap; }
:is(#sb, #home) .job-btn {
  display: inline-flex; align-items: center; gap: 5px;
  border: 1px solid var(--border); border-radius: 8px;
  padding: 5px 11px; background: var(--panel); color: var(--gray);
  font: inherit; font-size: 11.5px; cursor: pointer;
}
:is(#sb, #home) .job-btn:hover { color: var(--fg); border-color: var(--accent); }
:is(#sb, #home) .job-btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
:is(#sb, #home) .job-btn.danger:hover { color: var(--danger, #d9534f); border-color: var(--danger, #d9534f); }
.job-card-demo { width: min(420px, 100%); }
"#;

pub fn parts(st: &str) -> Vec<Node> {
    let job = format!("{st}.jobOf({st}.jobName)");
    vec![m("div").class("job-card").kids(vec![
        m("div").class("job-card-head").kids(vec![
            m("h3").text(&format!("{job}.display || {st}.jobName")),
            m("span")
                .class("job-chip")
                .bind("class", &format!("({job}.cron ? 'job-chip' : 'job-chip')"))
                .kids(vec![
                    Node::icon_dyn(&format!("({job}.cron ? 'ph-clock' : 'ph-hand-tap')")),
                    m("span").text(&format!("{job}.cron || 'manual'")),
                ]),
        ]),
        m("p").class("job-desc").text(&format!("{job}.description || '—'")),
        m("div").class("job-meta").kids(vec![
            m("code").text(&format!("{job}.sourcePath || 'inline'")),
            m("span").text(&format!("({job}.sourceSha || '').slice(0, 8)")),
        ]),
        m("div")
            .class("job-stages")
            .show(&format!("{job}.definition.stages"))
            .kids(vec![
                Node::icon("path"),
                m("code")
                    .for_each(&format!("s in {job}.definition.stages"))
                    .key("s.name")
                    .text("s.name"),
            ]),
        m("div").class("job-card-actions").kids(vec![
            m("button")
                .class("job-btn primary")
                .attr("type", "button")
                .on("click", &format!("{st}.triggerJob({st}.jobName)"))
                .kids(vec![Node::icon("play"), m("span").child(lit("run now"))]),
            m("button")
                .class("job-btn")
                .attr("type", "button")
                .on("click", &format!("{st}.toggleJob({st}.jobName, {job}.enabled ? '1' : '0')"))
                .kids(vec![
                    Node::icon_dyn(&format!("({job}.enabled ? 'pause' : 'play')")),
                    m("span").text(&format!("({job}.enabled ? 'disable schedule' : 'enable schedule')")),
                ]),
            m("button")
                .class("job-btn danger")
                .attr("type", "button")
                .on("click", &format!("{st}.deleteJob({st}.jobName)"))
                .kids(vec![Node::icon("trash"), m("span").child(lit("unregister"))]),
        ]),
    ])]
}

pub fn demo() -> Node {
    m("div").class("job-card-demo").kids(parts(jobs::EXPR))
}

pub fn register() {
    css::inject("job-card", CSS);
}

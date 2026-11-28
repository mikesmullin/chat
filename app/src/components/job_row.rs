use rustex1_kit::h::{m, Node};

use crate::components::samples;
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .job-row {
  display: flex; align-items: center; gap: 7px;
  padding: 5px 8px; border-radius: 8px;
  border: 1px solid transparent;
  color: var(--gray); font-size: 12px; cursor: pointer;
}
:is(#sb, #home) .job-row:hover { background: var(--panel-2); color: var(--fg); }
:is(#sb, #home) .job-row.is-active { background: var(--accent-tint); border-color: color-mix(in srgb, var(--accent) 40%, transparent); color: var(--fg); }
:is(#sb, #home) .job-row-name {
  flex: 1 1 auto; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .job-row-cron {
  flex: 0 0 auto; font-size: 10.5px; color: var(--gray-dim);
  display: inline-flex; align-items: center; gap: 3px;
}
:is(#sb, #home) .job-row-off { opacity: .5; }
.job-row-demo { display: flex; flex-direction: column; gap: 4px; width: 320px; }
"#;

pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![m("div")
        .class("job-row")
        .bind("class", &format!("({st}.jobName === {v}.name ? 'job-row is-active' : 'job-row') + ({v}.enabled ? '' : ' job-row-off')"))
        .on("click", &format!("{st}.openJob({v}.name)"))
        .kids(vec![
            Node::icon_dyn(&format!("({v}.cron ? 'ph-clock' : 'ph-hand-tap')")),
            m("span").class("job-row-name").text(&format!("{v}.display || {v}.name")),
            m("span")
                .class("job-row-cron")
                .show(&format!("{v}.cron"))
                .text(&format!("{v}.cron")),
        ])]
}

pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::sample("job"));
    m("div").class("job-row-demo").kids(parts(st, &v))
}

pub fn register() {
    css::inject("job-row", CSS);
}

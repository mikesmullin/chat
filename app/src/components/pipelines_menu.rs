use rustex1_kit::h::{lit, m, Node};

use crate::components::{jobs, rail_head, reltime, run_state_badge};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .pipe-list { display: flex; flex-direction: column; gap: 1px; }
:is(#sb, #home) .pipe-group {
  padding: 6px 8px 2px; font-size: 10px; letter-spacing: .05em;
  text-transform: uppercase; color: var(--gray-dim);
}
:is(#sb, #home) .pipe-row {
  display: flex; align-items: center; gap: 6px;
  width: 100%; text-align: left;
  border: 0; border-radius: 7px;
  background: transparent; color: var(--gray);
  padding: 4px 8px; cursor: pointer; font: inherit; font-size: 11.5px; line-height: 18px;
}
:is(#sb, #home) .pipe-row:hover { background: var(--panel-2); color: var(--fg); }
:is(#sb, #home) .pipe-row.is-active { background: var(--accent-tint); color: var(--fg); }
:is(#sb, #home) .pipe-name {
  flex: 1 1 auto; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .pipe-empty { margin: 0; padding: 10px 6px; color: var(--gray-dim); font-size: 11px; }
.pipelines-menu-demo { width: 300px; }
"#;

pub fn menu() -> Node {
    let st = jobs::EXPR;
    m("div")
        .class("pipelines-menu")
        .kids(vec![
            rail_head::head("Pipelines", "path", None, Some("pipelines")),
            m("div")
                .class("pipe-list")
                .show("$store.home.pipelinesOpen")
                .kids(vec![
                    m("div").for_each(&format!("b in {st}.basedirs()")).key(&format!("'pipe' + b")).kids(vec![
                        m("div")
                            .class("pipe-group")
                            .show(&format!("{st}.runsForBasedir(b).length > 0"))
                            .text("b"),
                        m("button")
                            .class("pipe-row")
                            .for_each(&format!("run in {st}.runsForBasedir(b)"))
                            .key("run.id")
                            .bind("class", &format!("({st}.runId === '' + run.id ? 'pipe-row is-active' : 'pipe-row')"))
                            .on("click", &format!("{st}.openRun('' + run.id)"))
                            .bind("title", &format!("{st}.runLabel(run)"))
                            .kids(vec![
                                Node::host(run_state_badge::NAME, &[]).bind("state", "run.state"),
                                m("span").class("pipe-name").text(&format!("{st}.runLabel(run)")),
                                Node::host(reltime::NAME, &[]).bind("at", "run.createdAt"),
                            ]),
                    ]),
                    m("p")
                        .class("pipe-empty")
                        .show(&format!("{st}.runs.length === 0"))
                        .child(lit("No pipeline runs yet.")),
                ]),
        ])
}

pub fn demo() -> Node {
    menu()
}

pub fn register() {
    css::inject("pipelines-menu", CSS);
}

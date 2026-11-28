use rustex1_kit::h::{m, Node};

use crate::components::samples;
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .journal-tail {
  display: flex; flex-direction: column; gap: 2px;
  font-family: var(--mjs-mono, ui-monospace, monospace);
  max-height: 320px; overflow-y: auto;
}
:is(#sb, #home) .jr-row {
  display: flex; gap: 7px; align-items: baseline;
  padding: 3px 6px; border-radius: 6px; font-size: 10.5px; line-height: 1.55;
}
:is(#sb, #home) .jr-row:hover { background: var(--panel-2); }
:is(#sb, #home) .jr-ico { flex: 0 0 auto; color: var(--gray-dim); font-size: 11px; }
:is(#sb, #home) .jr-row.jr-error .jr-ico, :is(#sb, #home) .jr-row.jr-error .jr-msg { color: var(--danger, #d9534f); }
:is(#sb, #home) .jr-row.jr-warn .jr-ico { color: var(--mjs-yellow); }
:is(#sb, #home) .jr-row.jr-artifact .jr-ico { color: var(--pastel-purple, #a78bfa); }
:is(#sb, #home) .jr-row.jr-qa .jr-ico { color: var(--pastel-amber, #f5d8a6); }
:is(#sb, #home) .jr-row.jr-tool .jr-ico, :is(#sb, #home) .jr-row.jr-llm .jr-ico { color: var(--accent-soft); }
:is(#sb, #home) .jr-stage {
  flex: 0 0 auto; color: var(--gray-dim);
  background: var(--panel-2); border-radius: 4px; padding: 0 5px;
}
:is(#sb, #home) .jr-msg {
  flex: 1 1 auto; min-width: 0;
  color: var(--gray); white-space: pre-wrap; word-break: break-word;
}
:is(#sb, #home) .jr-time { flex: 0 0 auto; color: var(--gray-dim); }
.journal-tail-demo { width: min(560px, 100%); }
"#;

const KIND_ICON: &str =
    "({stage_start:'ph-play',stage_end:'ph-stop',step_start:'ph-circle',step_end:'ph-check',qa:'ph-question',artifact:'ph-git-pull-request',note:'ph-note',error:'ph-x-circle',tool:'ph-wrench',llm:'ph-brain'}[e.kind] || 'ph-note')";

pub fn parts(st: &str) -> Vec<Node> {
    vec![m("div")
        .class("journal-tail")
        .for_each(&format!("e in {st}.journalOfRun()"))
        .key("e.id")
        .kids(vec![
            m("div")
                .class("jr-row")
                .bind("class", "('jr-row jr-' + e.kind + (e.level === 'error' ? ' jr-error' : e.level === 'warn' ? ' jr-warn' : ''))")
                .bind("title", "e.createdAt")
                .kids(vec![
                    m("i").class("jr-ico").bind("class", &format!("('ph jr-ico ' + {KIND_ICON})")),
                    m("span").class("jr-stage").show("e.stage").text("e.stage"),
                    m("span").class("jr-msg").text("e.message"),
                ]),
        ])]
}

pub fn demo() -> Node {
    m("div").class("journal-tail-demo").kids(parts(&format!("{}.journal", samples::EXPR)))
}

pub fn register() {
    css::inject("journal-tail", CSS);
}

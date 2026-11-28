use rustex1_kit::h::{lit, m, Node};

use crate::components::samples;
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .mdp-note {
  display: inline-flex; align-items: center; gap: 4px;
  color: var(--gray-dim); font-size: 10.5px; font-style: italic;
}
:is(#sb, #home) .mdp-prose { padding: 8px 10px; display: flex; flex-direction: column; gap: 6px; }
:is(#sb, #home) .mdp-run {
  border-radius: 8px; padding: 5px 9px; font-size: 12px;
  border: 1px solid transparent;
}
:is(#sb, #home) .mdp-run.mdp-add { background: color-mix(in srgb, var(--pastel-green, #22c55e) 12%, transparent); border-color: color-mix(in srgb, var(--pastel-green, #22c55e) 25%, transparent); }
:is(#sb, #home) .mdp-run.mdp-del { background: color-mix(in srgb, var(--danger, #d9534f) 11%, transparent); border-color: color-mix(in srgb, var(--danger, #d9534f) 24%, transparent); }
.mdp-demo { width: min(680px, 100%); max-height: 420px; overflow-y: auto; }
"#;

pub fn parts(files_expr: &str) -> Vec<Node> {
    vec![
        m("p")
            .class("diff-empty")
            .show(&format!("{files_expr}.length === 0"))
            .child(lit("Empty diff — nothing to preview.")),
        m("div").class("md-diff-preview").kids(vec![m("section")
            .class("diff-file")
            .for_each(&format!("f in {files_expr}"))
            .key("f.path")
            .kids(vec![
                m("header").class("diff-file-head").kids(vec![
                    m("i").bind("class", "(f.isMd ? 'ph ph-file-magnifying-glass' : 'ph ph-file-code')"),
                    m("code").class("diff-path").text("f.path"),
                    m("span").class("diff-stats").kids(vec![
                        m("span").class("d-add-count").text("'+' + f.added"),
                        m("span").class("d-del-count").text("'−' + f.removed"),
                    ]),
                    m("span")
                        .class("mdp-note")
                        .show("!f.isMd")
                        .kids(vec![Node::icon("info"), m("span").child(lit("no markdown preview — code shown"))]),
                ]),
                m("div")
                    .class("mdp-prose")
                    .show("f.isMd")
                    .kids(vec![m("div")
                        .class("mdp-run")
                        .for_each("run in f.runs")
                        .key("run.id")
                        .bind("class", "('mdp-run mdp-' + run.cls)")
                        .inner_html("run.html")]),
                m("div")
                    .class("diff-body")
                    .show("!f.isMd")
                    .kids(vec![m("div")
                        .class("d-line")
                        .for_each("line in f.lines")
                        .key("line.id")
                        .bind("class", "('d-line ' + line.cls)")
                        .kids(vec![
                            m("span").class("d-sign").text("line.sign"),
                            m("code").class("d-text").text("line.text"),
                        ])]),
            ])]),
    ]
}

pub fn demo() -> Node {
    m("div")
        .class("mdp-demo")
        .kids(parts(&format!("{}.diffFiles", samples::EXPR)))
}

pub fn register() {
    css::inject("md-diff-preview", CSS);
}

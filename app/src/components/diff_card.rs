use rustex1_kit::h::{lit, m, Node};

use crate::components::{diff_view, md_diff_preview, vcs};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .diff-card {
  border: 1px solid var(--border); border-radius: 12px;
  background: var(--panel-2); overflow: hidden;
}
:is(#sb, #home) .diff-card > summary {
  display: flex; align-items: center; gap: 7px;
  padding: 8px 11px; cursor: pointer; user-select: none;
  font-size: 12px; color: var(--fg); list-style: none;
}
:is(#sb, #home) .diff-card > summary::-webkit-details-marker { display: none; }
:is(#sb, #home) .diff-card .dc-caret { transition: transform .13s; color: var(--gray-dim); }
:is(#sb, #home) .diff-card[open] .dc-caret { transform: rotate(90deg); }
:is(#sb, #home) .diff-card .dc-dim { color: var(--gray-dim); font-size: 11px; }
:is(#sb, #home) .diff-card .dc-add { color: var(--pastel-green, #22c55e); }
:is(#sb, #home) .diff-card .dc-del { color: var(--danger, #d9534f); }
:is(#sb, #home) .diff-card .dc-tog { margin-left: auto; display: inline-flex; gap: 4px; }
:is(#sb, #home) .diff-card .dc-btn {
  display: inline-flex; align-items: center; justify-content: center;
  width: 26px; height: 24px; border-radius: 6px;
  border: 1px solid transparent; background: transparent;
  color: var(--gray-dim); cursor: pointer;
}
:is(#sb, #home) .diff-card .dc-btn:hover { color: var(--fg); background: var(--panel); }
:is(#sb, #home) .diff-card .dc-btn.on { color: var(--accent-soft); background: var(--accent-tint); border-color: color-mix(in srgb, var(--accent) 40%, transparent); }
:is(#sb, #home) .diff-card .dc-body { padding: 10px; border-top: 1px solid var(--border); }
.diff-card-demo { width: min(680px, 100%); max-height: 460px; overflow-y: auto; }
"#;

pub fn parts(st: &str) -> Vec<Node> {
    let files = format!("{st}.diffFiles");
    vec![m("details")
        .class("diff-card")
        .attr("open", "")
        .kids(vec![
            m("summary").kids(vec![
                Node::icon("caret-right").class("ph ph-caret-right dc-caret"),
                Node::icon("git-diff"),
                m("b").child(lit("Files changed")),
                m("span").class("dc-dim").show(&format!("!{st}.diffLoading")).kids(vec![
                    m("span").text(&format!("{files}.length")),
                    m("span").child(lit(" files · ")),
                    m("span").class("dc-add").text(&format!("'+' + {files}.reduce((n, f) => n + f.added, 0)")),
                    m("span").class("dc-del").text(&format!("'−' + {files}.reduce((n, f) => n + f.removed, 0)")),
                ]),
                m("span").class("dc-dim").show(&format!("{st}.diffLoading")).child(lit("computing…")),
                m("span").class("dc-tog").attr("role", "group").attr("aria-label", "diff view mode").kids(vec![
                    m("button")
                        .attr("type", "button")
                        .attr("title", "Code — raw unified diff")
                        .attr("aria-label", "Code view")
                        .bind("class", &format!("({st}.diffMode === 'code' ? 'dc-btn on' : 'dc-btn')"))
                        .on("click", &format!("$event.preventDefault(); $event.stopPropagation(); {st}.setDiffMode('code')"))
                        .child(Node::icon("file-code")),
                    m("button")
                        .attr("type", "button")
                        .attr("title", "Preview — rendered markdown")
                        .attr("aria-label", "Markdown preview")
                        .bind("class", &format!("({st}.diffMode === 'preview' ? 'dc-btn on' : 'dc-btn')"))
                        .on("click", &format!("$event.preventDefault(); $event.stopPropagation(); {st}.setDiffMode('preview')"))
                        .child(Node::icon("file-magnifying-glass")),
                ]),
            ]),
            m("div").class("dc-body").kids(vec![
                m("div").show(&format!("{st}.diffMode === 'code'")).kids(diff_view::parts(&files)),
                m("div").show(&format!("{st}.diffMode === 'preview'")).kids(md_diff_preview::parts(&files)),
            ]),
        ])]
}

pub fn demo() -> Node {
    m("div").class("diff-card-demo").kids(parts(vcs::EXPR))
}

pub fn register() {
    css::inject("diff-card", CSS);
}

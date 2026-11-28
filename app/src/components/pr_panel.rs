use rustex1_kit::h::{lit, m, Node};

use crate::components::{
    diff_card, merge_panel, pr_state_pill, pr_thread, review_changes, review_form, vcs,
};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .pr-panel { display: flex; flex-direction: column; gap: 10px; }
:is(#sb, #home) .prp-head { display: flex; align-items: center; gap: 8px; min-width: 0; }
:is(#sb, #home) .prp-title {
  flex: 1 1 auto; min-width: 0;
  font-size: 13px; font-weight: 700; color: var(--fg);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .prp-sub {
  display: flex; align-items: center; gap: 6px;
  font-size: 11px; color: var(--gray-dim); flex-wrap: wrap;
}
:is(#sb, #home) .prp-sub code {
  background: var(--panel-2); border: 1px solid var(--border); border-radius: 5px;
  padding: 0 5px;
}
:is(#sb, #home) .prp-attach {
  display: flex; gap: 6px; flex-wrap: wrap;
}
:is(#sb, #home) .prp-attach a {
  display: inline-flex; align-items: center; gap: 5px;
  border: 1px solid var(--border); border-radius: 999px;
  padding: 2px 10px; font-size: 10.5px; color: var(--gray);
  background: var(--panel-2); text-decoration: none;
}
:is(#sb, #home) .prp-attach a:hover { color: var(--fg); border-color: var(--accent); }
:is(#sb, #home) .prp-tools { display: flex; gap: 6px; flex-wrap: wrap; }
:is(#sb, #home) .prp-btn {
  display: inline-flex; align-items: center; gap: 5px;
  border: 1px solid var(--border); border-radius: 8px;
  padding: 4px 10px; background: var(--panel-2); color: var(--gray);
  font: inherit; font-size: 11px; cursor: pointer;
}
:is(#sb, #home) .prp-btn:hover { color: var(--fg); border-color: var(--accent); }
.prp-form {
  display: flex; flex-direction: column; gap: 8px;
  border: 1px solid var(--border); border-radius: 12px;
  background: var(--panel-2); padding: 11px 12px;
}
:is(#sb, #home) .prp-form h3 { margin: 0; font-size: 12px; font-weight: 700; color: var(--fg); }
:is(#sb, #home) .prp-form select,
:is(#sb, #home) .prp-form input,
:is(#sb, #home) .prp-form textarea {
  width: 100%; box-sizing: border-box;
  background: var(--panel); color: var(--fg);
  border: 1px solid var(--border); border-radius: 8px;
  padding: 7px 9px; font: inherit; font-size: 12px;
}
:is(#sb, #home) .prp-form textarea { resize: vertical; }
:is(#sb, #home) .prp-form select:focus-visible,
:is(#sb, #home) .prp-form input:focus-visible,
:is(#sb, #home) .prp-form textarea:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
:is(#sb, #home) .prp-error { margin: 0; font-size: 11px; color: var(--danger, #d9534f); }
:is(#sb, #home) .prp-actions { display: flex; justify-content: flex-end; }
:is(#sb, #home) .prp-submit {
  display: inline-flex; align-items: center; gap: 6px;
  border: 0; border-radius: 8px; padding: 6px 13px;
  background: var(--accent); color: #fff; font: inherit; font-size: 12px; cursor: pointer;
}
.pr-panel-demo { width: min(560px, 100%); }
"#;

fn pr_form(st: &str) -> Node {
    m("div").class("prp-form").kids(vec![
        m("h3").child(lit("Open a pull request")),
        m("select")
            .on("change", &format!("{st}.pickFormRepo($event.target.value)"))
            .kids(vec![
                m("option").attr("value", "").child(lit("choose a repo…")),
                m("option")
                    .for_each(&format!("n in {st}.repoNames()"))
                    .key("n")
                    .bind("value", "n")
                    .bind("selected", &format!("({st}.formRepo === n ? 'selected' : null)"))
                    .text("n"),
            ]),
        m("select")
            .on("change", &format!("{st}.pickFormBranch('source', $event.target.value)"))
            .kids(vec![
                m("option").attr("value", "").child(lit("source branch (the work)…")),
                m("option")
                    .for_each(&format!("b in {st}.formSourceBranches"))
                    .key("b.name")
                    .bind("value", "b.name")
                    .bind("selected", &format!("({st}.formSource === b.name ? 'selected' : null)"))
                    .text("b.name"),
            ]),
        m("select")
            .on("change", &format!("{st}.pickFormBranch('target', $event.target.value)"))
            .kids(vec![
                m("option").attr("value", "").child(lit("target branch…")),
                m("option")
                    .for_each(&format!("b in {st}.formTargetBranches"))
                    .key("b.name")
                    .bind("value", "b.name")
                    .bind("selected", &format!("({st}.formTarget === b.name ? 'selected' : null)"))
                    .text("b.name"),
            ]),
        m("input")
            .attr("type", "text")
            .attr("placeholder", "title")
            .on("input", &format!("{st}.formInput('formTitle', $event.target.value)")),
        m("textarea")
            .attr("rows", "4")
            .attr("placeholder", "description (markdown)")
            .on("input", &format!("{st}.formInput('formDescription', $event.target.value)")),
        m("textarea")
            .attr("rows", "2")
            .attr("placeholder", "attachment paths (one per line) — files ride the description, never the merge")
            .on("input", &format!("{st}.formInput('formAttachments', $event.target.value)")),
        m("p").class("prp-error").show(&format!("{st}.formError")).text(&format!("{st}.formError")),
        m("div").class("prp-actions").child(
            m("button")
                .class("prp-submit")
                .attr("type", "button")
                .on("click", &format!("{st}.createPr()"))
                .kids(vec![Node::icon("git-pull-request"), m("span").child(lit("open pull request"))]),
        ),
    ])
}

pub fn panel() -> Node {
    let st = vcs::EXPR;
    m("div")
        .class("agent-editor pr-panel")
        .show(&format!("{st}.prOpen || {st}.prFormOpen"))
        .kids(vec![
            pr_form(st).show(&format!("{st}.prFormOpen")),
            detail(st),
        ])
}

fn detail(st: &str) -> Node {
    m("div").class("prp-detail").show(&format!("{st}.prOpen")).kids(vec![
            m("div").class("agent-editor-head").kids(vec![
                Node::icon("git-pull-request"),
                m("span")
                    .class("agent-editor-title")
                    .text(&format!("'#' + {st}.pr.number + ' ' + {st}.pr.title")),
                Node::host(pr_state_pill::NAME, &[]).bind("state", &format!("{st}.pr.state")),
                m("button")
                    .class("prp-btn")
                    .attr("type", "button")
                    .attr("title", "refresh")
                    .on("click", &format!("{st}.refreshPr()"))
                    .child(Node::icon("arrows-clockwise")),
                m("button")
                    .class("prp-btn")
                    .attr("type", "button")
                    .attr("title", "close")
                    .on("click", &format!("{st}.closePr()"))
                    .child(Node::icon("x")),
            ]),
            m("div").class("prp-sub").kids(vec![
                m("code").text(&format!("{st}.pr.sourceBranch")),
                Node::icon("arrow-right"),
                m("code").text(&format!("{st}.pr.targetBranch")),
                m("span").child(rustex1_kit::h::lit("opened by")),
                m("b").text(&format!("{st}.pr.author || 'someone'")),
            ]),
            m("div")
                .class("prp-attach")
                .show(&format!("{st}.pr.attachments && {st}.pr.attachments.length > 0"))
                .kids(vec![m("a")
                    .attr("target", "_blank")
                    .attr("rel", "noopener")
                    .for_each(&format!("at in {st}.pr.attachments"))
                    .key("at.id")
                    .bind("href", &format!("'/api/vcs/pr-attachment?id=' + at.id"))
                    .kids(vec![
                        Node::icon_dyn("(at.mime.indexOf('video') === 0 ? 'ph-video' : at.mime.indexOf('image') === 0 ? 'ph-image' : 'ph-file')"),
                        m("span").text("at.name"),
                    ])]),
            m("div").class("prp-tools").kids(vec![
                m("button")
                    .class("prp-btn")
                    .attr("type", "button")
                    .attr("title", "attach a file from this machine")
                    .show(&format!("{st}.pr.state !== 'merged' && {st}.pr.state !== 'rejected'"))
                    .on("click", &format!("{st}.addAttachment()"))
                    .kids(vec![Node::icon("paperclip"), m("span").text("'attach file'")]),
            ]),
            m("input")
                .class("pr-attach-input")
                .attr("type", "file")
                .attr("aria-hidden", "true")
                .attr("style", "display:none")
                .on("change", &format!("{st}.attachmentChosen($event)")),
            panel_body(st),
        ])
}

fn panel_body(st: &str) -> Node {
    let mut kids = merge_panel::parts(st);
    kids.extend(diff_card::parts(st));
    kids.extend(pr_thread::parts(st, &format!("{st}.pr")));
    kids.extend(review_changes::parts(st));
    kids.extend(review_form::parts(st));
    m("div").class("prp-body").kids(kids)
}

pub fn demo() -> Node {
    panel()
}

pub fn register() {
    css::inject("pr-panel", CSS);
}

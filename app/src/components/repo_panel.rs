use rustex1_kit::h::{lit, m, Node};

use crate::components::vcs;
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .repo-panel { display: flex; flex-direction: column; gap: 10px; }
:is(#sb, #home) .repol-head { display: flex; align-items: center; gap: 8px; min-width: 0; }
:is(#sb, #home) .repol-title {
  flex: 1 1 auto; min-width: 0;
  font-size: 13px; font-weight: 700; color: var(--fg);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .repol-form {
  display: flex; flex-direction: column; gap: 8px;
  border: 1px solid var(--border); border-radius: 12px;
  background: var(--panel-2); padding: 11px 12px;
}
:is(#sb, #home) .repol-readonly {
  display: flex; align-items: center; gap: 6px;
  font-size: 11px; color: var(--gray-dim); min-width: 0;
}
:is(#sb, #home) .repol-readonly code {
  background: var(--panel); border: 1px solid var(--border); border-radius: 5px;
  padding: 1px 6px;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .repol-field { display: flex; flex-direction: column; gap: 4px; }
:is(#sb, #home) .repol-field label {
  font-size: 10.5px; letter-spacing: .04em; text-transform: uppercase; color: var(--gray-dim);
}
:is(#sb, #home) .repol-when {
  font-family: var(--mono); font-size: 11px; color: var(--gray-dim);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
:is(#sb, #home) .repol-form input {
  width: 100%; box-sizing: border-box;
  background: var(--panel); color: var(--fg);
  border: 1px solid var(--border); border-radius: 8px;
  padding: 7px 9px; font: inherit; font-size: 12px;
}
:is(#sb, #home) .repol-form input:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
:is(#sb, #home) .repol-error { margin: 0; font-size: 11px; color: var(--danger, #d9534f); }
:is(#sb, #home) .repol-actions { display: flex; align-items: center; gap: 8px; }
:is(#sb, #home) .repol-submit {
  display: inline-flex; align-items: center; gap: 6px;
  border: 0; border-radius: 8px; padding: 6px 13px;
  background: var(--accent); color: #fff; font: inherit; font-size: 12px; cursor: pointer;
}
:is(#sb, #home) .repol-remove {
  margin-left: auto;
  display: inline-flex; align-items: center; gap: 5px;
  border: 1px solid var(--border); border-radius: 8px;
  padding: 5px 11px; background: var(--panel); color: var(--gray);
  font: inherit; font-size: 11.5px; cursor: pointer;
  transition: color .13s, border-color .13s, background-color .13s;
}
:is(#sb, #home) .repol-remove:hover { color: var(--danger, #d9534f); border-color: var(--danger, #d9534f); }
:is(#sb, #home) .repol-remove.is-confirm {
  color: #fff; background: var(--danger, #d9534f); border-color: var(--danger, #d9534f);
}
:is(#sb, #home) .repol-note { margin: 0; font-size: 10.5px; color: var(--gray-dim); line-height: 1.5; }
.repo-panel-demo { width: min(480px, 100%); }
"#;

pub fn panel() -> Node {
    let st = vcs::EXPR;
    let repo = format!("{st}.repoOpen");
    m("div")
        .class("agent-editor repo-panel")
        .show(&format!("{st}.repoPanelOpen"))
        .kids(vec![
            m("div").class("repol-head").kids(vec![
                Node::icon("database"),
                m("span").class("repol-title").text(&repo),
            ]),
            m("div").class("repol-form").kids(vec![
                m("div").class("repol-readonly").kids(vec![
                    Node::icon("folder-open"),
                    m("code").text(&format!(
                        "(({st}.repos.find(r => r.name === {repo}) || {{}}).path || '')"
                    )),
                ]),
                m("div").class("repol-field").kids(vec![
                    m("label").child(lit("description")),
                    m("input")
                        .attr("type", "text")
                        .attr("placeholder", "what lives in this repo")
                        .model(&format!("{st}.repoDesc")),
                ]),
                m("div").class("repol-field").kids(vec![
                    m("label").child(lit("default branch")),
                    m("input")
                        .attr("type", "text")
                        .attr("spellcheck", "false")
                        .attr("placeholder", "main")
                        .model(&format!("{st}.repoBranch")),
                ]),
                m("div").class("repol-field").kids(vec![
                    m("label").child(lit("created")),
                    m("span")
                        .class("repol-when")
                        .text(
                            &format!(
                                "(({st}.repos.find(r => r.name === {repo})?.createdAt || '').length ? new Date(({st}.repos.find(r => r.name === {repo})?.createdAt)).toLocaleDateString() : '')"
                            )
                        ),
                ]),
                m("p").class("repol-error").show(&format!("{st}.repoError")).text(&format!("{st}.repoError")),
                m("div").class("repol-actions").kids(vec![
                    m("button")
                        .class("repol-submit")
                        .attr("type", "button")
                        .on("click", &format!("{st}.repoSave()"))
                        .kids(vec![Node::icon("check"), m("span").child(lit("save"))]),
                    m("button")
                        .class("repol-remove")
                        .attr("type", "button")
                        .bind("class", &format!(
                            "('repol-remove' + ({st}.removeArmed === {repo} ? ' is-confirm' : ''))"
                        ))
                        .bind("title", &format!(
                            "({st}.removeArmed === {repo} ? 'Click again to untrack' : 'Untrack this repo (the repo on disk is never touched)')"
                        ))
                        .on("click", &format!("{st}.repoRemove()"))
                        .kids(vec![
                            Node::icon("trash"),
                            m("span").text(&format!(
                                "({st}.removeArmed === {repo} ? 'click again to untrack' : 'untrack')"
                            )),
                        ]),
                ]),
                m("p").class("repol-note").child(lit(
                    "untracking removes this row from the registry — the repository, its branches and its history on disk are never deleted",
                )),
            ]),
        ])
}

pub fn demo() -> Node {
    panel()
        .init(
            "$store.vcs.repos = [$store.samples.repoSample]; $store.vcs.openRepo('vibe-arcade');"
        )
}

pub fn register() {
    css::inject("repo-panel", CSS);
}

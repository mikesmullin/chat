use rustex1_kit::h::{lit, m, Node};

use crate::components::{validation_info, vcs};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .repol-note {
  margin: 0; font-size: 11px; color: var(--gray-dim); line-height: 1.5;
}
.repo-panel-demo { width: min(480px, 100%); }
"#;

pub fn panel() -> Node {
    let st = vcs::EXPR;
    let repo = format!("{st}.repoOpen");
    m("div")
        .class("agent-editor repo-panel")
        .show(&format!("{st}.repoPanelOpen"))
        .kids(vec![
            m("div").class("agent-editor-head").kids(vec![
                Node::icon("github-logo"),
                m("span").class("agent-editor-title").child(lit("Edit repository")),
            ]),
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![m("span").child(lit("Name"))]),
                m("span").class("tform-value").text(&repo),
            ]),
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Path")),
                    m("span").class("hint").child(lit("where it lives on this machine")),
                ]),
                m("span").class("tform-value").kids(vec![
                    m("i").class("ph ph-folder-open tform-value-icon"),
                    m("span").text(&format!(
                        "(({st}.repos.find(r => r.name === {repo}) || {{}}).path || '')"
                    )),
                ]),
            ]),
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![m("span").child(lit("Description"))]),
                m("input")
                    .attr("type", "text")
                    .attr("placeholder", "what lives in this repo")
                    .model(&format!("{st}.repoDesc")),
            ]),
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![m("span").child(lit("Default branch"))]),
                m("input")
                    .attr("type", "text")
                    .attr("spellcheck", "false")
                    .attr("placeholder", "main")
                    .model(&format!("{st}.repoBranch")),
            ]),
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![m("span").child(lit("Created"))]),
                m("span").class("tform-value").text(
                    &format!(
                        "(({st}.repos.find(r => r.name === {repo})?.createdAt || '').length ? new Date(({st}.repos.find(r => r.name === {repo})?.createdAt)).toLocaleDateString() : '')"
                    )
                ),
            ]),
            validation_info::live("'error'", &format!("{st}.repoError")),
            m("div").class("ae-actions").kids(vec![
                m("button")
                    .class("ae-btn primary")
                    .attr("type", "button")
                    .on("click", &format!("{st}.repoSave()"))
                    .kids(vec![Node::icon("check"), m("span").child(lit("Save"))]),
                m("span").class("spacer"),
                m("button")
                    .class("ae-btn danger")
                    .attr("type", "button")
                    .bind("class", &format!(
                        "('ae-btn danger' + ({st}.removeArmed === {repo} ? ' is-confirm' : ''))"
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

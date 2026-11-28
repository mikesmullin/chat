use rustex1_kit::h::{lit, m, Node};

use crate::components::{rail_head, vcs};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .wt-list { display: flex; flex-direction: column; gap: 1px; }
:is(#sb, #home) .wt-row {
  display: flex; align-items: center; gap: 6px;
  width: calc(100% - 24px); margin-left: 24px; text-align: left;
  border: 0; border-radius: 7px;
  background: transparent; color: var(--gray);
  padding: 4px 8px; cursor: pointer; font: inherit; font-size: 12px; line-height: 18px;
}
:is(#sb, #home) .wt-row:hover { background: var(--panel-2); color: var(--fg); }
:is(#sb, #home) .wt-row i { color: var(--gray-dim); font-size: 13px; }
:is(#sb, #home) .wt-name {
  flex: 1 1 auto; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .wt-empty { margin: 0; padding: 10px 6px; color: var(--gray-dim); font-size: 11px; }
.worktrees-menu-demo { width: 300px; }
"#;

pub fn menu(live: &str) -> Node {
    let st = vcs::EXPR;
    m("div")
        .class("worktrees-menu")
        .bind("class", "($store.home.worktreesOpen ? '' : 'is-collapsed')")
        .kids(vec![
            rail_head::head("Worktrees", "git-fork", None, Some("worktrees")),
            m("div")
                .class("wt-list")
                .show("$store.home.worktreesOpen")
                .kids(vec![
                    m("div").for_each(&format!("r in {st}.repos")).key(&format!("'wt' + r.id")).kids(vec![
                        m("div")
                            .class("ptree-row wt-group")
                            .bind("class", &format!("({st}.wtOpen[r.id] === false ? '' : 'is-open')"))
                            .show(&format!("{st}.worktreesForRepo(r.id).length > 0"))
                            .bind("title", "r.name")
                            .on("click", &format!("{st}.toggleWtGroup(r.id)"))
                            .kids(vec![
                                m("i").class("ph ph-caret-right ptree-caret"),
                                m("i").class("ph ph-github-logo ptree-icon"),
                                m("span").class("ptree-name").text("r.name"),
                            ]),
                        m("div").class("wt-rows").show(&format!("{st}.wtOpen[r.id] !== false")).kids(vec![
                            m("button")
                                .class("wt-row")
                                .for_each(&format!("w in {st}.worktreesForRepo(r.id)"))
                                .key("w.id")
                                .bind("title", "w.path")
                                .on("click", &format!("{live}.editTarget('worktree', w.id)"))
                                .kids(vec![
                                    Node::icon("git-fork"),
                                    m("span").class("wt-name").text("w.branch"),
                                ]),
                        ]),
                    ]),
                    m("p")
                        .class("wt-empty")
                        .show(&format!("{st}.worktrees.length === 0"))
                        .child(lit("No worktrees tracked yet.")),
                ]),
        ])
}

pub fn demo() -> Node {
    menu(crate::components::samples::EXPR)
}

pub fn register() {
    css::inject("worktrees-menu", CSS);
}

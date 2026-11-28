use rustex1_kit::h::{lit, m, Node};

use crate::components::{rail_head, repo_card, vcs};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .repos-list { display: flex; flex-direction: column; gap: 1px; }
:is(#sb, #home) .repos-empty {
  margin: 0; padding: 10px 6px;
  color: var(--gray-dim); font-size: 11px;
}
.repos-menu-demo { width: 300px; }
"#;

pub fn menu(live: &str) -> Node {
    let st = vcs::EXPR;
    m("div")
        .class("repos-menu")
        .bind("class", "($store.home.reposOpen ? '' : 'is-collapsed')")
        .kids(vec![
            rail_head::head(
                "Repos",
                "github-logo",
                Some((
                    "Add repo",
                    &format!("{live}.createTarget('repo')"),
                    "$store.ident.can('repo.manage')",
                )),
                Some("repos"),
            ),
            m("div")
                .class("repos-list")
                .show("$store.home.reposOpen")
                .kids(vec![
                    m("div")
                        .class("repos-item")
                        .for_each(&format!("r in {st}.repos"))
                        .key("r.id")
                        .kids(repo_card::parts(st, "r")),
                    m("p")
                        .class("repos-empty")
                        .show(&format!("{st}.repos.length === 0"))
                        .child(lit("No repos tracked yet.")),
                ]),
        ])
}

pub fn demo() -> Node {
    menu(crate::components::samples::EXPR)
}

pub fn register() {
    css::inject("repos-menu", CSS);
}

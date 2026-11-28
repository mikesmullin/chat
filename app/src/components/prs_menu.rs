use rustex1_kit::h::{lit, m, Node};

use crate::components::{pr_card, rail_head, vcs};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .prs-list { display: flex; flex-direction: column; gap: 6px; }
:is(#sb, #home) .prs-group {
  padding: 4px 8px 0; font-size: 10px; letter-spacing: .05em;
  text-transform: uppercase; color: var(--gray-dim);
}
:is(#sb, #home) .prs-group-rows { display: flex; flex-direction: column; gap: 4px; }
:is(#sb, #home) .prs-filter {
  display: flex; gap: 4px; padding: 4px 6px 2px;
}
:is(#sb, #home) .prs-filter button {
  border: 1px solid var(--border); border-radius: 999px;
  background: var(--panel); color: var(--gray-dim);
  font: inherit; font-size: 10.5px; padding: 2px 9px; cursor: pointer;
}
:is(#sb, #home) .prs-filter button.on {
  color: var(--accent-soft); border-color: var(--accent); background: var(--accent-tint);
}
:is(#sb, #home) .prs-empty { margin: 0; padding: 10px 6px; color: var(--gray-dim); font-size: 11px; }
.prs-menu-demo { width: 300px; }
"#;

pub fn menu() -> Node {
    let st = vcs::EXPR;
    m("div")
        .class("prs-menu")
        .kids(vec![
            rail_head::head(
                "Pull Requests",
                "git-pull-request",
                Some((
                    "Open PR",
                    &format!("{st}.openPrForm()"),
                    "$store.ident.can('pr.create')",
                )),
                Some("prs"),
            ),
            m("div")
                .class("prs-list")
                .show("$store.home.prsOpen")
                .kids(vec![
                    m("div").class("prs-filter").kids(vec![
                        m("button")
                            .bind("class", &format!("({st}.listState === 'active' ? 'on' : '')"))
                            .attr("type", "button")
                            .on("click", &format!("{st}.setListState('active')"))
                            .child(lit("active")),
                        m("button")
                            .bind("class", &format!("({st}.listState === 'all' ? 'on' : '')"))
                            .attr("type", "button")
                            .on("click", &format!("{st}.setListState('all')"))
                            .child(lit("all")),
                    ]),
                    m("div").for_each(&format!("r in {st}.repos")).key(&format!("'pr' + r.id")).kids(vec![
                        m("div")
                            .class("prs-group")
                            .show(&format!("{st}.prsForRepo(r.name).length > 0"))
                            .text("r.name"),
                        m("div").class("prs-group-rows").kids(vec![
                            m("div")
                                .for_each(&format!("p in {st}.prsForRepo(r.name)"))
                                .key("p.id")
                                .kids(pr_card::parts(st, "p")),
                        ]),
                    ]),
                    m("p")
                        .class("prs-empty")
                        .show(&format!("{st}.prs.length === 0"))
                        .child(lit("No pull requests yet.")),
                ]),
        ])
}

pub fn demo() -> Node {
    menu()
}

pub fn register() {
    css::inject("prs-menu", CSS);
}

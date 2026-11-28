use rustex1_kit::h::{lit, m, Node};

use crate::components::{pr_card, rail_head, vcs};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .prs-settings { position: relative; display: inline-flex; flex: 0 0 auto; }
:is(#sb, #home) .prs-settings.is-open .rail-add { background: var(--panel-2); color: var(--accent); }
:is(#sb, #home) .prs-settings-menu {
  position: absolute; bottom: calc(100% + 6px); right: 0; z-index: 40;
  min-width: 148px; padding: 4px;
  display: flex; flex-direction: column; gap: 1px;
  background: var(--panel-2); border: 1px solid var(--border); border-radius: 10px;
  box-shadow: 0 10px 28px rgba(0, 0, 0, .55);
}
:is(#sb, #home) .prs-settings-label {
  padding: 5px 8px 3px; font-size: 10px; letter-spacing: .05em;
  text-transform: uppercase; color: var(--gray-dim);
}
.prs-menu-demo { width: 300px; }
"#;

fn settings_menu() -> Node {
    let st = vcs::EXPR;
    m("div")
        .class("prs-settings rail-middle")
        .bind("class", "($store.dropdown.open === 'prs-filter' ? 'is-open' : '')")
        .on("click.outside", "$store.dropdown.closeIf('prs-filter')")
        .kids(vec![
            m("button")
                .class("rail-add")
                .attr("type", "button")
                .attr("title", "Filter pull requests")
                .attr("aria-label", "Filter pull requests")
                .on("click", "$store.dropdown.toggle('prs-filter')")
                .child(Node::icon("funnel")),
            m("div")
                .class("prs-settings-menu")
                .show("$store.dropdown.open === 'prs-filter'")
                .kids(vec![
                    m("span").class("prs-settings-label").child(lit("Filter")),
                    m("button")
                        .class("dropdown-item")
                        .attr("type", "button")
                        .bind("class", &format!("({st}.listState === 'active' ? 'on' : '')"))
                        .on("click", &format!("{st}.setListState('active'); $store.dropdown.closeIf('prs-filter')"))
                        .kids(vec![
                            m("span").class("dropdown-item-label").child(lit("active")),
                            m("i").class("ph ph-check dropdown-item-check").show(&format!("{st}.listState === 'active'")),
                        ]),
                    m("button")
                        .class("dropdown-item")
                        .attr("type", "button")
                        .bind("class", &format!("({st}.listState === 'all' ? 'on' : '')"))
                        .on("click", &format!("{st}.setListState('all'); $store.dropdown.closeIf('prs-filter')"))
                        .kids(vec![
                            m("span").class("dropdown-item-label").child(lit("all")),
                            m("i").class("ph ph-check dropdown-item-check").show(&format!("{st}.listState === 'all'")),
                        ]),
                ]),
        ])
}

pub fn menu() -> Node {
    let st = vcs::EXPR;
    m("div")
        .class("prs-menu")
        .bind("class", "($store.home.prsOpen ? '' : 'is-collapsed')")
        .kids(vec![
            rail_head::head_with(
                "Pull Requests",
                "git-pull-request",
                Some(settings_menu()),
                Some((
                    "Open PR",
                    &format!("{st}.openPrForm()"),
                    "$store.ident.can('pr.create')",
                )),
                Some("prs"),
            ),
            m("div")
                .class("wt-list")
                .show("$store.home.prsOpen")
                .kids(vec![
                    m("div").for_each(&format!("r in {st}.repos")).key(&format!("'pr' + r.id")).kids(vec![
                        m("div")
                            .class("ptree-row wt-group")
                            .bind("class", &format!("({st}.prsOpen[r.name] === false ? '' : 'is-open')"))
                            .show(&format!("{st}.prsForRepo(r.name).length > 0"))
                            .bind("title", "r.name")
                            .on("click", &format!("{st}.togglePrsGroup(r.name)"))
                            .kids(vec![
                                m("i").class("ph ph-caret-right ptree-caret"),
                                m("i").class("ph ph-github-logo ptree-icon"),
                                m("span").class("ptree-name").text("r.name"),
                            ]),
                        m("div").class("wt-rows").show(&format!("{st}.prsOpen[r.name] !== false")).kids(vec![
                            m("div")
                                .for_each(&format!("p in {st}.prsForRepo(r.name)"))
                                .key("p.id")
                                .kids(pr_card::parts(st, "p")),
                        ]),
                    ]),
                    m("p")
                        .class("wt-empty")
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

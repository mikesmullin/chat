use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

pub const NAME: &str = "uiPrStatePill";

pub const CSS: &str = r#"
.pr-pill {
  display: inline-flex; align-items: center; gap: 6px;
  padding: 4px 14px; border-radius: 999px;
  font-size: 13px; font-weight: 700; letter-spacing: .01em;
  color: #fff; white-space: nowrap;
}
.pr-pill .ph { font-size: 14px; line-height: 1; }
.pr-pill.p-open { background: var(--pastel-green, #22c55e); }
.pr-pill.p-review { background: var(--mjs-yellow); color: #201a05; }
.pr-pill.p-approved { background: var(--accent); }
.pr-pill.p-merged { background: var(--pastel-purple, #a78bfa); }
.pr-pill.p-rejected { background: var(--danger, #d9534f); }
.pr-pill-demo { display: flex; gap: 10px; flex-wrap: wrap; }
"#;

pub fn mount(state: &str) -> Node {
    Node::host(NAME, &[("state", state)])
}

pub fn demo() -> Node {
    m("div").class("pr-pill-demo").kids(vec![
        mount("open"),
        mount("review"),
        mount("approved"),
        mount("merged"),
        mount("rejected"),
    ])
}

pub fn register() {
    css::inject(NAME, CSS);
    let mut d = Def::new().named(NAME);
    d.prop_str("state", "open").tree(
        m("span")
            .bind("class", "('pr-pill p-' + (state || 'open'))")
            .kids(vec![
                m("i").class("ph").bind(
                    "class",
                    &format!(
                        "'ph ' + {}",
                        "({open:'ph-git-pull-request',review:'ph-eye',approved:'ph-seal-check',merged:'ph-git-merge',rejected:'ph-prohibit'}[state] || 'ph-question')"
                    ),
                ),
                m("span").text(
                    "({open:'Open',review:'In review',approved:'Approved',merged:'Merged',rejected:'Rejected'}[state] || state || 'Unknown')",
                ),
            ]),
    );
    mjs::component(NAME, &d.finish());
}

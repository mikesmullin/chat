use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

pub const NAME: &str = "uiPrStateBadge";

pub const CSS: &str = r#"
.pr-badge {
  display: inline-flex; align-items: center; gap: 4px;
  padding: 1px 8px; border-radius: 999px;
  font-size: 11px; font-weight: 600; letter-spacing: .02em;
  border: 1px solid transparent; white-space: nowrap;
}
.pr-badge .ph { font-size: 11px; line-height: 1; }
.pr-badge.st-open { background: color-mix(in srgb, var(--pastel-green, #22c55e) 15%, transparent); color: var(--pastel-green, #22c55e); border-color: color-mix(in srgb, var(--pastel-green, #22c55e) 35%, transparent); }
.pr-badge.st-review { background: color-mix(in srgb, var(--mjs-yellow) 15%, transparent); color: var(--mjs-yellow); border-color: color-mix(in srgb, var(--mjs-yellow) 35%, transparent); }
.pr-badge.st-approved { background: color-mix(in srgb, var(--accent) 15%, transparent); color: var(--accent-soft); border-color: color-mix(in srgb, var(--accent) 35%, transparent); }
.pr-badge.st-merged { background: color-mix(in srgb, var(--pastel-purple, #a78bfa) 18%, transparent); color: var(--pastel-purple, #a78bfa); border-color: color-mix(in srgb, var(--pastel-purple, #a78bfa) 40%, transparent); }
.pr-badge.st-rejected { background: color-mix(in srgb, var(--danger, #d9534f) 12%, transparent); color: var(--danger, #d9534f); border-color: color-mix(in srgb, var(--danger, #d9534f) 32%, transparent); }
.pr-badge-demo { display: flex; gap: 10px; flex-wrap: wrap; }
"#;

pub const ICON_EXPR: &str =
    "({open:'ph-git-pull-request',review:'ph-eye',approved:'ph-seal-check',merged:'ph-git-merge',rejected:'ph-prohibit'}[state] || 'ph-question')";
pub const LABEL_EXPR: &str =
    "({open:'open',review:'review',approved:'approved',merged:'merged',rejected:'rejected'}[state] || state || 'unknown')";

pub fn mount(state: &str) -> Node {
    Node::host(NAME, &[("state", state)])
}

pub fn demo() -> Node {
    m("div").class("pr-badge-demo").kids(vec![
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
            .class("pr-badge")
            .bind("class", "('pr-badge st-' + (state || 'open'))")
            .bind("title", "state")
            .kids(vec![
                m("i").class("ph").bind("class", &format!("'ph ' + {ICON_EXPR}")),
                m("span").text(LABEL_EXPR),
            ]),
    );
    mjs::component(NAME, &d.finish());
}

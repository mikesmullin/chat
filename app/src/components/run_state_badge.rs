use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

pub const NAME: &str = "uiRunStateBadge";

pub const CSS: &str = r#"
.run-badge {
  display: inline-flex; align-items: center; gap: 4px;
  padding: 1px 8px; border-radius: 999px;
  font-size: 11px; font-weight: 600;
  border: 1px solid transparent; white-space: nowrap;
}
.run-badge .ph { font-size: 11px; line-height: 1; }
.run-badge.rs-queued, .run-badge.rs-pending { background: var(--panel-2); color: var(--gray-dim); border-color: var(--border); }
.run-badge.rs-provisioning, .run-badge.rs-posting, .run-badge.rs-stopping { background: color-mix(in srgb, var(--mjs-yellow) 14%, transparent); color: var(--mjs-yellow); border-color: color-mix(in srgb, var(--mjs-yellow) 34%, transparent); }
.run-badge.rs-running { background: color-mix(in srgb, var(--accent) 14%, transparent); color: var(--accent-soft); border-color: color-mix(in srgb, var(--accent) 34%, transparent); }
.run-badge.rs-blocked { background: color-mix(in srgb, var(--pastel-amber, #f5d8a6) 18%, transparent); color: var(--pastel-amber, #f5d8a6); border-color: color-mix(in srgb, var(--pastel-amber, #f5d8a6) 38%, transparent); }
.run-badge.rs-success { background: color-mix(in srgb, var(--pastel-green, #22c55e) 14%, transparent); color: var(--pastel-green, #22c55e); border-color: color-mix(in srgb, var(--pastel-green, #22c55e) 34%, transparent); }
.run-badge.rs-failure, .run-badge.rs-aborted { background: color-mix(in srgb, var(--danger, #d9534f) 12%, transparent); color: var(--danger, #d9534f); border-color: color-mix(in srgb, var(--danger, #d9534f) 32%, transparent); }
.run-badge.rs-skipped { background: transparent; color: var(--gray-dim); border-color: var(--border); opacity: .7; }
.run-badge-demo { display: flex; gap: 10px; flex-wrap: wrap; }
"#;

pub fn mount(state: &str) -> Node {
    Node::host(NAME, &[("state", state)])
}

pub fn demo() -> Node {
    m("div").class("run-badge-demo").kids(vec![
        mount("queued"),
        mount("provisioning"),
        mount("running"),
        mount("blocked"),
        mount("posting"),
        mount("success"),
        mount("failure"),
        mount("aborted"),
        mount("skipped"),
    ])
}

pub fn register() {
    css::inject(NAME, CSS);
    let mut d = Def::new().named(NAME);
    d.prop_str("state", "queued").tree(
        m("span")
            .bind("class", "('run-badge rs-' + (state || 'queued'))")
            .bind("title", "state")
            .kids(vec![
                m("i").class("ph").bind(
                    "class",
                    "'ph ' + ({queued:'ph-inbox',pending:'ph-circle',provisioning:'ph-cube',running:'ph-circle-notch',blocked:'ph-pause-circle',posting:'ph-paper-plane-tilt',stopping:'ph-stop-circle',success:'ph-seal-check',failure:'ph-x-circle',aborted:'ph-prohibit',skipped:'ph-minus-circle'}[state] || 'ph-question')",
                ),
                m("span").text("state || 'unknown'"),
            ]),
    );
    mjs::component(NAME, &d.finish());
}

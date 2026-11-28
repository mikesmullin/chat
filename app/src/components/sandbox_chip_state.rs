use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

pub const NAME: &str = "uiSandboxChip";

pub const CSS: &str = r#"
.sbx-chip {
  display: inline-flex; align-items: center; gap: 4px;
  padding: 1px 8px; border-radius: 999px;
  font-size: 10.5px; font-weight: 600;
  border: 1px solid var(--border); background: var(--panel);
  color: var(--gray); white-space: nowrap;
}
.sbx-chip .ph { font-size: 10.5px; line-height: 1; }
.sbx-chip.sc-ready { color: var(--pastel-green, #22c55e); border-color: color-mix(in srgb, var(--pastel-green, #22c55e) 40%, transparent); }
.sbx-chip.sc-running { color: var(--mjs-yellow); }
.sbx-chip.sc-error, .sbx-chip.sc-dead { color: var(--danger, #d9534f); border-color: color-mix(in srgb, var(--danger, #d9534f) 38%, transparent); }
.sbx-chip-demo { display: flex; gap: 10px; flex-wrap: wrap; }
"#;

pub fn mount(state: &str) -> Node {
    Node::host(NAME, &[("state", state)])
}

pub fn demo() -> Node {
    m("div").class("sbx-chip-demo").kids(vec![
        mount("provisioning"),
        mount("ready"),
        mount("running"),
        mount("error"),
    ])
}

pub fn register() {
    css::inject(NAME, CSS);
    let mut d = Def::new().named(NAME);
    d.prop_str("state", "ready")
        .prop_str("id", "")
        .tree(
            m("span")
                .bind("class", "('sbx-chip sc-' + (state || 'ready'))")
                .bind("title", "('sandbox ' + (id || ''))")
                .kids(vec![
                    m("i").bind(
                        "class",
                        "('ph ' + ({provisioning:'ph-cube',ready:'ph-check-fat',running:'ph-circle-notch',error:'ph-x-circle',dead:'ph-prohibit'}[state] || 'ph-cube'))",
                    ),
                    m("span").text("state || 'ready'"),
                ]),
        );
    mjs::component(NAME, &d.finish());
}

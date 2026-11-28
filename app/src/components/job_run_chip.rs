use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

pub const NAME: &str = "uiJobRunChip";

pub const CSS: &str = r#"
.run-chip {
  display: inline-flex; align-items: center; gap: 4px;
  padding: 1px 8px; border-radius: 999px;
  font-size: 10.5px; font-weight: 600;
  border: 1px solid var(--border); background: var(--panel);
  color: var(--gray); white-space: nowrap;
}
.run-chip .ph { font-size: 10.5px; line-height: 1; }
.run-chip.rc-queued, .run-chip.rc-pending { color: var(--gray-dim); }
.run-chip.rc-running, .run-chip.rc-provisioning, .run-chip.rc-posting, .run-chip.rc-stopping { color: var(--mjs-yellow); }
.run-chip.rc-blocked { color: var(--pastel-amber, #f5d8a6); }
.run-chip.rc-success { color: var(--pastel-green, #22c55e); border-color: color-mix(in srgb, var(--pastel-green, #22c55e) 40%, transparent); }
.run-chip.rc-failure, .run-chip.rc-aborted { color: var(--danger, #d9534f); border-color: color-mix(in srgb, var(--danger, #d9534f) 38%, transparent); }
.run-chip.rc-skipped { color: var(--gray-dim); opacity: .7; }
.run-chip-spin { animation: run-chip-spin 1s linear infinite; }
@keyframes run-chip-spin { to { transform: rotate(360deg); } }
.run-chip-demo { display: flex; gap: 10px; flex-wrap: wrap; }
"#;

pub fn mount(state: &str) -> Node {
    Node::host(NAME, &[("state", state)])
}

pub fn demo() -> Node {
    m("div").class("run-chip-demo").kids(vec![
        mount("queued"),
        mount("running"),
        mount("blocked"),
        mount("success"),
        mount("failure"),
    ])
}

pub fn register() {
    css::inject(NAME, CSS);
    let mut d = Def::new().named(NAME);
    d.prop_str("state", "queued").tree(
        m("span")
            .bind("class", "('run-chip rc-' + (state || 'queued'))")
            .bind("title", "('job run — ' + (state || 'queued'))")
            .kids(vec![
                m("i").bind(
                    "class",
                    "('ph ' + ({queued:'ph-inbox',pending:'ph-circle',provisioning:'ph-cube run-chip-spin',running:'ph-circle-notch run-chip-spin',blocked:'ph-pause-circle',posting:'ph-paper-plane-tilt',stopping:'ph-stop-circle',success:'ph-seal-check',failure:'ph-x-circle',aborted:'ph-prohibit',skipped:'ph-minus-circle'}[state] || 'ph-question'))",
                ),
                m("span").text("state || 'unknown'"),
            ]),
    );
    mjs::component(NAME, &d.finish());
}

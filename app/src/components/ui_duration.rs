use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

pub const NAME: &str = "uiDuration";

pub const CSS: &str = r#"
.ui-duration {
  font-variant-numeric: tabular-nums;
  font-size: 11px; color: var(--gray-dim); white-space: nowrap;
}
.ui-duration-demo { display: flex; gap: 14px; }
"#;

pub fn mount(ms: &str) -> Node {
    Node::host(NAME, &[("ms", ms)])
}

pub fn demo() -> Node {
    m("div").class("ui-duration-demo").kids(vec![
        mount("4200"),
        mount("83000"),
        mount("3723000"),
    ])
}

pub fn register() {
    css::inject(NAME, CSS);
    let mut d = Def::new().named(NAME);
    d.prop_num("ms", 0.0).tree(
        m("span")
            .class("ui-duration")
            .text("((ms || 0) >= 3600000 ? Math.floor((ms || 0) / 3600000) + 'h ' : '') + ((ms || 0) >= 60000 ? Math.floor(((ms || 0) % 3600000) / 60000) + 'm ' : '') + (Math.floor(((ms || 0) % 60000) / 1000)) + 's'"),
    );
    mjs::component(NAME, &d.finish());
}

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

pub const NAME: &str = "uiTooltip";

pub const CSS: &str = r#"
.ui-tip { position: relative; display: inline-flex; align-items: center; gap: 4px; }
.ui-tip .ui-tip-body {
  position: absolute; bottom: calc(100% + 6px); left: 50%; transform: translateX(-50%);
  padding: 4px 9px; border-radius: 7px;
  background: var(--panel-2); color: var(--fg);
  border: 1px solid var(--border);
  font-size: 11px; line-height: 1.4; white-space: nowrap;
  box-shadow: 0 6px 18px rgba(0, 0, 0, .35);
  opacity: 0; pointer-events: none; transition: opacity .12s;
  z-index: 60;
}
.ui-tip:hover .ui-tip-body, .ui-tip:focus-within .ui-tip-body { opacity: 1; }
.ui-tip-demo { display: flex; gap: 18px; }
"#;

pub fn mount(text: &str, label: &str) -> Node {
    Node::host(NAME, &[("text", text), ("label", label)])
}

pub fn demo() -> Node {
    m("div").class("ui-tip-demo").kids(vec![
        mount("the run's absolute start time", "hover me"),
        mount("merged as 734c042", "chip"),
    ])
}

pub fn register() {
    css::inject(NAME, CSS);
    let mut d = Def::new().named(NAME);
    d.prop_str("text", "")
        .prop_str("label", "")
        .tree(
            m("span")
                .class("ui-tip")
                .attr("tabindex", "0")
                .kids(vec![
                    m("span").text("label"),
                    m("span").class("ui-tip-body").text("text"),
                ]),
        );
    mjs::component(NAME, &d.finish());
    let _ = lit;
}

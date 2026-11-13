//! `radio input` — a single-choice segmented control: the kit `uiChoice`
//! (the `select` replacement: one click, arrow keys walk) with no label of
//! its own. Backs the sandbox and model fields.

use rustex1_kit::h::{m, Node};

use crate::css;

pub const CSS: &str = r#"
.radio-demo { width: min(320px, 100%); }
"#;

/// bare control for composition.
pub fn bare(options: &[&str]) -> Node {
    Node::host("uiChoice", &[("options", &options.join(","))])
}

/// form-bound control.
pub fn input(name: &str, model: &str, options: &[&str]) -> Node {
    bare(options).attr("name", name).bind("model", model)
}

/// standalone demo: three-way choice, starts unselected.
pub fn demo() -> Node {
    use crate::components::demo::MODEL;
    m("div")
        .class("radio-demo")
        .child(input("radio", MODEL, &["opus", "sonnet", "haiku"]))
}

pub fn register() {
    css::inject("radio-input", CSS);
}

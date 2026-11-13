//! `toggle button` — a bare boolean switch: the kit `uiToggle` with no
//! label of its own (`field` provides it). Clicks flip the widget's value;
//! model-free instances run on `localValue`.

use rustex1_kit::h::{m, Node};

use crate::css;

pub const CSS: &str = r#"
.toggle-demo { display: flex; }
"#;

/// bare control for composition.
pub fn bare() -> Node {
    Node::host("uiToggle", &[])
}

/// form-bound control.
pub fn input(name: &str, model: &str) -> Node {
    bare().attr("name", name).bind("model", model)
}

/// standalone demo: one bound switch, starts off, flips on click.
pub fn demo() -> Node {
    use crate::components::demo::MODEL;
    m("div").class("toggle-demo").child(input("toggle", MODEL))
}

pub fn register() {
    css::inject("toggle-button", CSS);
}

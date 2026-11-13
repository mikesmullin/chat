//! `range input` — the numeric stepper: the kit `uiNumber` (− / +
//! buttons around a number box) with no label of its own. Backs the timeout
//! field (minutes).

use rustex1_kit::h::{m, Node};

use crate::css;

pub const CSS: &str = r#"
.range-demo { width: min(200px, 100%); }
"#;

/// bare control for composition.
pub fn bare() -> Node {
    Node::host("uiNumber", &[])
}

/// form-bound control.
pub fn input(name: &str, model: &str) -> Node {
    bare().attr("name", name).bind("model", model)
}

/// standalone demo: one bound stepper.
pub fn demo() -> Node {
    use crate::components::demo::MODEL;
    m("div").class("range-demo").child(input("range", MODEL))
}

pub fn register() {
    css::inject("range-input", CSS);
}

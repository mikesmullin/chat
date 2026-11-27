use rustex1_kit::h::{m, Node};

use crate::css;

pub const CSS: &str = r#"
.range-demo { width: min(200px, 100%); }
"#;

pub fn bare() -> Node {
    Node::host("uiNumber", &[])
}

pub fn input(name: &str, model: &str) -> Node {
    bare().attr("name", name).bind("model", model)
}

pub fn demo() -> Node {
    use crate::components::demo::MODEL;
    m("div").class("range-demo").child(input("range", MODEL))
}

pub fn register() {
    css::inject("range-input", CSS);
}

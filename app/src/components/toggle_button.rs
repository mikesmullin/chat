use rustex1_kit::h::{m, Node};

use crate::css;

pub const CSS: &str = r#"
.toggle-demo { display: flex; }
"#;

pub fn bare() -> Node {
    Node::host("uiToggle", &[])
}

pub fn input(name: &str, model: &str) -> Node {
    bare().attr("name", name).bind("model", model)
}

pub fn demo() -> Node {
    use crate::components::demo::MODEL;
    m("div").class("toggle-demo").child(input("toggle", MODEL))
}

pub fn register() {
    css::inject("toggle-button", CSS);
}

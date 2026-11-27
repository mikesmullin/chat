use rustex1_kit::h::{m, Node};

use crate::css;

pub const CSS: &str = r#"
.text-input-demo {
  display: flex; flex-direction: column; gap: 12px;
  width: min(320px, 100%);
}
"#;

pub fn bare(placeholder: &str) -> Node {
    Node::host("uiText", &[("placeholder", placeholder)])
}

pub fn input(name: &str, model: &str, placeholder: &str) -> Node {
    bare(placeholder).attr("name", name).bind("model", model)
}

pub fn demo() -> Node {
    use crate::components::demo::MODEL;
    m("div")
        .class("text-input-demo")
        .kids(vec![
            input("text1", MODEL, "my-automation"),
            input("text2", MODEL, "0 9 * * *"),
        ])
}

pub fn register() {
    css::inject("text-input", CSS);
}

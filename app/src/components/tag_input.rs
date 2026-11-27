use rustex1_kit::h::{m, Node};

use crate::css;

pub const CSS: &str = r#"
.tag-input-demo { width: min(320px, 100%); }
"#;

pub fn bare(placeholder: &str) -> Node {
    Node::host("uiChips", &[("placeholder", placeholder)])
}

pub fn input(name: &str, model: &str, placeholder: &str) -> Node {
    bare(placeholder).attr("name", name).bind("model", model)
}

pub fn demo() -> Node {
    use crate::components::demo::MODEL;
    m("div")
        .class("tag-input-demo")
        .child(input("tags", MODEL, "read, bash…"))
}

pub fn register() {
    css::inject("tag-input", CSS);
}

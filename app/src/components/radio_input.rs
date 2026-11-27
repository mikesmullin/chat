use rustex1_kit::h::{m, Node};

use crate::css;

pub const CSS: &str = r#"
.radio-demo { width: min(320px, 100%); }
"#;

pub fn bare(options: &[&str]) -> Node {
    Node::host("uiChoice", &[("options", &options.join(","))])
}

pub fn input(name: &str, model: &str, options: &[&str]) -> Node {
    bare(options).attr("name", name).bind("model", model)
}

pub fn demo() -> Node {
    use crate::components::demo::MODEL;
    m("div")
        .class("radio-demo")
        .child(input("radio", MODEL, &["opus", "sonnet", "haiku"]))
}

pub fn register() {
    css::inject("radio-input", CSS);
}

use rustex1_kit::h::{m, Node};

use crate::css;

pub const CSS: &str = r#"
.text-area-demo { width: min(360px, 100%); }
"#;

pub fn bare(placeholder: &str, rows: f64) -> Node {
    Node::host("uiArea", &[("placeholder", placeholder), ("rows", &format!("{rows:.0}"))])
}

pub fn input(name: &str, model: &str, placeholder: &str, rows: f64) -> Node {
    bare(placeholder, rows)
        .attr("name", name)
        .bind("model", model)
}

pub fn demo() -> Node {
    use crate::components::demo::MODEL;
    m("div")
        .class("text-area-demo")
        .child(input("area", MODEL, "What should be done on schedule…", 3.0))
}

pub fn register() {
    css::inject("text-area", CSS);
}

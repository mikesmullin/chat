use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.label-demo { display: flex; flex-direction: column; gap: 12px; }
"#;

pub fn label(text: &str, required: bool) -> Node {
    let mut kids = vec![m("span").child(lit(text))];
    if required {
        kids.push(m("em").class("req").child(lit("*")));
    }
    m("span").class("field-label").kids(kids)
}

pub fn demo() -> Node {
    m("div")
        .class("label-demo")
        .kids(vec![label("Name", true), label("Schedule", false)])
}

pub fn register() {
    css::inject("label", CSS);
}

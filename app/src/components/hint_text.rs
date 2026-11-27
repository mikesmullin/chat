use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.hint-demo { display: flex; flex-direction: column; gap: 12px; }
"#;

pub fn hint(text: &str) -> Node {
    m("span").class("field-hint").child(lit(text))
}

pub fn demo() -> Node {
    m("div")
        .class("hint-demo")
        .kids(vec![hint("crontab"), hint("absolute path")])
}

pub fn register() {
    css::inject("hint-text", CSS);
}

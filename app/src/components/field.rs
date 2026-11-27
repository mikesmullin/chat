use rustex1_kit::h::{m, Node};

use crate::components::{hint_text, label, text_input};
use crate::css;

pub const CSS: &str = r#"
.field-demo { width: min(360px, 100%); }
"#;

pub fn field(label_text: &str, required: bool, hint_text: &str, control: Node, wide: bool) -> Node {
    m("div")
        .class(if wide { "field wide" } else { "field" })
        .kids(vec![
            label::label(label_text, required),
            control,
            hint_text::hint(hint_text),
        ])
}

pub fn demo() -> Node {
    use crate::components::demo::MODEL;
    m("div").class("field-demo").child(field(
        "Schedule",
        true,
        "crontab",
        text_input::input("field", MODEL, "0 9 * * *"),
        false,
    ))
}

pub fn register() {
    css::inject("field", CSS);
}

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.field-demo { width: min(360px, 100%); }
"#;

pub fn field(label_text: &str, required: bool, hint_text: &str, control: Node, wide: bool) -> Node {
    let mut label_kids = vec![m("span").child(lit(label_text))];
    if required {
        label_kids.push(m("em").class("req").child(lit("*")));
    }
    label_kids.push(m("span").class("hint").child(lit(hint_text)));
    m("div")
        .class(if wide { "ae-field wide" } else { "ae-field" })
        .kids(vec![
            m("label").class("ae-label").kids(label_kids),
            control,
        ])
}

pub fn demo() -> Node {
    use crate::components::demo::MODEL;
    let input = m("input")
        .attr("type", "text")
        .attr("spellcheck", "false")
        .attr("placeholder", "acme-corp")
        .model(&format!("{MODEL}.field"));
    m("div")
        .class("field-demo")
        .child(field("Name", false, "absolute path", input, false))
}

pub fn register() {
    css::inject("field", CSS);
}

//! `label` — a field label: uppercase mono word plus a required asterisk.
//!
//! Own markup reusing the kit `.field-label` / `.req` classes (the kit's
//! tokens already adapt to the storybook charcoal). No inputs, no hints —
//! `field` puts those around it.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.label-demo { display: flex; flex-direction: column; gap: 12px; }
"#;

/// the label itself: word + `*` when required.
pub fn label(text: &str, required: bool) -> Node {
    let mut kids = vec![m("span").child(lit(text))];
    if required {
        kids.push(m("em").class("req").child(lit("*")));
    }
    m("span").class("field-label").kids(kids)
}

/// standalone demo: one required label, one optional.
pub fn demo() -> Node {
    m("div")
        .class("label-demo")
        .kids(vec![label("Name", true), label("Schedule", false)])
}

pub fn register() {
    css::inject("label", CSS);
}

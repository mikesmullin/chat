//! `text input` — a bare single-line input: the kit `uiText` with no
//! label or hint of its own (`field` provides those). Model-free instances
//! stay typeable through the widget's `localValue` fallback.

use rustex1_kit::h::{m, Node};

use crate::css;

pub const CSS: &str = r#"
.text-input-demo {
  display: flex; flex-direction: column; gap: 12px;
  width: min(320px, 100%);
}
"#;

/// bare control for composition (the form binds name + model onto it).
pub fn bare(placeholder: &str) -> Node {
    Node::host("uiText", &[("placeholder", placeholder)])
}

/// form-bound control.
pub fn input(name: &str, model: &str, placeholder: &str) -> Node {
    bare(placeholder).attr("name", name).bind("model", model)
}

/// standalone demo: two bound inputs.
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

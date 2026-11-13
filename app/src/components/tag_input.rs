//! `tag input` — a token box: the kit `uiChips` (Enter/Space commits,
//! Backspace pops, × removes) with no label of its own. Its comma-joined
//! value stays a plain string. Backs the tool-allowlist field.

use rustex1_kit::h::{m, Node};

use crate::css;

pub const CSS: &str = r#"
.tag-input-demo { width: min(320px, 100%); }
"#;

/// bare control for composition.
pub fn bare(placeholder: &str) -> Node {
    Node::host("uiChips", &[("placeholder", placeholder)])
}

/// form-bound control.
pub fn input(name: &str, model: &str, placeholder: &str) -> Node {
    bare(placeholder).attr("name", name).bind("model", model)
}

/// standalone demo: one bound box.
pub fn demo() -> Node {
    use crate::components::demo::MODEL;
    m("div")
        .class("tag-input-demo")
        .child(input("tags", MODEL, "read, bash…"))
}

pub fn register() {
    css::inject("tag-input", CSS);
}

//! `hint text` — the small hint line under a field (`crontab`,
//! `absolute path`). Own markup reusing the kit `.field-hint` class, retinted
//! to the storybook muted token in the global layer.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.hint-demo { display: flex; flex-direction: column; gap: 12px; }
"#;

/// the hint itself.
pub fn hint(text: &str) -> Node {
    m("span").class("field-hint").child(lit(text))
}

/// standalone demo: two sample hints.
pub fn demo() -> Node {
    m("div")
        .class("hint-demo")
        .kids(vec![hint("crontab"), hint("absolute path")])
}

pub fn register() {
    css::inject("hint-text", CSS);
}

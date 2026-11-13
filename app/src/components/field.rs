//! `field` — one labelled form row: `label` + a control + `hint text`,
//! in a plain `div.field` (not a `<label>`: the kit widgets already render
//! their own `<label class="field">` inside, and nested labels are invalid
//! HTML — the parser would silently repair them).
//!
//! The control is any bare input (`text_input::bare`, `toggle_button::bare`,
//! …). Depth 3: label + text input + hint text.

use rustex1_kit::h::{m, Node};

use crate::components::{hint_text, label, text_input};
use crate::css;

pub const CSS: &str = r#"
.field-demo { width: min(360px, 100%); }
"#;

/// one row: label on top, control, hint below. `wide` spans both grid
/// columns of the form.
pub fn field(label_text: &str, required: bool, hint_text: &str, control: Node, wide: bool) -> Node {
    m("div")
        .class(if wide { "field wide" } else { "field" })
        .kids(vec![
            label::label(label_text, required),
            control,
            hint_text::hint(hint_text),
        ])
}

/// standalone demo: one text row, the shape every form row shares.
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

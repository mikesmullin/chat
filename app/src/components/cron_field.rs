use rustex1_kit::h::{lit, m, Node};

use crate::components::text_input;
use crate::css;

pub const CSS: &str = r#"
.cron-field-demo { width: min(360px, 100%); }
.cron-field { display: flex; flex-direction: column; gap: 5px; min-width: 0; }
.cron-field-input {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--panel-2);
  border: 1px solid var(--line-2);
  border-radius: 9px;
  padding: 5px 10px;
}
.cron-field-input > .ph { color: var(--gray-dim); font-size: 14px; line-height: 1; flex: 0 0 auto; }
.cron-field-input > span { flex: 1 1 auto; min-width: 0; display: block; }
.cron-field input[type="text"] {
  width: 100%;
  min-width: 0;
  min-height: 0;
  padding: 0;
  border: 0;
  border-radius: 0;
  background: transparent;
  color: var(--text);
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.5;
  outline: none;
}
:is(#sb, #home) .cron-field input[type="text"] {
  border: 0;
  background: transparent;
  padding: 0;
}
:is(#sb, #home) .cron-field input[type="text"]:focus {
  border-color: transparent;
  box-shadow: none;
  outline: none;
}
:is(#sb, #home) .cron-field-input:focus-within {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-tint);
}
.cron-field-hint { display: flex; flex-direction: column; gap: 3px; margin-top: 2px; }
.cron-field-line { display: block; font: 11px/1.4 var(--sans); font-style: italic; white-space: pre-wrap; color: var(--text); }
.cron-field-line.cron-field-epoch { font-family: var(--sans); font-style: normal; color: var(--gray-dim); opacity: .7; font-size: 10.5px; }
"#;

pub fn field(label_text: &str, model: &str, wide: bool) -> Node {
    m("div")
        .class(if wide { "field cron-field wide" } else { "field cron-field" })
        .kids(vec![
            m("label")
                .class("ae-label")
                .child(m("span").child(lit(label_text))),
            m("div")
                .class("cron-field-input")
                .kids(vec![
                    Node::icon("clock"),
                    text_input::input("cron", model, "m h d M W"),
                ]),
            m("span").class("field-hint cron-field-hint").kids(vec![
                m("span")
                    .class("cron-field-line")
                    .text(&format!("{model}.cronDescription()")),
                m("span")
                    .class("cron-field-line cron-field-epoch")
                    .text(&format!("{model}.cronNext()")),
            ]),
        ])
}

pub fn demo() -> Node {
    use crate::components::demo::MODEL;
    m("div")
        .class("cron-field-demo")
        .child(field("Schedule", MODEL, false))
}

pub fn register() {
    css::inject("cron-field", CSS);
}

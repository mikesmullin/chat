use rustex1_kit::h::{m, Node};

use crate::css;

pub const CSS: &str = r#"
.search-input {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--panel-2);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 7px 10px;
}
.search-input .search-input-icon {
  font-size: 14px;
  line-height: 1;
  color: var(--gray-dim);
  flex: 0 0 auto;
}
.search-input input {
  flex: 1 1 auto;
  min-width: 0;
  border: 0;
  background: transparent;
  color: var(--text);
  font-family: var(--sans);
  font-size: 13px;
  line-height: 18px;
  padding: 0;
  outline: none;
}
.search-input input::placeholder { color: var(--gray-dim); }
.search-input:focus-within { border-color: var(--accent); }
.search-input-clear {
  flex: 0 0 auto;
  width: 20px;
  height: 20px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--gray-dim);
  cursor: pointer;
  padding: 0;
  line-height: 1;
}
.search-input-clear:hover { color: var(--text); background: var(--panel); }
.search-input-clear:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
.search-input-clear .ph { font-size: 12px; line-height: 1; }
:is(#sb, #home) .search-input input { background: transparent; border: 0; }
:is(#sb, #home) .search-input input:focus { border-color: transparent; box-shadow: none; outline: none; }

.search-input-demo { width: min(320px, 100%); }
"#;

pub fn input(model: &str, placeholder: &str) -> Node {
    m("div")
        .class("search-input")
        .kids(vec![
            m("i").class("ph ph-magnifying-glass search-input-icon"),
            m("input")
                .attr("type", "text")
                .attr("spellcheck", "false")
                .attr("autocomplete", "off")
                .attr("autocapitalize", "off")
                .attr("placeholder", placeholder)
                .attr("aria-label", "Search")
                .model(model)
                .on("input", &format!("{model} = $event.target.value")),
            m("button")
                .class("search-input-clear")
                .attr("type", "button")
                .attr("aria-label", "Clear search")
                .attr("title", "Clear search")
                .show(model)
                .on(
                    "click",
                    &format!(
                        "{model} = ''; $event.currentTarget.closest('.search-input').querySelector('input').focus()"
                    ),
                )
                .child(m("i").class("ph ph-x")),
        ])
}

pub fn demo() -> Node {
    use crate::components::demo::MODEL;
    m("div")
        .class("search-input-demo")
        .child(input(&format!("{MODEL}.search"), "Filter components…"))
}

pub fn register() {
    css::inject("search-input", CSS);
}

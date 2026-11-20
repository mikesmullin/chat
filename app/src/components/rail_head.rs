//! `rail head` — the title row of a sidebar section: the section's name in
//! proper case, and the one button its items come from.
//!
//! Every section in the left rail opens the same way — "Agents" with a `+`,
//! "Projects" with a `+` — so it is defined once here rather than once per
//! section. A section that cannot add anything (a group, a filter) passes
//! `None` and gets the title alone, in the same type.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.rail-head {
  display: flex; align-items: center; gap: 6px;
  margin-bottom: 10px;
}
/* The title is a label, not a heading: same size and weight as the rows
   beneath it, dimmed so the list reads first. */
.rail-title {
  margin: 0;
  font-size: 13px; font-weight: 400;
  color: var(--gray-dim);
}
.rail-add {
  margin-left: auto;
  width: 22px; height: 22px;
  display: inline-flex; align-items: center; justify-content: center;
  border: 0; border-radius: 6px;
  background: transparent; color: var(--gray-dim);
  cursor: pointer; font: inherit; line-height: 1;
}
.rail-add:hover { color: var(--accent); background: var(--panel-2); }

.rail-head-demo { display: flex; flex-direction: column; gap: 18px; width: 264px; }
"#;

/// `title` is what the section is called, written the way it is written in
/// prose. `add` is `(tooltip, click expression)`; `None` draws the title
/// alone. The click expression — not a closure — so the guide's button writes
/// to the guide's store and the app's writes to the session.
pub fn head(title: &str, add: Option<(&str, &str)>) -> Node {
    let mut kids = vec![m("p").class("rail-title").child(lit(title))];
    if let Some((tip, on)) = add {
        kids.push(
            m("button")
                .class("rail-add")
                .attr("type", "button")
                .attr("title", tip)
                .attr("aria-label", tip)
                .on("click", on)
                .child(Node::icon("plus")),
        );
    }
    m("div").class("rail-head").kids(kids)
}

/// standalone demo: the two sections that use it, plus a title with no
/// button. The buttons are inert — the guide has nothing to create.
pub fn demo() -> Node {
    m("div").class("rail-head-demo").kids(vec![
        head("Projects", Some(("New project", "$store.samples.noop()"))),
        head("Agents", Some(("Define a new agent", "$store.samples.noop()"))),
        head("Unfiled", None),
    ])
}

pub fn register() {
    css::inject("rail-head", CSS);
}

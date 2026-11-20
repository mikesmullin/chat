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
/* The button is neutral; the head is what puts it at the far edge of a title
   row. `presence` mounts the same button beside a cluster it changes, where
   pushing it to the far edge would detach it from what it acts on. */
.rail-head .rail-add { margin-left: auto; }
/* The title is a label, not a heading: same size and weight as the rows
   beneath it, dimmed so the list reads first. */
.rail-title {
  margin: 0;
  font-size: 13px; font-weight: 400;
  color: var(--gray-dim);
}
.rail-add {
  width: 22px; height: 22px;
  display: inline-flex; align-items: center; justify-content: center;
  border: 0; border-radius: 6px;
  background: transparent; color: var(--gray-dim);
  cursor: pointer; font: inherit; line-height: 1;
}
.rail-add:hover { color: var(--accent); background: var(--panel-2); }

.rail-head-demo { display: flex; flex-direction: column; gap: 18px; width: 264px; }
"#;

/// The add button on its own.
///
/// Split out of [`head`] because the same button is the one control beside a
/// presence cluster — "who is here, and the thing that changes it" — and a
/// second definition of a 22px `+` is the thing this repo exists to avoid.
/// `title` is what it is called, written the way it is written in prose.
/// The click is an **expression**, not a closure, so the guide's button writes
/// to the guide's store and the app's writes to the session.
fn button(icon: &str, tip: &str, on: &str) -> Node {
    m("button")
        .class("rail-add")
        .attr("type", "button")
        .attr("title", tip)
        .attr("aria-label", tip)
        .on("click", on)
        .child(Node::icon(icon))
}

pub fn add(tip: &str, on: &str) -> Node {
    button("plus", tip, on)
}

/// The same 22px ghost button with a pencil: the session header edit
/// control. It lives beside the presence cluster but is not part of the
/// indicator - it edits the session the cluster belongs to. Same brick,
/// different glyph, so the two never drift into two button styles.
pub fn edit(tip: &str, on: &str) -> Node {
    button("pencil-simple", tip, on)
}

/// `title` is what the section is called, written the way it is written in
/// prose. `add` is `(tooltip, click expression)`; `None` draws the title
/// alone.
pub fn head(title: &str, add: Option<(&str, &str)>) -> Node {
    let mut kids = vec![m("p").class("rail-title").child(lit(title))];
    if let Some((tip, on)) = add {
        kids.push(self::add(tip, on));
    }
    m("div").class("rail-head").kids(kids)
}

/// standalone demo: the two sections that use it, plus a title with no
/// button. The buttons are inert — the guide has nothing to create.
pub fn demo() -> Node {
    m("div").class("rail-head-demo").kids(vec![
        head("Projects", Some(("New project", "$store.samples.noop()"))),
        head("Agents", Some(("Define a new agent", "$store.samples.noop()"))),
        head("(ungrouped)", None),
    ])
}

pub fn register() {
    css::inject("rail-head", CSS);
}

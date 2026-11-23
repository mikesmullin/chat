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
/* A collapsible title is a button wearing the label's clothes: no chrome
   of its own, pointer on hover, and a caret that says which way the
   section goes. */
button.rail-title {
  background: transparent; border: 0; padding: 0;
  display: inline-flex; align-items: center; gap: 5px;
  /* Family only: size/weight/ink stay on `.rail-title`, whose lower
     specificity would otherwise lose to this button rule. */
  font-family: inherit; text-align: left;
  cursor: pointer;
}
button.rail-title:hover { color: var(--text); }
button.rail-title .ph { font-size: 11px; line-height: 1; }
button.rail-title:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; border-radius: 4px; }
/* Collapsed: the section keeps only its head — nothing trails it, so the
   head sheds its margin-bottom and the *container gap* alone decides the
   space to the next group. (An earlier version pulled the root up by a
   negative margin to stack heads flush — which dragged the next group up
   over this one and made every gap measurement a liar.) */
.project-tree.is-collapsed > .rail-head,
.agents-menu.is-collapsed > .rail-head,
.sandboxes-menu.is-collapsed > .rail-head { margin-bottom: 0; }
.rail-add {
  width: 22px; height: 22px;
  display: inline-flex; align-items: center; justify-content: center;
  border: 0; border-radius: 6px;
  background: transparent; color: var(--gray-dim);
  cursor: pointer; font: inherit; line-height: 1;
}
.rail-add:hover { color: var(--accent); background: var(--panel-2); }

.rail-head-demo { display: flex; flex-direction: column; gap: 18px; width: 264px; }

/* Narrow shell (see home.rs breakpoints): the rail is a 60px icon strip, so
   the section label goes and the caret toggle stays — a collapsed section
   with no visible toggle could never be expanded again. */
@media (max-width: 900px) {
  .rail-head .rail-title span { display: none; }
  .rail-head { justify-content: center; }
  .rail-head .rail-add { margin-left: 0; }
}
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
/// prose. `icon` is the section's phosphor glyph (`"folders"`, `"robot"`),
/// static because a section never changes what it is. `add` is `(tooltip,
/// click expression)`; `None` draws the title alone. `section` is
/// `Some("projects")` / `Some("agents")` when the title toggles its
/// section in the shell: the title becomes a button with a caret, bound to
/// `$store.home`, and the section body shows on the same flag. `None` keeps
/// the plain label for heads with nothing to collapse.
pub fn head(title: &str, icon: &str, add: Option<(&str, &str, &str)>, section: Option<&str>) -> Node {
    // The caret leads, tree-row style: it acts on what follows. Then the
    // section's own glyph, then the name.
    let title_node = match section {
        // The heading is the toggle: caret says which way the section goes,
        // `aria-expanded` says it to a reader. The `+` stays a sibling, so
        // its click never reaches this button and needs no `.stop`.
        Some(key) => m("button")
            .class("rail-title")
            .attr("type", "button")
            .bind("title", &format!("($store.home.{key}Open ? 'Collapse section' : 'Expand section')"))
            .bind("aria-expanded", &format!("($store.home.{key}Open ? 'true' : 'false')"))
            .on("click", &format!("$store.home.toggleSection('{key}')"))
            .kids(vec![
                Node::icon_dyn(&format!("($store.home.{key}Open ? 'caret-down' : 'caret-right')")),
                Node::icon(icon),
                m("span").child(lit(title)),
            ]),
        None => m("p")
            .class("rail-title")
            .kids(vec![Node::icon(icon), m("span").child(lit(title))]),
    };
    let mut kids = vec![title_node];
    if let Some((tip, on, guard)) = add {
        // The guard is a capability expression (PLAN44 Layer 1): a user who
        // may not create the resource does not see the button that would.
        let b = self::add(tip, on);
        kids.push(if guard.is_empty() { b } else { b.show(guard) });
    }
    m("div").class("rail-head").kids(kids)
}

/// standalone demo: the two sections that use it, plus a title with no
/// button. The buttons are inert — the guide has nothing to create — but
/// the section toggles are live, writing to the guide's own `home` store.
pub fn demo() -> Node {
    m("div").class("rail-head-demo").kids(vec![
        head("Projects", "folders", Some(("New project", "$store.samples.noop()", "")), Some("projects")),
        head("Agents", "robot", Some(("Define a new agent", "$store.samples.noop()", "")), Some("agents")),
        head("(ungrouped)", "tray", None, None),
    ])
}

pub fn register() {
    css::inject("rail-head", CSS);
}

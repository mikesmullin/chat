//! Left Sidebar Navigation — two forms:
//!   * `nav_rail_demo()` — the standalone, fixed-size rail shown on
//!     `/storybook/nav-rail` (a 264×460 panel in the content area, like the
//!     original storybook's `.nav-demo`).
//!   * `page_rail(active)` — the live left sidebar used by every storybook
//!     page; its items link to the other storybook routes.
//!
//! Both share the `.sb-rail` / `.sb-rail-head` / `.kicker` / `.nav-list` /
//! `.nav-item` classes so the CSS is the contract. Both carry a top-right
//! layout toggle (`.rail-toggle`); clicking it collapses to a 56px icon column
//! and back (live rail collapses `#sb`'s grid track; demo collapses its width).
//!
//! Owns its CSS; injected at boot by `components::register`.

use rustex1_kit::h::{lit, m, Node};

/// the fixed catalog of storybook components, depth-first (0 = atomic, most
/// composite last); ties break alphabetically. The count shown in the rail
/// is the number of distinct library components the item is built from.
const ENTRIES: [(&'static str, &'static str, &'static str); 51] = [
    ("Approval Card", "/storybook/approval-card", "0"),
    ("Assistant Card", "/storybook/assistant-card", "0"),
    ("Automation Form", "/storybook/automation-form", "1"),
    ("Automations Menu", "/storybook/automations", "1"),
    ("Counter", "/storybook/counter", "0"),
    ("Dropdown", "/storybook/dropdown", "0"),
    ("Decision Card", "/storybook/decision-card", "0"),
    ("Error Card", "/storybook/error-card", "0"),
    ("Fold Card", "/storybook/fold-card", "0"),
    ("Hint Text", "/storybook/hint", "0"),
    ("History Tree", "/storybook/history-tree", "0"),
    ("Label", "/storybook/label", "0"),
    ("Left Sidebar Navigation", "/storybook/nav-rail", "0"),
    ("Mode Toggle", "/storybook/mode-toggle", "0"),
    ("Notification Toggle", "/storybook/notification-toggle", "0"),
    ("Perf Badge", "/storybook/perf-badge", "0"),
    ("Pie Ring Graph", "/storybook/pie-ring", "0"),
    ("Prompt Card", "/storybook/prompt-card", "0"),
    ("Rail Head", "/storybook/rail-head", "0"),
    ("Reasoning Card", "/storybook/reasoning-card", "0"),
    ("Relative Timestamp", "/storybook/reltime", "0"),
    ("Settings Menu Item", "/storybook", "0"),
    ("System Card", "/storybook/system-card", "0"),
    ("Task List", "/storybook/task-list", "0"),
    ("Thinking", "/storybook/thinking", "0"),
    ("Tool Call Card", "/storybook/tool-call-card", "0"),
    ("Tool Response Card", "/storybook/tool-response-card", "0"),
    ("User Card", "/storybook/user-card", "0"),
    ("Validation Info", "/storybook/validation", "0"),
    ("Picker", "/storybook/picker", "1"),
    ("Radio Input", "/storybook/radio", "1"),
    ("Range Input", "/storybook/range", "1"),
    ("Sandboxes Menu", "/storybook/sandboxes", "1"),
    ("Session State", "/storybook/session-state", "1"),
    ("Tag Input", "/storybook/tags", "1"),
    ("Target Form", "/storybook/target-form", "1"),
    ("Text Area", "/storybook/text-area", "1"),
    ("Text Input", "/storybook/text-input", "1"),
    ("Toggle Button", "/storybook/toggle", "1"),
    ("Composer", "/storybook/composer", "2"),
    ("Presence", "/storybook/presence", "2"),
    ("Tab", "/storybook/tab", "2"),
    ("Field", "/storybook/field", "3"),
    ("Item Group", "/storybook/item-group", "3"),
    ("Project Tree", "/storybook/project-tree", "3"),
    ("Session Editor", "/storybook/session-editor", "3"),
    ("Tab Bar", "/storybook/tab-bar", "3"),
    ("Agents Menu", "/storybook/agents", "5"),
    ("Session", "/storybook/session", "5"),
    ("Session Panel", "/storybook/session-panel", "9"),
    ("New Automation", "/storybook/automation", "10"),
];

/// The layout toggle. Fires `toggleRail($event)` — a method each storybook
/// route defines on its own scope (the idiom m-js itself uses: its 404
/// template binds `@click="goHome"` to a method on the route object). No
/// `$el`, no inline IIFE: the method receives the real event and reads
/// `currentTarget`, so there is nothing for the template compiler to
/// mis-compile. The button is a direct child of the rail so it stays
/// visible when the collapsed state hides the head/group/list. Live rail ->
/// toggle `collapsed` on `#sb` (grid) and `.sb-rail`; demo -> toggle on
/// `.nav-demo`.

pub const CSS: &str = r#"
.sb-rail {
  /* no surface of its own — the rail reads as part of the black page,
     the way the reference app's sidebar does. */
  background: transparent;
  border-right: 0;
  /* breathing room between the rail's scrollbar and the content card —
     the scrollbar sits on .nav-list's own right edge, so without this
     it rides flush against the column. Matches the card's 10px inset. */
  margin-right: 10px;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
  position: relative;
}
.sb-rail-head { padding: 24px 20px 14px; }
.sb-rail-head h1 { font-size: 15px; font-weight: 700; letter-spacing: -.015em; line-height: 1.2; color: var(--text); }
.rail-group { margin-top: 18px; padding: 4px 20px 10px; }
.kicker {
  font-family: var(--mono); font-size: 10.5px; letter-spacing: .16em;
  text-transform: uppercase; color: var(--muted);
}
.nav-list {
  list-style: none; padding: 6px 12px 40px;
  display: flex; flex-direction: column; gap: 0;
  min-height: 0; overflow-y: auto;
}
.nav-item {
  display: flex; align-items: stretch;
  width: 100%; text-align: left; padding: 0;
  border: none;
  background: transparent; font: inherit; font-size: 15px; color: var(--muted);
  cursor: pointer; transition: color .13s, background .13s;
}
.nav-item .counter { font-size: 11px; }
/* the row's link IS the full item rect (padding + indicator border live
   here, not on the li) so every pixel navigates — no dead zones.
   Inherits the item's state colour (incl. the active indigo). */
.nav-item a {
  flex: 1 1 auto; min-width: 0;
  display: flex; align-items: center; gap: 11px; justify-content: space-between;
  padding: 9px 12px;
  border: none;
  border-left: 4px solid transparent;
  color: inherit; text-decoration: none;
  transition: border-color .13s;
}
.nav-item a:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.nav-item .ph { color: inherit; font-size: 15px; line-height: 1; flex: 0 0 auto; transition: color .13s; }
.nav-item span { overflow: hidden; white-space: nowrap; }
.nav-item:hover { color: var(--text); background: var(--accent-glow); }
.nav-item.on { color: var(--accent-soft); background: var(--accent-tint); }
.nav-item.on a { border-left-color: var(--accent); }
.nav-item.on .ph { color: var(--accent-soft); }

/* standalone showcase rail */
.nav-demo {
  width: 264px; height: 460px;
  /* raised: this one is a card sitting ON the content column, so it takes
     --panel-2 (the column itself is --panel). */
  background: var(--panel-2);
  border: 1px solid var(--border);
  border-radius: 14px;
  display: flex; flex-direction: column; overflow: hidden;
  transition: width .3s ease-in-out, border-radius .3s ease-in-out;
  position: relative;
}
/* collapsed = width shrinks toward the left edge; the icon stays
   centred in the 56px column so it remains clickable. */
.nav-demo.collapsed { width: 56px; }
.nav-demo.collapsed .sb-rail-head,
.nav-demo.collapsed .rail-group,
.nav-demo.collapsed .nav-list { display: none; }
.nav-demo .nav-list { padding-bottom: 8px; }
.nav-demo .nav-item:hover { background: var(--accent-glow); }

/* top-right open/close toggle — bright, gray, layout glyph. Bare
   button like the settings icon: transparent, hover = colour shift, no
   box chrome. */
.rail-toggle {
  position: absolute; top: 14px; right: 14px; z-index: 1;
  width: 30px; height: 30px; display: flex;
  align-items: center; justify-content: center;
  background: transparent; border: 0; border-radius: 0;
  cursor: pointer;
  padding: 0;
}
.rail-toggle .ph {
  font-size: 17px; line-height: 1;
  color: var(--text);
  transition: transform .3s ease-in-out, color .13s;
}
.rail-toggle:hover .ph { color: var(--accent-soft); }
.rail-toggle:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
/* when collapsed, the icon points the other way (slide back) */
.nav-demo.collapsed .rail-toggle .ph { transform: scaleX(-1); }
.sb-rail.collapsed .rail-toggle .ph { transform: scaleX(-1); }
/* live rail collapsed: hide the text content so the 56px grid track shows
   only the toggle, centred. (#sb animates the track width; the rail clips.) */
.sb-rail.collapsed .sb-rail-head,
.sb-rail.collapsed .rail-group,
.sb-rail.collapsed .nav-list { display: none; }
.sb-rail.collapsed .rail-toggle { right: 13px; }
"#;

fn toggle_button() -> Node {
    m("button")
        .class("rail-toggle")
        .attr("type", "button")
        .attr("aria-label", "Toggle sidebar")
        .attr("aria-expanded", "true")
        .on("click", "toggleRail($event)")
        .child(Node::icon("sidebar-simple"))
}

fn rail(active: Option<&str>, demo: bool) -> Node {
    m(if demo { "div" } else { "aside" })
        .class(if demo { "nav-demo" } else { "sb-rail" })
        .attr("role", if demo { "figure" } else { "navigation" })
        .attr("aria-label", "Design Guide")
        .kids(vec![
            // toggle is a direct child (not in .sb-rail-head) so it stays visible
            // when the collapsed state hides the head/group/list.
            toggle_button(),
            m("header")
                .class("sb-rail-head")
                .child(m("h1").child(lit("Design Guide"))),
            m("div").class("rail-group").child(m("span").class("kicker").child(lit("Components"))),
            m("ul")
                .class("nav-list")
                .kids(
                    ENTRIES
                        .iter()
                        .map(|(label, url, depth)| {
                            let is_active = active == Some(label);
                            row(label, url, depth, is_active)
                        })
                        .collect(),
                ),
        ])
}

fn row(label: &str, url: &str, depth: &str, active: bool) -> Node {
    // the whole row is one link (label + depth count) — no dead zones:
    // clicks anywhere on the row navigate (client-side via the SPA
    // delegation), and the full area keeps native link behaviour
    // (middle-click, right-click, hover status).
    m("li")
        .class(&format!("nav-item{}", if active { " on" } else { "" }))
        .child(
            m("a")
                .attr("href", url)
                .kids(vec![
                    m("span").child(lit(label)),
                    m("span").class("counter").child(lit(depth)),
                ]),
        )
}

/// the standalone 264×460 rail — the `/storybook/nav-rail` component.
pub fn nav_rail_demo() -> Node {
    rail(Some("Settings Menu Item"), true)
}

/// the live left sidebar on every storybook page (`active` = current label).
pub fn page_rail(active: &str) -> Node {
    rail(Some(active), false)
}

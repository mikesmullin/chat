use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue};

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::{self, Def};
use rustex1_kit::js;

use crate::components::search_input;
use crate::css;

const ENTRIES: [(&'static str, &'static str, &'static str); 84] = [
    ("Approval Card", "/storybook/approval-card", "0"),
    ("Assistant Card", "/storybook/assistant-card", "0"),
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
    ("Search Input", "/storybook/search-input", "0"),
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
    ("Switch", "/storybook/switch", "1"),
    ("Browser Pane", "/storybook/browser-pane", "2"),
    ("Composer", "/storybook/composer", "2"),
    ("Presence", "/storybook/presence", "2"),
    ("Tab", "/storybook/tab", "2"),
    ("Field", "/storybook/field", "3"),
    ("Item Group", "/storybook/item-group", "3"),
    ("Project Tree", "/storybook/project-tree", "3"),
    ("Session Editor", "/storybook/session-editor", "3"),
    ("Group Editor", "/storybook/group-editor", "3"),
    ("Tab Bar", "/storybook/tab-bar", "3"),
    ("Agents Menu", "/storybook/agents", "5"),
    ("Session", "/storybook/session", "5"),
    ("Session Panel", "/storybook/session-panel", "9"),
    ("PR State Badge", "/storybook/pr-state-badge", "1"),
    ("PR State Pill", "/storybook/pr-state-pill", "1"),
    ("PR Avatar", "/storybook/pr-avatar", "1"),
    ("Pull Request Card", "/storybook/pr-card", "1"),
    ("PR Thread", "/storybook/pr-thread", "1"),
    ("Review Form", "/storybook/review-form", "1"),
    ("Review Changes", "/storybook/review-changes", "1"),
    ("Diff Card", "/storybook/diff-card", "2"),
    ("Diff View", "/storybook/diff-view", "2"),
    ("Markdown Diff Preview", "/storybook/md-diff", "2"),
    ("Merge Panel", "/storybook/merge-panel", "2"),
    ("Repo Card", "/storybook/repo-card", "1"),
    ("Repos Menu", "/storybook/repos-menu", "3"),
    ("Worktrees Menu", "/storybook/worktrees-menu", "3"),
    ("Pull Requests Menu", "/storybook/prs-menu", "3"),
    ("Job Card", "/storybook/job-card", "1"),
    ("Job Row", "/storybook/job-row", "1"),
    ("Jobs Menu", "/storybook/jobs-menu", "3"),
    ("Pipeline Graph", "/storybook/pipeline-graph", "2"),
    ("Pipelines Menu", "/storybook/pipelines-menu", "3"),
    ("Stage Timeline", "/storybook/stage-timeline", "2"),
    ("Journal Tail", "/storybook/journal-tail", "2"),
    ("Run State Badge", "/storybook/run-state-badge", "1"),
    ("Job Run Chip", "/storybook/job-run-chip", "1"),
    ("Sandbox Chip", "/storybook/sandbox-chip", "1"),
    ("Input Prompt", "/storybook/input-prompt", "2"),
    ("Repo Panel", "/storybook/repo-panel", "2"),
    ("PR Panel", "/storybook/pr-panel", "2"),
    ("Run Panel", "/storybook/run-panel", "2"),
    ("Job Panel", "/storybook/job-panel", "2"),
    ("Duration", "/storybook/duration", "0"),
    ("Tooltip", "/storybook/tooltip", "0"),
];


pub const CSS: &str = r#"
.sb-rail {
  background: transparent;
  border-right: 0;
  margin-right: 10px;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
  position: relative;
}
.sb-rail-head { padding: 14px 20px 10px; }
.sb-rail-head h1 { margin: 0; font-size: 15px; font-weight: 700; letter-spacing: -.015em; line-height: 30px; color: var(--text); }
.rail-group {
  margin-top: 14px; padding: 4px 20px 10px;
  display: flex; align-items: center; justify-content: space-between;
}
.rail-sort { display: flex; gap: 4px; align-items: center; }
.rail-sort-btn {
  width: 28px; height: 28px; display: inline-flex;
  align-items: center; justify-content: center;
  border: 0; border-radius: 7px;
  background: transparent; color: var(--gray-dim);
  cursor: pointer; padding: 0; line-height: 1;
  transition: color .13s, background .13s;
}
.rail-sort-btn:hover { color: var(--text); background: var(--accent-glow); }
.rail-sort-btn.on { color: var(--accent-soft); background: var(--accent-tint); }
.rail-sort-btn:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.rail-sort-btn .ph { font-size: 15px; line-height: 1; }
.kicker {
  font-family: var(--mono); font-size: 10.5px; letter-spacing: .16em;
  text-transform: uppercase; color: var(--muted);
}
.rail-search { padding: 2px 12px 14px; flex: 0 0 auto; }
.nav-empty {
  padding: 10px 12px;
  color: var(--gray-dim);
  font-size: 12px;
  list-style: none;
}
.nav-list {
  margin: 0;
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

.nav-demo {
  width: 264px; height: 460px;
  background: var(--panel-2);
  border: 1px solid var(--border);
  border-radius: 14px;
  display: flex; flex-direction: column; overflow: hidden;
  transition: width .3s ease-in-out, border-radius .3s ease-in-out;
  position: relative;
}
.nav-demo.collapsed { width: 56px; }
.nav-demo.collapsed .sb-rail-head,
.nav-demo.collapsed .rail-group,
.nav-demo.collapsed .rail-search,
.nav-demo.collapsed .nav-list { display: none; }
.nav-demo .nav-list { padding-bottom: 8px; }
.nav-demo .nav-item:hover { background: var(--accent-glow); }

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
.nav-demo.collapsed .rail-toggle .ph { transform: scaleX(-1); }
.sb-rail.collapsed .rail-toggle .ph { transform: scaleX(-1); }
.sb-rail.collapsed .sb-rail-head,
.sb-rail.collapsed .rail-group,
.sb-rail.collapsed .rail-search,
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

const STORE: &str = "navrail";
const EXPR: &str = "$store.navrail";

fn entries_js() -> JsValue {
    let arr = Array::new();
    for (label, url, depth) in ENTRIES {
        let o = js::obj();
        js::set_str(&o, "label", label);
        js::set_str(&o, "url", url);
        js::set_str(&o, "depth", depth);
        arr.push(&o);
    }
    arr.into()
}

fn match_entries(_this: JsValue, args: js_sys::Array) -> JsValue {
    let list = args.get(0);
    let out = Array::new();
    let Ok(arr) = list.dyn_into::<Array>() else {
        return out.into();
    };
    let store = mjs::store_ref(STORE);
    let query = js::get_str(&store, "query").trim().to_lowercase();
    let sort = js::get_str(&store, "sort");
    let mut rows: Vec<JsValue> = arr
        .iter()
        .filter(|item| {
            let label = js::get_str(item, "label").to_lowercase();
            query.is_empty() || label.contains(&query)
        })
        .collect();
    rows.sort_by(|a, b| {
        let an = js::get_str(a, "label").to_lowercase();
        let bn = js::get_str(b, "label").to_lowercase();
        if sort == "alpha" {
            an.cmp(&bn)
        } else {
            let ac = js::get_str(a, "depth").parse::<u32>().unwrap_or(0);
            let bc = js::get_str(b, "depth").parse::<u32>().unwrap_or(0);
            ac.cmp(&bc).then_with(|| an.cmp(&bn))
        }
    });
    for item in rows {
        out.push(&item);
    }
    out.into()
}

fn rail(active: Option<&str>, demo: bool) -> Node {
    let active = active.unwrap_or("");
    let matches = format!("{EXPR}.match({EXPR}.entries)");
    m(if demo { "div" } else { "aside" })
        .class(if demo { "nav-demo" } else { "sb-rail" })
        .attr("role", if demo { "figure" } else { "navigation" })
        .attr("aria-label", "Design Guide")
        .kids(vec![
            toggle_button(),
            m("header")
                .class("sb-rail-head")
                .child(m("h1").child(lit("Design Guide"))),
            m("div")
                .class("rail-group")
                .kids(vec![
                    m("span").class("kicker").child(lit("Components")),
                    m("div")
                        .class("rail-sort")
                        .kids(vec![
                            m("button")
                                .bind("class", &format!("{EXPR}.sort === 'alpha' ? 'rail-sort-btn on' : 'rail-sort-btn'"))
                                .attr("type", "button")
                                .attr("title", "Sort by A–Z")
                                .attr("aria-label", "Sort by A–Z")
                                .on("click", &format!("{EXPR}.sort = 'alpha'"))
                                .child(Node::icon("text-aa")),
                            m("button")
                                .bind("class", &format!("{EXPR}.sort === 'count' ? 'rail-sort-btn on' : 'rail-sort-btn'"))
                                .attr("type", "button")
                                .attr("title", "Sort by component count")
                                .attr("aria-label", "Sort by component count")
                                .on("click", &format!("{EXPR}.sort = 'count'"))
                                .child(Node::icon("list-numbers")),
                        ]),
                ]),
            m("div")
                .class("rail-search")
                .child(search_input::input(&format!("{EXPR}.query"), "Filter components…")),
            m("ul")
                .class("nav-list")
                .kids(vec![
                    m("li")
                        .bind("class", &format!("e.label === '{active}' ? 'nav-item on' : 'nav-item'"))
                        .for_each(&format!("e in {matches}"))
                        .key("e.url")
                        .child(
                            m("a")
                                .bind("href", "e.url")
                                .kids(vec![
                                    m("span").text("e.label"),
                                    m("span").class("counter").text("e.depth"),
                                ]),
                        ),
                    m("li")
                        .class("nav-empty")
                        .show(&format!("{matches}.length === 0"))
                        .child(lit("No matches")),
                ]),
        ])
}

pub fn nav_rail_demo() -> Node {
    rail(Some("Settings Menu Item"), true)
}

pub fn page_rail(active: &str) -> Node {
    rail(Some(active), false)
}

pub fn register() {
    css::inject("nav-rail", CSS);
    let mut d = Def::new().named("store navrail");
    d.val("entries", &entries_js())
        .str("query", "")
        .str("sort", "count")
        .method("match", match_entries);
    mjs::store(STORE, &d.finish());
}

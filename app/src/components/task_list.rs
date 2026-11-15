//! `task list` — the composer's todo panel: a progress header plus one
//! row per task (status icon + text). Pure function of the list: progress
//! is done/total, current is the first unfinished task. Statuses and icons
//! mirror the app (`completed` check-circle, `in_progress` spinning
//! circle-notch, anything else plain circle). Verbatim app classes.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
/* ---- the panel itself -------------------------------------------
   These rules came out of `web/styles.css`, where they were the only
   thing dressing this one component — a look kept in a global sheet
   is a look that outlives the markup it was written for. The
   selectors are copied unchanged, still scoped under `.mjs-chat`,
   because the panel is drawn inside the composer's envelope; nothing
   about specificity or the cascade moved with them. */
.mjs-chat .todo-panel {
  border: 1px solid var(--mjs-line);
  border-radius: 10px;
  background: var(--mjs-bg);
  overflow: hidden;
}
.mjs-chat .todo-header {
  appearance: none;
  width: 100%;
  border: 0;
  background: transparent;
  color: var(--mjs-ink);
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  cursor: pointer;
  font: 600 12px/1.3 var(--mjs-sans);
  text-align: left;
}
.mjs-chat .todo-header:hover { background: color-mix(in srgb, var(--mjs-accent) 8%, transparent); }
.mjs-chat .todo-header i { color: var(--mjs-dim); font-size: 13px; flex: 0 0 auto; }
.mjs-chat .todo-progress {
  flex: 0 0 auto;
  color: var(--mjs-dim);
  font: 600 11px/1 var(--mjs-mono);
}
.mjs-chat .todo-current {
  min-width: 0;
  flex: 1 1 auto;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 600;
}
.mjs-chat .todo-list {
  list-style: none;
  margin: 0;
  padding: 0 10px 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 168px;
  overflow: auto;
}
.mjs-chat .todo-list li {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  font-size: 12px;
  line-height: 1.35;
  color: var(--mjs-ink);
}
.mjs-chat .todo-list li i { flex: 0 0 auto; margin-top: 1px; color: var(--mjs-dim); }
.mjs-chat .todo-list li.is-completed {
  color: var(--mjs-dim);
  text-decoration: line-through;
}
.mjs-chat .todo-list li.is-completed i { color: var(--mjs-ok); }
.mjs-chat .todo-list li.is-in_progress { color: var(--mjs-accent-2); }
.mjs-chat .todo-list li.is-in_progress i { color: var(--mjs-accent); }
.mjs-chat .todo-list li.is-in_progress .ph-circle-notch {
  animation: mjs-todo-spin 1s linear infinite;
}
@keyframes mjs-todo-spin { to { transform: rotate(360deg); } }
.mjs-chat .todo-text { min-width: 0; }

/* demo framing: the panel alone */
.task-list-demo { width: min(400px, 100%); }
.task-list-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub struct Task {
    pub status: &'static str,
    pub content: &'static str,
}

pub const SAMPLES: &[Task] = &[
    Task { status: "completed", content: "Map current endpoints" },
    Task { status: "in_progress", content: "Adding the compat shim" },
    Task { status: "pending", content: "Write migration notes" },
];

fn icon_for(status: &str) -> &'static str {
    match status {
        "completed" => "check-circle",
        "in_progress" => "circle-notch",
        _ => "circle",
    }
}

fn row(task: &Task) -> Node {
    m("li")
        .class(&format!("is-{}", task.status))
        .kids(vec![
            Node::icon(icon_for(task.status)),
            m("span").class("todo-text").child(lit(task.content)),
        ])
}

/// the panel for any task slice.
pub fn task_list(tasks: &[Task]) -> Node {
    let done = tasks.iter().filter(|t| t.status == "completed").count();
    let mut header = vec![
        m("span")
            .class("todo-progress")
            .child(lit(&format!("{done}/{}", tasks.len()))),
    ];
    if let Some(current) = tasks.iter().find(|t| t.status != "completed") {
        header.push(
            m("span")
                .class("todo-current")
                .child(lit(current.content)),
        );
    }
    m("div").class("todo-panel").kids(vec![
        m("div").class("todo-header").kids(header),
        m("ul")
            .class("todo-list")
            .kids(tasks.iter().map(row).collect()),
    ])
}

/// standalone demo: one of each status.
pub fn demo() -> Node {
    m("div").class("task-list-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(task_list(SAMPLES)),
    )
}

pub fn register() {
    css::inject("task-list", CSS);
}

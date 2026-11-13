//! `task list` — the composer's todo panel: a progress header plus one
//! row per task (status icon + text). Pure function of the list: progress
//! is done/total, current is the first unfinished task. Statuses and icons
//! mirror the app (`completed` check-circle, `in_progress` spinning
//! circle-notch, anything else plain circle). Verbatim app classes.

use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
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

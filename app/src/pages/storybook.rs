use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs::{self, Def};
use rustex1_kit::{js, lifecycle, Runtime};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use crate::components::{
    agents_menu, approval_card, assistant_card, browser_pane, composer, decision_card,
    samples, mode_toggle,
    dropdown, error_card, fold_card, identity_switcher, picker, project_tree, prompt_card, queued_card, rail_head,
    share_dialog,
    target_form,
    diff_card, diff_view, field, hint_text, history_tree, input_prompt, item_group, job_card, job_panel, job_row,
    job_run_chip, jobs_menu, journal_tail, label, md_diff_preview, merge_panel, nav_rail, notification_toggle,
    perf_badge, pipeline_graph, pipelines_menu, pr_avatar, pr_card, pr_panel, pr_state_badge, pr_state_pill,
    pr_thread, prs_menu, radio_input, range_input, repo_card, repo_panel, repos_menu, review_changes, review_form,
    presence, profile_panel, reasoning_card, relative_timestamp, sandboxes_menu, session, session_editor, session_panel,
    session_state,
    run_panel, run_state_badge, sandbox_chip_state, stage_timeline, system_card, tab, tag_input, task_list, text_area,
    text_input, thinking, toggle_button, tool_call_card, tool_response_card, ui_duration, ui_tooltip, user_card,
    validation_info, worktrees_menu,
};

fn shell(active: &str, content: Node) -> Node {
    m("div")
        .attr("id", "sb")
        .class("sb")
        .kids(vec![
            nav_rail::page_rail(active),
            m("main")
                .class("sb-content")
                .child(m("section").class("feature").child(content)),
            crate::components::lightbox::lightbox(),
        ])
}

fn route_def(name: &str, active: &str, content: Node) -> JsValue {
    let mut d = Def::new().named(name);
    d.tree(shell(active, content));
    d.method("toggleRail", toggle_rail);
    d.finish()
}

fn toggle_rail(_this: JsValue, args: js_sys::Array) -> JsValue {
    let ev = js::arg(&args, 0);
    let btn = js::get(&ev, "currentTarget");
    if js::is_missing(&btn) {
        return JsValue::UNDEFINED;
    }
    let rail = js::call(
        &btn,
        "closest",
        &[JsValue::from_str(".sb-rail,.nav-demo")],
    );
    if js::is_missing(&rail) {
        return JsValue::UNDEFINED;
    }
    let cls = js::get(&rail, "classList");
    let toggle = |el_cls: &JsValue| {
        js::call(el_cls, "toggle", &[JsValue::from_str("collapsed")]);
    };
    let is_collapsed = |el_cls: &JsValue| {
        js::call(el_cls, "contains", &[JsValue::from_str("collapsed")])
            .as_bool()
            .unwrap_or(false)
    };
    let set_expanded = |collapsed: bool| {
        js::call(
            &btn,
            "setAttribute",
            &[
                JsValue::from_str("aria-expanded"),
                JsValue::from_str(if collapsed { "false" } else { "true" }),
            ],
        );
    };
    let is_live = js::call(&cls, "contains", &[JsValue::from_str("sb-rail")])
        .as_bool()
        .unwrap_or(false);
    if is_live {
        if let Some(page) = js::query("#sb") {
            let page_cls = js::get(&page, "classList");
            toggle(&page_cls);
            set_expanded(is_collapsed(&page_cls));
        }
        toggle(&cls);
    } else {
        toggle(&cls);
        set_expanded(is_collapsed(&cls));
    }
    JsValue::UNDEFINED
}

pub fn register(rt: &mut Runtime) {
    register_spa_nav();
    rt.route("/storybook", "Settings Menu Item", |_| {
        route_def(
            "route /storybook",
            "Settings Menu Item",
            Node::host("uiSettingsMenuItem", &[]),
        )
    });

    rt.route("/storybook/counter", "Counter", |_| {
        let content = m("div")
            .class("counter-demo")
            .kids(vec![
                Node::host("uiCounter", &[]).bind("value", "1"),
                Node::host("uiCounter", &[]).bind("value", "3"),
                Node::host("uiCounter", &[]).bind("value", "24"),
                Node::host("uiCounter", &[]).bind("value", "128"),
            ]);
        route_def("route /storybook/counter", "Counter", content)
    });

    rt.route("/storybook/browser-pane", "Browser Pane", |_| {
        route_def("route /storybook/browser-pane", "Browser Pane", browser_pane::demo())
    });

    rt.route("/storybook/nav-rail", "Left Sidebar Navigation", |_| {
        route_def(
            "route /storybook/nav-rail",
            "Left Sidebar Navigation",
            nav_rail::nav_rail_demo(),
        )
    });

    rt.route("/storybook/perf-badge", "Perf Badge", |_| {
        route_def("route /storybook/perf-badge", "Perf Badge", perf_badge::badge())
    });

    rt.route("/storybook/pie-ring", "Pie Ring Graph", |_| {
        let ring = m("div")
            .class("demo-ring")
            .child(
                Node::host("uiPieRing", &[("color", "var(--accent)")])
                    .bind("fraction", "0.68"),
            );
        route_def("route /storybook/pie-ring", "Pie Ring Graph", ring)
    });

    rt.route("/storybook/reltime", "Relative Timestamp", |_| {
        route_def(
            "route /storybook/reltime",
            "Relative Timestamp",
            relative_timestamp::demo(),
        )
    });

    rt.route("/storybook/item-group", "Item Group", |_| {
        let group = item_group::item_group(
            "done",
            "Done",
            "2",
            &[
                (
                    "Eve".into(),
                    "Finished the onboarding copy and tone guide".into(),
                    "1h".into(),
                ),
                ("Dan".into(), "Fixed the session timeout bug".into(), "45m".into()),
            ],
        );
        route_def("route /storybook/item-group", "Item Group", group)
    });

    rt.route("/storybook/agents", "Agents Menu", |_| {
        route_def(
            "route /storybook/agents",
            "Agents Menu",
            agents_menu::agents_menu(),
        )
    });

    rt.route("/storybook/sandboxes", "Sandboxes Menu", |_| {
        route_def(
            "route /storybook/sandboxes",
            "Sandboxes Menu",
            sandboxes_menu::sandboxes_menu(samples::EXPR),
        )
    });

    rt.route("/storybook/field", "Field", |_| {
        route_def("route /storybook/field", "Field", field::demo())
    });

    rt.route("/storybook/label", "Label", |_| {
        route_def("route /storybook/label", "Label", label::demo())
    });

    rt.route("/storybook/hint", "Hint Text", |_| {
        route_def("route /storybook/hint", "Hint Text", hint_text::demo())
    });

    rt.route("/storybook/text-input", "Text Input", |_| {
        route_def(
            "route /storybook/text-input",
            "Text Input",
            text_input::demo(),
        )
    });

    rt.route("/storybook/toggle", "Toggle Button", |_| {
        route_def(
            "route /storybook/toggle",
            "Toggle Button",
            toggle_button::demo(),
        )
    });

    rt.route("/storybook/range", "Range Input", |_| {
        route_def("route /storybook/range", "Range Input", range_input::demo())
    });

    rt.route("/storybook/radio", "Radio Input", |_| {
        route_def("route /storybook/radio", "Radio Input", radio_input::demo())
    });

    rt.route("/storybook/text-area", "Text Area", |_| {
        route_def("route /storybook/text-area", "Text Area", text_area::demo())
    });

    rt.route("/storybook/tags", "Tag Input", |_| {
        route_def("route /storybook/tags", "Tag Input", tag_input::demo())
    });

    rt.route("/storybook/validation", "Validation Info", |_| {
        route_def(
            "route /storybook/validation",
            "Validation Info",
            validation_info::demo(),
        )
    });

    rt.route("/storybook/session", "Session", |_| {
        route_def("route /storybook/session", "Session", session::demo())
    });
    rt.route("/storybook/session-panel", "Session Panel", |_| {
        route_def(
            "route /storybook/session-panel",
            "Session Panel",
            session_panel::demo(),
        )
    });

    rt.route("/storybook/fold-card", "Fold Card", |_| {
        route_def("route /storybook/fold-card", "Fold Card", fold_card::demo())
    });
    rt.route("/storybook/prompt-card", "Prompt Card", |_| {
        route_def("route /storybook/prompt-card", "Prompt Card", prompt_card::demo())
    });
    rt.route("/storybook/decision-card", "Decision Card", |_| {
        route_def("route /storybook/decision-card", "Decision Card", decision_card::demo())
    });

    rt.route("/storybook/composer", "Composer", |_| {
        route_def("route /storybook/composer", "Composer", composer::demo())
    });

    rt.route("/storybook/dropdown", "Dropdown", |_| {
        route_def("route /storybook/dropdown", "Dropdown", dropdown::demo())
    });

    rt.route("/storybook/picker", "Picker", |_| {
        route_def("route /storybook/picker", "Picker", picker::demo())
    });

    rt.route("/storybook/rail-head", "Rail Head", |_| {
        route_def("route /storybook/rail-head", "Rail Head", rail_head::demo())
    });

    rt.route("/storybook/project-tree", "Project Tree", |_| {
        route_def("route /storybook/project-tree", "Project Tree", project_tree::demo())
    });

    rt.route("/storybook/mode-toggle", "Mode Toggle", |_| {
        route_def("route /storybook/mode-toggle", "Mode Toggle", mode_toggle::button())
    });

    rt.route("/storybook/history-tree", "History Tree", |_| {
        route_def("route /storybook/history-tree", "History Tree", history_tree::demo())
    });

    rt.route("/storybook/target-form", "Target Form", |_| {
        route_def("route /storybook/target-form", "Target Form", target_form::demo())
    });

    rt.route("/storybook/automation-form", "Automation Form", |_| {
        route_def("route /storybook/automation-form", "Automation Form", target_form::demo_automation())
    });

    rt.route("/storybook/pr-card", "Pull Request Card", |_| {
        route_def("route /storybook/pr-card", "Pull Request Card", pr_card::demo())
    });
    rt.route("/storybook/pr-thread", "PR Thread", |_| {
        route_def("route /storybook/pr-thread", "PR Thread", pr_thread::demo())
    });
    rt.route("/storybook/review-form", "Review Form", |_| {
        route_def("route /storybook/review-form", "Review Form", review_form::demo())
    });
    rt.route("/storybook/review-changes", "Review Changes", |_| {
        route_def("route /storybook/review-changes", "Review Changes", review_changes::demo())
    });
    rt.route("/storybook/diff-card", "Diff Card", |_| {
        route_def("route /storybook/diff-card", "Diff Card", diff_card::demo())
    });
    rt.route("/storybook/diff-view", "Diff View", |_| {
        route_def("route /storybook/diff-view", "Diff View", diff_view::demo())
    });
    rt.route("/storybook/md-diff", "Markdown Diff Preview", |_| {
        route_def("route /storybook/md-diff", "Markdown Diff Preview", md_diff_preview::demo())
    });
    rt.route("/storybook/merge-panel", "Merge Panel", |_| {
        route_def("route /storybook/merge-panel", "Merge Panel", merge_panel::demo())
    });
    rt.route("/storybook/pr-state-badge", "PR State Badge", |_| {
        route_def("route /storybook/pr-state-badge", "PR State Badge", pr_state_badge::demo())
    });
    rt.route("/storybook/pr-state-pill", "PR State Pill", |_| {
        route_def("route /storybook/pr-state-pill", "PR State Pill", pr_state_pill::demo())
    });
    rt.route("/storybook/pr-avatar", "PR Avatar", |_| {
        route_def("route /storybook/pr-avatar", "PR Avatar", pr_avatar::demo())
    });
    rt.route("/storybook/repo-card", "Repo Card", |_| {
        route_def("route /storybook/repo-card", "Repo Card", repo_card::demo())
    });
    rt.route("/storybook/repos-menu", "Repos Menu", |_| {
        route_def("route /storybook/repos-menu", "Repos Menu", repos_menu::demo())
    });
    rt.route("/storybook/worktrees-menu", "Worktrees Menu", |_| {
        route_def("route /storybook/worktrees-menu", "Worktrees Menu", worktrees_menu::demo())
    });
    rt.route("/storybook/prs-menu", "Pull Requests Menu", |_| {
        route_def("route /storybook/prs-menu", "Pull Requests Menu", prs_menu::demo())
    });
    rt.route("/storybook/job-card", "Job Card", |_| {
        route_def("route /storybook/job-card", "Job Card", job_card::demo())
    });
    rt.route("/storybook/job-row", "Job Row", |_| {
        route_def("route /storybook/job-row", "Job Row", job_row::demo())
    });
    rt.route("/storybook/jobs-menu", "Jobs Menu", |_| {
        route_def("route /storybook/jobs-menu", "Jobs Menu", jobs_menu::demo())
    });
    rt.route("/storybook/pipeline-graph", "Pipeline Graph", |_| {
        route_def("route /storybook/pipeline-graph", "Pipeline Graph", pipeline_graph::demo())
    });
    rt.route("/storybook/pipelines-menu", "Pipelines Menu", |_| {
        route_def("route /storybook/pipelines-menu", "Pipelines Menu", pipelines_menu::demo())
    });
    rt.route("/storybook/stage-timeline", "Stage Timeline", |_| {
        route_def("route /storybook/stage-timeline", "Stage Timeline", stage_timeline::demo())
    });
    rt.route("/storybook/journal-tail", "Journal Tail", |_| {
        route_def("route /storybook/journal-tail", "Journal Tail", journal_tail::demo())
    });
    rt.route("/storybook/run-state-badge", "Run State Badge", |_| {
        route_def("route /storybook/run-state-badge", "Run State Badge", run_state_badge::demo())
    });
    rt.route("/storybook/job-run-chip", "Job Run Chip", |_| {
        route_def("route /storybook/job-run-chip", "Job Run Chip", job_run_chip::demo())
    });
    rt.route("/storybook/sandbox-chip", "Sandbox Chip", |_| {
        route_def("route /storybook/sandbox-chip", "Sandbox Chip", sandbox_chip_state::demo())
    });
    rt.route("/storybook/input-prompt", "Input Prompt", |_| {
        route_def("route /storybook/input-prompt", "Input Prompt", input_prompt::demo())
    });
    rt.route("/storybook/repo-panel", "Repo Panel", |_| {
        route_def("route /storybook/repo-panel", "Repo Panel", repo_panel::demo())
    });
    rt.route("/storybook/pr-panel", "PR Panel", |_| {
        route_def("route /storybook/pr-panel", "PR Panel", pr_panel::demo())
    });
    rt.route("/storybook/run-panel", "Run Panel", |_| {
        route_def("route /storybook/run-panel", "Run Panel", run_panel::demo())
    });
    rt.route("/storybook/job-panel", "Job Panel", |_| {
        route_def("route /storybook/job-panel", "Job Panel", job_panel::demo())
    });
    rt.route("/storybook/duration", "Duration", |_| {
        route_def("route /storybook/duration", "Duration", ui_duration::demo())
    });
    rt.route("/storybook/tooltip", "Tooltip", |_| {
        route_def("route /storybook/tooltip", "Tooltip", ui_tooltip::demo())
    });

    rt.route("/storybook/system-card", "System Card", |_| {
        route_def("route /storybook/system-card", "System Card", system_card::demo())
    });
    rt.route("/storybook/queued-card", "Queued Card", |_| {
        route_def("route /storybook/queued-card", "Queued Card", queued_card::demo())
    });
    rt.route("/storybook/identity-switcher", "Identity Switcher", |_| {
        route_def(
            "route /storybook/identity-switcher",
            "Identity Switcher",
            identity_switcher::demo(),
        )
    });
    rt.route("/storybook/share-dialog", "Share Dialog", |_| {
        route_def("route /storybook/share-dialog", "Share Dialog", share_dialog::demo())
    });
    rt.route("/storybook/profile-panel", "Profile Panel", |_| {
        route_def("route /storybook/profile-panel", "Profile Panel", profile_panel::demo())
    });
    rt.route("/storybook/user-card", "User Card", |_| {
        route_def("route /storybook/user-card", "User Card", user_card::demo())
    });
    rt.route("/storybook/assistant-card", "Assistant Card", |_| {
        route_def(
            "route /storybook/assistant-card",
            "Assistant Card",
            assistant_card::demo(),
        )
    });
    rt.route("/storybook/reasoning-card", "Reasoning Card", |_| {
        route_def(
            "route /storybook/reasoning-card",
            "Reasoning Card",
            reasoning_card::demo(),
        )
    });
    rt.route("/storybook/tool-call-card", "Tool Call Card", |_| {
        route_def(
            "route /storybook/tool-call-card",
            "Tool Call Card",
            tool_call_card::demo(),
        )
    });
    rt.route("/storybook/tool-response-card", "Tool Response Card", |_| {
        route_def(
            "route /storybook/tool-response-card",
            "Tool Response Card",
            tool_response_card::demo(),
        )
    });
    rt.route("/storybook/error-card", "Error Card", |_| {
        route_def("route /storybook/error-card", "Error Card", error_card::demo())
    });
    rt.route("/storybook/approval-card", "Approval Card", |_| {
        route_def(
            "route /storybook/approval-card",
            "Approval Card",
            approval_card::demo(),
        )
    });
    rt.route("/storybook/thinking", "Thinking", |_| {
        route_def("route /storybook/thinking", "Thinking", thinking::demo())
    });
    rt.route("/storybook/task-list", "Task List", |_| {
        route_def("route /storybook/task-list", "Task List", task_list::demo())
    });

    rt.route("/storybook/session-state", "Session State", |_| {
        route_def(
            "route /storybook/session-state",
            "Session State",
            m("div")
                .class("state-demo")
                .kids(vec![
                    Node::host("uiSessionState", &[])
                        .bind(
                            "tone",
                            &session_state::tone_of("$store.samples.sessions[2]"),
                        )
                        .bind(
                            "title",
                            &session_state::tone_of("$store.samples.sessions[2]"),
                        ),
                    Node::host("uiSessionState", &[])
                        .bind(
                            "tone",
                            &session_state::tone_of("$store.samples.sessions[1]"),
                        )
                        .bind(
                            "title",
                            &session_state::tone_of("$store.samples.sessions[1]"),
                        ),
                    Node::host("uiSessionState", &[])
                        .bind(
                            "tone",
                            &session_state::tone_of("$store.samples.sessions[3]"),
                        )
                        .bind(
                            "title",
                            &session_state::tone_of("$store.samples.sessions[3]"),
                        ),
                ]),
        )
    });
    rt.route("/storybook/tab", "Tab", |_| {
        route_def("route /storybook/tab", "Tab", tab::demo())
    });
    rt.route("/storybook/tab-bar", "Tab Bar", |_| {
        route_def("route /storybook/tab-bar", "Tab Bar", tab::tab_bar(samples::EXPR))
    });

    rt.route("/storybook/notification-toggle", "Notification Toggle", |_| {
        route_def("route /storybook/notification-toggle", "Notification Toggle", notification_toggle::button(samples::EXPR))
    });

    rt.route("/storybook/presence", "Presence", |_| {
        route_def("route /storybook/presence", "Presence", presence::demo())
    });

    rt.route("/storybook/group-editor", "Group Editor", |_| {
        route_def("route /storybook/group-editor", "Group Editor", crate::components::group_editor::demo())
    });
    rt.route("/storybook/session-editor", "Session Editor", |_| {
        route_def(
            "route /storybook/session-editor",
            "Session Editor",
            session_editor::demo(),
        )
    });
}

fn register_spa_nav() {
    let token = lifecycle::token();
    let cb = Closure::wrap(Box::new(move |ev: JsValue| {
        if !lifecycle::am_current_token(&token) {
            return;
        }
        if js::get_num(&ev, "button") != 0.0
            || js::get_bool(&ev, "ctrlKey")
            || js::get_bool(&ev, "metaKey")
            || js::get_bool(&ev, "shiftKey")
            || js::get_bool(&ev, "altKey")
        {
            return;
        }
        let target = js::get(&ev, "target");
        if js::is_missing(&target) {
            return;
        }
        let anchor = js::call(&target, "closest", &[JsValue::from_str("a[href]")]);
        if js::is_missing(&anchor) {
            return;
        }
        let href = js::call_string(&anchor, "getAttribute", &[JsValue::from_str("href")]);
        if !href.starts_with("/storybook") {
            return;
        }
        js::call(&ev, "preventDefault", &[]);
        mjs::router_set(&href);
    }) as Box<dyn FnMut(JsValue)>);
    js::call(
        &js::document(),
        "addEventListener",
        &[
            JsValue::from_str("click"),
            cb.as_ref().unchecked_ref::<JsValue>().clone(),
        ],
    );
    cb.forget();
}

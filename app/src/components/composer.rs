//! `composer` — the prompt bar under the transcript: the `task list`
//! panel, the draft box, and the row (agent select, model select, context
//! pie, allowlist toggle, send/stop).
//!
//! Verbatim app classes. The draft is two-way-bound to the shared `demo`
//! store so typing works; send/stop flip the waiting flag through it (the
//! selects stay native — the browser owns their state, no JS needed). The
//! allowlist toggle is resting chrome here; its panel is dynamic app state
//! and lives outside this showcase.
//!
//! Depth 1: the task list.

use rustex1_kit::h::{lit, m, Node};

use crate::components::task_list;
use crate::css;

pub const CSS: &str = r#"
/* demo framing: bottom bar, full demo width */
.composer-demo { width: min(560px, 100%); }
.composer-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

const DRAFT: &str = "$store.demo.composerDraft";
const WAITING: &str = "$store.demo.composerWaiting";

/// the prompt bar alone — the part the panel reuses.

fn option(value: &str, label: &str, selected: bool) -> Node {
    // NOTE: `selected="selected"`, not bare `selected` — m-js drops
    // valueless attributes from its template AST, so the bare form never
    // reaches the DOM (verified: the browser's own parser keeps it fine).
    let mut o = m("option").attr("value", value).child(lit(label));
    if selected {
        o = o.attr("selected", "selected");
    }
    o
}

fn composer_row() -> Node {
    m("div").class("composer-row").kids(vec![
        m("select")
            .class("dd agent-dd")
            .attr("title", "Agent")
            .kids(vec![
                option("cowork", "cowork", true),
                option("scout", "scout", false),
            ]),
        m("select")
            .class("dd model-dd")
            .attr("title", "Model")
            .kids(vec![
                option("opus", "opus", false),
                option("sonnet", "sonnet", true),
                option("haiku", "haiku", false),
            ]),
        // context window: static 38% pie (r=15.5 → C≈97.4, like the app).
        m("div")
            .class("ctx-pie")
            .attr("title", "38000 / 100000 tokens")
            .kids(vec![
                m("svg")
                    .at(&[("viewBox", "0 0 36 36"), ("width", "28"), ("height", "28")])
                    .kids(vec![
                        m("circle").at(&[
                            ("cx", "18"), ("cy", "18"), ("r", "15.5"),
                            ("fill", "none"), ("stroke", "currentColor"),
                            ("class", "ctx-pie-track"), ("stroke-width", "4"),
                        ]),
                        m("circle").at(&[
                            ("cx", "18"), ("cy", "18"), ("r", "15.5"),
                            ("fill", "none"), ("stroke", "currentColor"),
                            ("class", "ctx-pie-fill"), ("stroke-width", "4"),
                            ("stroke-linecap", "round"),
                            ("transform", "rotate(-90 18 18)"),
                            ("stroke-dasharray", "37.0 97.4"),
                        ]),
                    ]),
                m("span").class("ctx-pie-label").child(lit("38k")),
            ]),
        m("button")
            .class("icon-btn allowlist-toggle")
            .attr("type", "button")
            .attr("title", "Allowlist")
            .child(Node::icon("shield-check")),
        // waiting flag flips through the demo store: stop shows while
        // waiting, send (disabled on an empty draft) otherwise.
        m("button")
            .class("btn-stop")
            .attr("type", "button")
            .show(WAITING)
            .on("click", "$store.demo.composerStop()")
            .attr("title", "Stop agent")
            .kids(vec![Node::icon("stop"), m("span").child(lit("Stop"))]),
        m("button")
            .class("btn-primary")
            .attr("type", "button")
            .show(&format!("!{WAITING}"))
            .bind("disabled", &format!("!{DRAFT}.trim()"))
            .on("click", "$store.demo.composerSend()")
            .kids(vec![
                Node::icon("paper-plane-right"),
                m("span").child(lit("Send")),
            ]),
    ])
}

/// the prompt bar alone — the part the panel reuses.
pub fn panel() -> Node {
    m("footer").class("composer").child(
        m("div").class("composer-inner").kids(vec![
            task_list::task_list(task_list::SAMPLES),
            m("textarea")
                .class("draft-ta")
                .model(DRAFT)
                .attr("placeholder", "Message… (Enter send, Shift+Enter newline)"),
            composer_row(),
        ]),
    )
}

/// the composer, bound to the demo store.
pub fn composer() -> Node {
    m("div").class("mjs-chat mode-sidebar").child(panel())
}

/// standalone demo.
pub fn demo() -> Node {
    m("div").class("composer-demo").child(composer())
}

pub fn register() {
    css::inject("composer", CSS);
}

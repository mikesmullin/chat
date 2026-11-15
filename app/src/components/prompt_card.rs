//! `prompt card` — what the model was actually handed.
//!
//! Two of them, because two different things were sent: the **system
//! prompt** (who the agent is, assembled from its soul, goals and memory)
//! and the **user prompt** (the conversation as the agent received it,
//! flattened to text, with however many prior turns came along).
//!
//! Kept apart from what the person typed, and folded away by default: "why
//! did it answer that?" is unanswerable without them, and unaskable when
//! they are mixed into the conversation.

use rustex1_kit::h::{m, Node};

use crate::components::{fold_card, samples, session};
use crate::css;

pub const CSS: &str = r#"
.prompt-demo { width: min(560px, 100%); }
.prompt-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// The system prompt row.
pub fn system_parts(st: &str, v: &str) -> Vec<Node> {
    vec![fold_card::fold(
        st,
        v,
        "fold-system",
        "scroll",
        "System prompt",
        None,
        fold_card::pre(&format!("{v}.text")),
    )
    .iff(&format!("{v}.kind === 'system_prompt'"))]
}

/// The user prompt row, as the agent received it.
pub fn user_parts(st: &str, v: &str) -> Vec<Node> {
    vec![fold_card::fold(
        st,
        v,
        "fold-prompt",
        "arrow-line-right",
        "User prompt",
        // How many prior turns went with it, but not its size: the first
        // line of the prompt itself is the useful preview, and a byte count
        // was taking room from it to say something nobody acts on.
        Some(&format!(
            "({v}.historyTurns ? {v}.historyTurns + ' prior' : '')"
        )),
        fold_card::pre(&format!("{v}.text")),
    )
    .iff(&format!("{v}.kind === 'agent_prompt'"))]
}

/// standalone demo: both, as the transcript stacks them.
pub fn demo() -> Node {
    let st = samples::EXPR;
    let sys = samples::row("system_prompt");
    let usr = samples::row("agent_prompt");
    m("div").class("prompt-demo").child(
        m("div").class("mjs-chat mode-sidebar").kids(vec![
            session::row(st, &sys, system_parts(st, &sys)),
            session::row(st, &usr, user_parts(st, &usr)),
        ]),
    )
}

pub fn register() {
    css::inject("prompt-card", CSS);
}

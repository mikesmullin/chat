use rustex1_kit::h::{m, Node};

use crate::components::{fold_card, samples, session};
use crate::css;

pub const CSS: &str = r#"
.prompt-demo { width: min(560px, 100%); }
.prompt-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

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

pub fn user_parts(st: &str, v: &str) -> Vec<Node> {
    vec![fold_card::fold(
        st,
        v,
        "fold-prompt",
        "arrow-line-right",
        "User prompt",
        Some(&format!(
            "({v}.historyTurns ? {v}.historyTurns + ' prior' : '')"
        )),
        fold_card::pre(&format!("{v}.text")),
    )
    .iff(&format!("{v}.kind === 'agent_prompt'"))]
}

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

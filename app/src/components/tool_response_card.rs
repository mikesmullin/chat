use rustex1_kit::h::{m, Node};

use crate::components::{fold_card, samples, session};
use crate::css;

pub const CSS: &str = r#"
.tool-response-demo { width: min(560px, 100%); }
.tool-response-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![fold_card::fold_as(
        st,
        v,
        "fold-tool-result",
        &format!("({v}.ok === false || {v}.denied ? 'is-err' : 'ok')"),
        fold_card::head_dyn(
            &format!("'ph ' + ({v}.ok === false || {v}.denied ? 'ph-prohibit' : 'ph-check-circle')"),
            &format!("{v}.name"),
            Some(&format!(
                "({v}.denied ? 'denied' : {v}.ok === false ? 'failed' : 'ok')"
            )),
        ),
        fold_card::pre(&format!("{v}.text || ''")),
    )
    .iff(&format!("{v}.kind === 'tool_result'"))]
}

pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::row("tool_result"));
    m("div").class("tool-response-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(session::row(st, &v, parts(st, &v))),
    )
}

pub fn register() {
    css::inject("tool-response-card", CSS);
}

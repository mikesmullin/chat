//! `user card` — what the person typed.
//!
//! The bubble, with a fork-from-here button beside its time below. One definition: the transcript composes [`parts`] and so
//! does this module's own demo, so there is no second drawing to keep in
//! step.

use rustex1_kit::h::{m, Node};

use crate::components::{name_tag, samples, session};
use crate::css;

pub const CSS: &str = r#"
/* The fork action sits beside the timestamp, within easy reach. */
:is(#sb, #home) .bubble-fork {
  display: inline-flex; align-items: center; justify-content: center;
  width: 24px; height: 24px;
  border: 0; background: transparent; color: var(--gray-dim);
  cursor: pointer; padding: 2px; line-height: 16px; font: inherit;
  opacity: 0; pointer-events: none; transition: opacity 100ms ease;
}
:is(#sb, #home) .msg-user:hover .bubble-fork,
:is(#sb, #home) .bubble-fork:focus-visible { opacity: 1; pointer-events: auto; }
:is(#sb, #home) .bubble-fork:hover { color: var(--accent); }
:is(#sb, #home) .bubble-fork:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; border-radius: 4px; }

.user-demo { width: min(560px, 100%); }
.user-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// What this kind contributes to a message block.
pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![
        m("div")
            .class("bubble-user")
            .iff(&format!("{v}.kind === 'user'"))
            .child(m("div").class("md-body").inner_html(&format!("{v}.html"))),
        // The time and prompt action share a byline under the bubble.
        name_tag::user_byline(
            st,
            &format!("{v}.ts"),
            &format!("{v}.kind === 'user'"),
            m("button")
                .class("bubble-fork")
                .attr("type", "button")
                .attr("title", "Edit this prompt on a new branch")
                .attr("aria-label", "Edit this prompt on a new branch")
                .on("click", &format!("{st}.forkPrompt({v}.eventId, {v}.text)"))
                .child(Node::icon("git-branch").class("ph ph-git-branch")),
        ),
    ]
}

/// standalone demo — the card in the row the transcript draws it in.
pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::row("user"));
    m("div").class("user-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(session::row(st, &v, parts(st, &v))),
    )
}

pub fn register() {
    css::inject("user-card", CSS);
}

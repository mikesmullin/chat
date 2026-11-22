//! `user card` — what the person typed.
//!
//! The bubble, the fork-from-here button it reveals on hover, and a bare
//! time under it. One definition: the transcript composes [`parts`] and so
//! does this module's own demo, so there is no second drawing to keep in
//! step.

use rustex1_kit::h::{m, Node};

use crate::components::{name_tag, samples, session};
use crate::css;

pub const CSS: &str = r#"
/* The user's bubble carries the fork button, revealed on hover so it does
   not compete with the message it sits on. */
:is(#sb, #home) .bubble-user { position: relative; }
:is(#sb, #home) .bubble-fork {
  position: absolute; top: 4px; right: 4px;
  border: 0; background: transparent; color: var(--gray-dim);
  cursor: pointer; padding: 2px; line-height: 16px; font: inherit;
  opacity: 0; transition: opacity 100ms ease;
}
:is(#sb, #home) .bubble-user:hover .bubble-fork { opacity: .8; }
:is(#sb, #home) .bubble-fork:hover { color: var(--accent); opacity: 1; }

.user-demo { width: min(560px, 100%); }
.user-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// What this kind contributes to a message block.
pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![
        m("div")
            .class("bubble-user")
            .iff(&format!("{v}.kind === 'user'"))
            .kids(vec![
                m("div").class("md-body").inner_html(&format!("{v}.html")),
                // Edit the prompt on a new branch, preserving the old path.
                m("button")
                    .class("bubble-fork")
                    .attr("title", "Edit this prompt on a new branch")
                    .on("click", &format!("{st}.forkPrompt({v}.eventId, {v}.text)"))
                    .child(Node::icon("git-branch").class("ph ph-git-branch")),
            ]),
        // A time but no name: the message is theirs, and in a conversation of
        // one human a byline naming them is noise. Right-aligned under the
        // bubble, which the block's `flex-end` handles.
        name_tag::bare_time(
            st,
            &format!("{v}.ts"),
            &format!("{v}.kind === 'user' && {v}.ts"),
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

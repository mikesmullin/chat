//! `name tag` — a byline: who said it, and when.
//!
//! Two forms, because two things are being said. Under an agent's answer the
//! byline names them, in their own colour, with their own glyph — in a group
//! that attribution is the only thing telling three bubbles apart, and even
//! alone an agent has a name and a time. Under the user's own message there
//! is only a time: a byline naming the human to the human is noise.
//!
//! No `@`: the sigil is command syntax (`/invite @name`), not part of
//! anybody's name, and repeating it on every byline makes the transcript read
//! like a mention list. The icon and the colour carry "this is a participant",
//! which is what the `@` was doing.

use rustex1_kit::h::{m, Node};

use crate::css;

pub const CSS: &str = r#"
/* A participant's byline, under what they said. */
:is(#sb, #home) .name-tag {
  /* `inline-flex`, not `flex`: the assistant's byline sits inside the bubble,
     which is a plain block, so `align-self` cannot reach it and a block-level
     flex box would stretch the full width for a line of 100px of text. */
  display: inline-flex; align-items: center; gap: 6px;
  font-size: 11px; line-height: 18px;
  color: var(--agent-accent, #a78bfa);
}
:is(#sb, #home) .name-tag-who { display: inline-flex; align-items: center; gap: 5px; }
:is(#sb, #home) .name-tag i { font-size: 13px; line-height: 18px; }
/* When, in the dim the rest of the chrome uses: it is a footnote to the
   byline, not a second thing being announced. */
:is(#sb, #home) .name-tag-when { color: var(--gray-dim); }
"#;

/// The full byline: glyph, handle, and how long ago.
///
/// `when` is the condition it shows under — usually "there is a handle".
pub fn byline(st: &str, handle_expr: &str, ts_expr: &str, when: &str) -> Node {
    m("div")
        .class("name-tag")
        .iff(when)
        .kids(vec![
            m("span").class("name-tag-who").kids(vec![
                // The author's own glyph, resolved from the roster by
                // handle — the same route the byline's colour takes.
                Node::icon_dyn(&format!("{st}.iconOf({handle_expr} || '')")),
                m("span").text(handle_expr),
            ]),
            // Immediately after the name, reading as one phrase — not pushed
            // to the far edge, where it would read as a separate column.
            m("span")
                .class("name-tag-when")
                .show(ts_expr)
                .text(&format!("'· ' + {st}.relAgo({ts_expr})")),
        ])
}

/// The time on its own, for the user's message.
pub fn bare_time(st: &str, ts_expr: &str, when: &str) -> Node {
    m("div")
        .class("name-tag name-tag-bare")
        .iff(when)
        .child(
            m("span")
                .class("name-tag-when")
                .text(&format!("{st}.relAgo({ts_expr})")),
        )
}

pub fn register() {
    css::inject("name-tag", CSS);
}

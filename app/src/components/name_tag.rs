//! `name tag` — a byline: who said it, and when.
//!
//! Each byline names its author and shows when they spoke. User prompts also
//! have an action alongside the time.
//!
//! No `@`: the sigil is command syntax (`/invite @name`), not part of
//! anybody's name, and repeating it on every byline makes the transcript read
//! like a mention list. The icon and the colour carry "this is a participant",
//! which is what the `@` was doing.

use rustex1_kit::h::{m, Node};

use crate::components::presence;
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
/* The author mark as a control: a button wearing the byline's clothes
   exactly — no chrome of its own, just a pointer and a focus ring. Only
   button chrome is reset here: `font: inherit` would outrank `.name-tag`'s
   own 11px/18px (this selector is one class more specific) and the mark
   would stop matching a byline.
   */
:is(#sb, #home) button.name-tag {
  background: transparent; border: 0; padding: 0;
  font-family: inherit; text-align: left; cursor: pointer;
}
:is(#sb, #home) button.name-tag:focus-visible {
  outline: 2px solid var(--accent); outline-offset: 2px; border-radius: 4px;
}
"#;

fn who(icon: &str, name: &str) -> Node {
    m("span").class("name-tag-who").kids(vec![
        Node::icon_dyn(icon),
        m("span").text(name),
    ])
}

/// The author mark as a **control**: the same glyph and ink as [`mention`],
/// as a button — the sandbox form's agent mark links to the agent's own
/// editor. The parts are expressions, so the caller resolves them however
/// it knows them (`mention` computes them from a handle; the form carries
/// them from its seed).
pub fn who_link(icon: &str, accent: &str, name: &str, on: &str) -> Node {
    m("button")
        .class("name-tag")
        .attr("type", "button")
        .bind("style", &format!("('--agent-accent: ' + ({accent}))"))
        .on("click", on)
        .child(who(icon, name))
}

/// The same author mark used on a byline, for a mention in a status line.
pub fn mention(st: &str, handle: &str) -> Node {
    let user = format!("({handle} === 'user' || ({st}.userName && {handle} === {st}.userName))");
    let icon = format!("({user} ? '{}' : {st}.iconOf({handle}))", presence::USER_ICON);
    m("span")
        .class("name-tag")
        .bind("style", &format!("('--agent-accent: ' + ({user} ? '{}' : {st}.accentOf({handle})))", presence::USER_ACCENT))
        .child(who(&icon, handle))
}

/// The full byline: glyph, handle, and how long ago.
///
/// `when` is the condition it shows under — usually "there is a handle".
pub fn byline(st: &str, handle_expr: &str, ts_expr: &str, when: &str) -> Node {
    m("div")
        .class("name-tag")
        .iff(when)
        .kids(vec![
            who(&format!("{st}.iconOf({handle_expr} || '')"), handle_expr),
            // Immediately after the name, reading as one phrase — not pushed
            // to the far edge, where it would read as a separate column.
            m("span")
                .class("name-tag-when")
                .show(ts_expr)
                .text(&format!("'· ' + {st}.relAgo({ts_expr})")),
        ])
}

/// The current user's byline, with the prompt action after the time.
/// The user's byline under a bubble — named for whoever sent it: a
/// multi-human room shows who said it, not always "You" (PLAN44 A6).
pub fn user_byline_as(
    st: &str,
    name_expr: &str,
    ts_expr: &str,
    when: &str,
    action: Node,
) -> Node {
    m("div")
        .class("name-tag name-tag-user")
        .attr("style", &format!("--agent-accent: {}", presence::USER_ACCENT))
        .iff(when)
        .kids(vec![
            who(&format!("'{}'", presence::USER_ICON), name_expr),
            m("span")
                .class("name-tag-when")
                .show(ts_expr)
                .text(&format!("'· ' + {st}.relAgo({ts_expr})")),
            action,
        ])
}

pub fn register() {
    css::inject("name-tag", CSS);
}

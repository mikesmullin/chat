//! `session` — the middle column: the transcript for one session, a
//! stack of message cards in `msg-block` rows (user rows right-aligned)
//! plus the thinking indicator.
//!
//! Composed literally from the card builders, so each part renders
//! identically alone and in the transcript. Depth 5: user + assistant +
//! tool call + tool response + thinking.

use rustex1_kit::h::{m, Node};

use crate::components::{
    assistant_card, thinking, tool_call_card, tool_response_card, user_card,
};
use crate::css;

pub const CSS: &str = r#"
/* demo framing: transcript column size; the sidebar divider is an edge
   against a neighbouring column, not a property of the column */
.session-demo { width: min(640px, 100%); height: 560px; }
.session-demo .mjs-chat.mode-sidebar { border-left: 0; }
.session-demo .mjs-chat { height: 100%; }
"#;

/// one row: the card, right-aligned for the user's own messages.
fn row(card: Node, user: bool) -> Node {
    m("div")
        .class(if user { "msg-block msg-user" } else { "msg-block" })
        .child(card)
}

/// the message list alone — the part the panel reuses.
pub fn messages() -> Node {
    m("div").class("messages").kids(vec![
        row(user_card::card(user_card::TEXT), true),
        row(assistant_card::card(&[assistant_card::PARA_1]), false),
        row(
            tool_call_card::card(
                tool_call_card::NAME,
                tool_call_card::EXPLANATION,
                tool_call_card::CODE,
            ),
            false,
        ),
        row(
            tool_response_card::card(
                tool_response_card::NAME,
                tool_response_card::BODY,
                true,
            ),
            false,
        ),
        row(assistant_card::card(&[assistant_card::PARA_2]), false),
        // bare, as in the app — the indicator is not a message.
        thinking::thinking(),
    ])
}

/// the transcript column: messages in a `section.main`.
pub fn transcript() -> Node {
    m("section").class("main").child(messages())
}

/// the transcript: messages in a `section.main`, inside the chat shell.
pub fn session() -> Node {
    m("div")
        .class("mjs-chat mode-sidebar")
        .attr("aria-label", "Session")
        .child(transcript())
}

/// standalone demo.
pub fn demo() -> Node {
    m("div").class("session-demo").child(session())
}

pub fn register() {
    css::inject("session", CSS);
}

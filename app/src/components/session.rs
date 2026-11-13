//! `session` — the middle column: the transcript for one session, a
//! stack of message cards in `msg-block` rows (user rows right-aligned)
//! plus the thinking indicator.
//!
//! Composed literally from the card builders, so each part renders
//! identically alone and in the transcript. Depth 5: user + assistant +
//! tool call + tool response + thinking.

use rustex1_kit::h::{lit, m, Node};

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

// ===========================================================================
// Live variant — the same card classes, bound to `$store.live.messages`.
// ===========================================================================

/// The transcript driven by angela.
///
/// **One** `x-for` with per-kind children, not one loop per kind: separate
/// loops would group all the user rows, then all the assistant rows, and lose
/// chronological order. `x-if` on a child is evaluated in the row scope, so
/// each row renders exactly the card its `kind` names.
///
/// Classes are verbatim from the storybook cards — the contract — so the
/// design guide and the running transcript are the same markup.
pub fn messages_live() -> Node {
    let st = "$store.live";
    m("div")
        .class("messages")
        // Sticky scroll. The effect names its dependencies explicitly so
        // m-js tracks them, then defers the scroll to `$nextTick` — the DOM
        // only reflects the new rows after the diff commits, so scrolling
        // during the effect would land one row short. `stickBottom` is set
        // by the scroll handler, so a user who scrolls up to read is not
        // dragged back down by every arriving token.
        .effect(&format!(
            "({st}.messages.length, {st}.streamText, {st}.streamReasoning, \
              {st}.waiting, {st}.keepPinned($el))"
        ))
        .on("scroll", &format!("{st}.onScroll($event)"))
        .kids(vec![
        // --- the transcript ------------------------------------------------
        m("div")
            .class("msg-block")
            .for_each(&format!("m in {st}.messages"))
            .key("m.id")
            .bind("class", "(m.kind === 'user' ? 'msg-user' : '')")
            .kids(vec![
                m("div")
                    .class("bubble-user")
                    .iff("m.kind === 'user'")
                    .text("m.text"),
                // Pre-rendered in the store. Safe for `x-html` because the
                // renderer escapes the source before emitting any tag, so
                // every tag here is one it wrote (see helpers::markdown_to_html).
                m("div")
                    .class("bubble-assistant md-body")
                    .iff("m.kind === 'assistant'")
                    .inner_html("m.html || m.text"),
                m("div")
                    .class("bubble-error")
                    .iff("m.kind === 'error'")
                    .text("m.text"),
                m("div")
                    .class("bubble-reasoning")
                    .iff("m.kind === 'reasoning'")
                    .kids(vec![
                        m("div").class("reasoning-header").kids(vec![
                            Node::icon("brain").class("ph ph-brain"),
                            m("span").class("reasoning-label").child(lit("Reasoning")),
                        ]),
                        // Same pre-rendered markdown as the answer card; safe
                        // for `x-html` because the renderer escapes before it
                        // emits any tag.
                        m("div")
                            .class("reasoning-body md-body")
                            .inner_html("m.html || m.text"),
                    ]),
                m("div")
                    .class("tool-card tool-call-card")
                    .iff("m.kind === 'tool_call'")
                    .kids(vec![
                        m("div").class("tool-call-header").kids(vec![
                            Node::icon("wrench").class("ph ph-wrench"),
                            m("span").class("tool-call-name").text("m.name"),
                        ]),
                        m("pre")
                            .class("tool-call-code")
                            .text("m.args ? JSON.stringify(m.args) : ''"),
                    ]),
                m("div")
                    .class("tool-card tool-card-row")
                    .iff("m.kind === 'tool_result'")
                    // A denied or failed call must not wear the green rule.
                    .bind("class", "(m.ok === false || m.denied ? 'err' : 'ok')")
                    .kids(vec![
                        m("div").class("tool-result-header").kids(vec![
                            m("i").bind(
                                "class",
                                "'ph ' + (m.ok === false || m.denied ? 'ph-prohibit' : 'ph-check-circle')",
                            ),
                            m("span").text("m.name"),
                        ]),
                        m("pre").class("tool-result-body").text("m.text || ''"),
                    ]),
            ]),

        // --- the turn in flight ---------------------------------------------
        // Streamed deltas live in their own store strings and are swapped for
        // a real row when the sealed card arrives, so a 60-per-second token
        // append never rebuilds the message list.
        m("div")
            .class("msg-block")
            .show(&format!("{st}.streamReasoning"))
            .child(
                m("div").class("bubble-reasoning").kids(vec![
                    m("div").class("reasoning-header").kids(vec![
                        Node::icon("brain").class("ph ph-brain"),
                        m("span").class("reasoning-label").child(lit("Reasoning")),
                    ]),
                    m("div")
                        .class("reasoning-body md-body")
                        .inner_html(&format!(
                            "{st}.streamReasoningHtml || {st}.streamReasoning"
                        )),
                ]),
            ),
        // The in-flight bubble renders markdown as it arrives. The renderer
        // is prefix-safe, so an unterminated fence, a half-typed `**` or a
        // partial link render as themselves instead of leaking broken tags.
        m("div")
            .class("msg-block")
            .show(&format!("{st}.streamText"))
            .child(
                m("div")
                    .class("bubble-assistant md-body streaming")
                    .inner_html(&format!("{st}.streamHtml || {st}.streamText")),
            ),
        // Visible for the WHOLE turn, not just before the first token: the
        // model is still working while it streams, and the indicator is what
        // says the turn has not reached its stop reason yet. It is last in
        // the list, so it always sits at the bottom of the transcript.
        m("div")
            .class("msg-block thinking-block")
            .show(&format!("{st}.waiting"))
            .child(thinking::thinking()),
    ])
}

//! `queued card` — the "waiting for" card.
//!
//! **Live state, never history.** The card is a projection of the session's
//! message queue: the messages waiting to become turns, why the line waits,
//! and a way to take each one back. It is bound to the store's `queue` (the
//! feed pushes it), not to a message row — a queued message is pending work
//! and history has no place for it. The card is on screen exactly while the
//! queue holds something: no enqueued message, no card.
//!
//! The waits of *finished* turns leave nothing behind. A turn that waited
//! for a model slot, or a request parked on the session's configuration,
//! used to leave a row on the rail; the rule now is the same for every
//! wait: it is shown while it holds a message, and gone when it does not.

use rustex1_kit::h::{lit, m, Node};

use crate::components::samples;
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .bubble-queued {
  display: flex; gap: 7px; align-items: center;
  font-size: 12px; color: var(--gray-dim); line-height: 1.6;
  padding: 2px 0; white-space: pre-wrap;
}
:is(#sb, #home) .bubble-queued i { line-height: 19px; }
:is(#sb, #home) .queued-text { overflow-wrap: anywhere; min-width: 0; }

/* One card, one line per waiting message. The reason explains the line;
   each message can be taken back on its own. */
:is(#sb, #home) .queue-card { align-items: flex-start; }
:is(#sb, #home) .queue-card > i { line-height: 19px; margin-top: 1px; }
:is(#sb, #home) .queue-lines {
  display: flex; flex-direction: column; gap: 3px; min-width: 0; flex: 1 1 auto;
}
:is(#sb, #home) .queue-item {
  display: flex; gap: 8px; align-items: baseline; min-width: 0;
}
:is(#sb, #home) .queue-text {
  flex: 1 1 auto; min-width: 0; overflow-wrap: anywhere;
  color: var(--fg); opacity: .85;
}
:is(#sb, #home) .queue-dequeue {
  flex: none; display: inline-flex; align-items: center; gap: 4px;
  border: 0; background: transparent; color: var(--gray-dim);
  font: inherit; font-size: 11px; line-height: 16px;
  cursor: pointer; padding: 1px 5px; border-radius: 6px;
}
:is(#sb, #home) .queue-dequeue:hover { color: var(--mjs-err); background: var(--panel-2); }
:is(#sb, #home) .queue-dequeue:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }

.queued-demo { width: min(560px, 100%); }
.queued-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

/// The "waiting for" card: the messages waiting to become turns.
///
/// Bound to `{st}.queue` — live state the feed pushes — rather than to a
/// message row: a queued message is pending work and history has no place
/// for it. The line is explained by its head's reason; each message carries
/// its own way back to the composer. The card shows itself only while the
/// queue holds a message.
pub fn queue_parts(st: &str) -> Vec<Node> {
    vec![
        m("div")
            .class("bubble-queued queue-card")
            .show(&format!("{st}.queue.items.length > 0"))
            .kids(vec![
                Node::icon("hourglass").class("ph ph-hourglass"),
                m("div").class("queue-lines").kids(vec![
                    m("div")
                        .class("queued-text")
                        .text(&format!("{st}.queue.reason || 'waiting to run'")),
                    m("div")
                        .class("queue-item")
                        .for_each(&format!("(q, i) in {st}.queue.items"))
                        .key("q.id")
                        .kids(vec![
                            m("span").class("queue-text").text("q.text"),
                            m("button")
                                .class("queue-dequeue")
                                .attr("type", "button")
                                .attr("title", "Dequeue — this message goes back to the composer")
                                .on("click", &format!("{st}.dequeue(q.id)"))
                                .kids(vec![Node::icon("x"), m("span").child(lit("Dequeue"))]),
                        ]),
                ]),
            ]),
    ]
}

/// standalone demo — the card over the guide's samples, both of its states:
/// the line held up by the session's configuration, and the dequeue that
/// takes a message back.
pub fn demo() -> Node {
    m("div").class("queued-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(m("div").class("messages").kids(queue_parts(samples::EXPR))),
    )
}

pub fn register() {
    css::inject("queued-card", CSS);
}

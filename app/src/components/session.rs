//! `session` — the middle column: the transcript for one session, a
//! stack of message cards in `msg-block` rows (user rows right-aligned)
//! plus the thinking indicator.
//!
//! Composed literally from the card builders, so each part renders
//! identically alone and in the transcript. Depth 5: user + assistant +
//! tool call + tool response + thinking.

use rustex1_kit::h::{lit, m, Node};

use crate::components::{
    assistant_card, thinking, thread, tool_call_card, tool_response_card, user_card,
};
use crate::css;

pub const CSS: &str = r#"
/* ---- the turn rail -------------------------------------------------------
   One inference request is drawn as a run of rows down a vertical line, with
   a dot per event. Everything is indented to the same content column — the
   user's own message included — so the conversation reads as one column with
   the machinery hanging off a rail beside it, rather than as two layouts. */
:is(#sb, #home) .turn-row {
  display: flex;
  align-items: stretch;
  gap: 10px;
}
:is(#sb, #home) .turn-rail {
  flex: 0 0 auto;
  width: 18px;
  position: relative;
  display: flex;
  justify-content: center;
}
/* The rail lives inside each row, but the rows are laid out by a flex column
   with a gap between them — so a line that stops at the row's edge leaves a
   hole at every join. Each segment therefore runs from its own row's top
   down to where the NEXT row starts: the pieces **tile**, meeting edge to
   edge.
   Extending at both ends instead would also close the holes, but the
   segments would then overlap by the gap — and two translucent overlaps
   composite darker than the rest, which is what made the line look dashed.
   For the same reason the alpha is baked into the colour rather than applied
   with `opacity`: a stray overlap then paints the same value twice instead
   of compounding. `--turn-gap` has to match the container's `gap`. */
:is(#sb, #home) .turn-row { --turn-gap: 4px; --dot-center: 10px; }
/* The dot sits on the first line of whatever the row holds, so where that
   line falls has to be matched per row type. A collapsed card is a bare
   19px line, centre 10px down. An answer bubble has 4px of padding above a
   20px line, so its first line centres 14px down — and since the answer is
   normally the run's last row, this is also where the rail stops. */
:is(#sb, #home) .turn-row:has(.bubble-assistant) { --dot-center: 14px; }
:is(#sb, #home) .turn-row.in-turn .turn-rail::before {
  content: "";
  position: absolute;
  top: 0;
  bottom: calc(var(--turn-gap) * -1);
  left: 50%;
  /* 4px rather than 2: a hairline at a fractional offset rasterises
     differently from row to row, which reads as a line that wavers. */
  width: 4px; margin-left: -2px;
  background: color-mix(in srgb, var(--agent-accent, #a78bfa) 45%, transparent);
}
/* A run begins and ends at a dot, not at a row edge. */
:is(#sb, #home) .turn-row.turn-first .turn-rail::before {
  top: var(--dot-center); border-radius: 2px 2px 0 0;
}
:is(#sb, #home) .turn-row.turn-last .turn-rail::before {
  top: 0;
  bottom: auto;
  height: var(--dot-center);
  border-radius: 0 0 2px 2px;
}
/* A turn of one row is a dot on its own; a stub of line would read as a run
   that lost its other half. */
:is(#sb, #home) .turn-row.turn-first.turn-last .turn-rail::before { display: none; }
:is(#sb, #home) .turn-dot {
  position: absolute;
  top: calc(var(--dot-center) - 9px); left: 50%;
  /* 18px box, 14px visible disc: the 2px ring is painted in the panel's own
     background, so it punches a clean gap in the line running behind it.
     Without it the rail meets the dot edge to edge and the two read as one
     lumpy stroke rather than as beads on a thread. Box-sizing keeps the disc
     at 14px, and 18px is exactly the gutter width.
     No z-index needed: a `::before` paints as its element's first child, so
     this real child lands on top of the line. */
  width: 18px; height: 18px; margin-left: -9px;
  box-sizing: border-box;
  border: 2px solid var(--panel, #161616);
  border-radius: 50%;
  background: var(--agent-accent, #a78bfa);
}
/* What the user typed is not part of anyone's turn: indented, no dot, no
   rail. */
:is(#sb, #home) .turn-row.no-turn .turn-dot { display: none; }
:is(#sb, #home) .turn-row .msg-block { flex: 1 1 auto; min-width: 0; }
/* The user's own message is right-aligned and sized to its content.
   The skin got both from `.msg-block` being a direct child of the flex
   column (`align-self: flex-end`, no stretch). Wrapping each row in
   `.turn-row` made it a child of a flex ROW instead, where `align-self`
   means vertical alignment and the block stretches — so the alignment and
   the shrink-to-fit are re-stated here in terms the new layout has.
   `align-items: flex-end` on the column does both: it pushes the bubble to
   the right edge AND stops it stretching, so it sizes to its text. */
:is(#sb, #home) .turn-row .msg-block.msg-user {
  align-items: flex-end;
  /* The skin caps the block at `min(720px, 85%)`. In a flex row that cap
     leaves free space which the block, sitting at the start, does not use —
     so the bubble right-aligned to 85% across rather than to the row's edge.
     `margin-left: auto` eats the free space and moves the block right. */
  margin-left: auto;
}
/* …but the skin sets `width: 100%` on the bubble itself, which would undo
   the shrink-to-fit. `auto` restores it; the cap is the skin's. */
:is(#sb, #home) .turn-row .msg-block.msg-user .bubble-user {
  width: auto;
  max-width: min(720px, 85%);
}
/* The byline hugs its text rather than stretching across the column, and
   sits the same distance below the answer as any two events sit apart. */
:is(#sb, #home) .turn-row .name-tag { align-self: flex-start; margin-top: 4px; }
/* The user's block is `align-items: flex-end`, so this lands on the same
   right edge the bubble does. */
:is(#sb, #home) .turn-row .msg-user .name-tag { align-self: flex-end; }
/* Putting the answer in its own div (so the byline can be a sibling) moved
   the rendered markdown one level down, which put it out of reach of the
   skin's `.md-body > :first-child { margin-top: 0 }`. The first paragraph
   kept its `0.55em` top margin and the last kept its bottom one — about 7px
   of padding at each end that nobody asked for, and the reason the first
   line sat below its dot. */
:is(#sb, #home) .turn-row .bubble-assistant > div > :first-child { margin-top: 0; }
:is(#sb, #home) .turn-row .bubble-assistant > div > :last-child { margin-bottom: 0; }

/* ---- foldable evidence cards --------------------------------------------
   The system prompt, the prompt as sent, tool calls and results, reasoning
   and the engagement decision. All start shut: essential when an answer
   needs explaining, noise the rest of the time. */
:is(#sb, #home) .fold-card {
  width: 100%;
  margin: 4px 0 8px;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid color-mix(in srgb, var(--agent-accent, #a78bfa) 35%, transparent);
  background: color-mix(in srgb, var(--agent-accent, #a78bfa) 12%, transparent);
  color: color-mix(in srgb, var(--agent-accent, #a78bfa) 55%, #ffffff);
  cursor: pointer;
}
:is(#sb, #home) .fold-head {
  display: flex; align-items: center; gap: 6px;
  margin-bottom: 6px;
  font-size: 12px; font-weight: 600;
  letter-spacing: 0.02em; text-transform: uppercase;
  color: color-mix(in srgb, var(--agent-accent, #a78bfa) 72%, #ffffff);
}
:is(#sb, #home) .fold-note {
  font-weight: 400; text-transform: none; letter-spacing: 0;
  opacity: .75; flex: 0 0 auto;
}
:is(#sb, #home) .fold-body {
  font-size: 13px; line-height: 1.5;
  word-break: break-word;
}
:is(#sb, #home) .fold-pre {
  margin: 0; white-space: pre-wrap; font-family: var(--mono, monospace);
  font-size: 12px; max-height: 60vh; overflow: auto;
}
/* A failed or denied tool result is not in the agent's colour: the point is
   that something went wrong, which a tint cannot say. */
:is(#sb, #home) .fold-tool-result.err {
  border-color: rgba(217, 83, 79, .4);
  background: rgba(217, 83, 79, .1);
}
:is(#sb, #home) .fold-tool-result.err .fold-head { color: #e8908c; }

/* Collapsed: one line, no chrome at all — just the tinted text. */
:is(#sb, #home) .fold-card.is-collapsed {
  display: flex; align-items: baseline; gap: 6px;
  min-width: 0;
  margin: 2px 0; padding: 0;
  background: transparent; border: 0; border-radius: 0;
}
:is(#sb, #home) .fold-card.is-collapsed .fold-head {
  flex: 0 0 auto; margin-bottom: 0; min-width: 0;
}
:is(#sb, #home) .fold-card.is-collapsed .fold-body {
  flex: 1 1 auto; min-width: 0;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  max-height: none;
}
/* The body may be rendered markdown, so it is full of block elements. One
   line only really is one line once they are inline and their own
   white-space rules (a <pre> keeps its newlines) give way to the clamp. */
:is(#sb, #home) .fold-card.is-collapsed .fold-body * {
  display: inline; margin: 0; padding: 0;
  white-space: inherit; border: 0; background: transparent;
}
/* A <br> would break the line regardless of white-space. */
:is(#sb, #home) .fold-card.is-collapsed .fold-body br { display: none; }

/* The app talking: a slash command's answer, or "nobody is here to reply".
   Deliberately not a bubble — it is not something anyone said. */
:is(#sb, #home) .bubble-system {
  display: flex; gap: 7px; align-items: flex-start;
  font-size: 12px; color: var(--gray-dim); line-height: 1.6;
  padding: 2px 0; white-space: pre-wrap;
}
:is(#sb, #home) .bubble-system i { line-height: 19px; }

/* Fork-from-here, revealed on hover so it does not compete with the message
   it sits on. */
:is(#sb, #home) .bubble-user { position: relative; }
:is(#sb, #home) .bubble-fork {
  position: absolute; top: 4px; right: 4px;
  border: 0; background: transparent; color: var(--gray-dim);
  cursor: pointer; padding: 2px; line-height: 16px; font: inherit;
  opacity: 0; transition: opacity 100ms ease;
}
:is(#sb, #home) .bubble-user:hover .bubble-fork { opacity: .8; }
:is(#sb, #home) .bubble-fork:hover { color: var(--accent); opacity: 1; }

/* A participant's byline, under what they said. Only in a group, where it
   is the one thing distinguishing three agents. */
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
/// A participant's name, wherever one is shown.
///
/// No `@`: the sigil is command syntax (`/invite @name`), not part of
/// anybody's name, and repeating it on every byline makes the transcript
/// read like a mention list. The icon and the agent's own colour carry the
/// "this is a participant" signal instead, which is what the `@` was doing.
fn name_tag(handle_expr: &str, ts_expr: &str, when: &str) -> Node {
    m("div")
        .class("name-tag")
        .iff(when)
        .kids(vec![
            m("span").class("name-tag-who").kids(vec![
                Node::icon("robot").class("ph ph-robot"),
                m("span").text(handle_expr),
            ]),
            // Immediately after the name, reading as one phrase — not pushed
            // to the far edge, where it would read as a separate column.
            m("span")
                .class("name-tag-when")
                .show(ts_expr)
                .text(&format!("'· ' + $store.live.relAgo({ts_expr})")),
        ])
}

/// A card that starts shut and opens on a click anywhere on it.
///
/// Used by every row that records *what the model was given or produced* —
/// the system prompt, the prompt as the agent received it, each tool call and
/// result, the reasoning, the engagement decision. All of them are evidence
/// rather than conversation: essential when an answer needs explaining,
/// noise the rest of the time. So they collapse to one line and the whole
/// card is the toggle, since a line this small has no room for a separate
/// affordance.
///
/// `summary` is what shows on that one line when shut.
fn fold_card(
    v: &str,
    kind: &str,
    class: &str,
    icon: &str,
    label: &str,
    summary: Option<&str>,
    body: Node,
) -> Node {
    let mut header = vec![
        Node::icon(icon).class(&format!("ph ph-{icon}")),
        m("span").class("fold-label").child(lit(label)),
    ];
    if let Some(expr) = summary {
        header.push(m("span").class("fold-note").text(expr));
    }
    m("div")
        .class(&format!("fold-card {class}"))
        .iff(&format!("{v}.kind === '{kind}'"))
        .bind("class", &format!("({v}.expanded ? '' : 'is-collapsed')"))
        .attr("title", "Click to expand or collapse")
        .on("click", &format!("$store.live.toggleCard({v}.id)"))
        .kids(vec![
            m("div").class("fold-head").kids(header),
            body,
        ])
}

/// The per-kind message cards, over whichever expression holds the row.
///
/// Factored out because the transcript now renders rows from two places —
/// the flat message list and the leaves of a collapsed thread — and a second
/// copy of these classes is a second place for the design guide and the
/// running app to drift apart. `v` is the expression naming the row (`m`, or
/// `t.message` inside a thread).
fn card_kids(v: &str) -> Vec<Node> {
    vec![
        m("div")
            .class("bubble-user")
            .iff(&format!("{v}.kind === 'user'"))
            .kids(vec![
                m("div").text(&format!("{v}.text")),
                // Fork from this message. It only moves the cursor — the
                // branch you are on now is not touched, and the Branches bar
                // above brings it straight back. That is why this can be a
                // one-click action with no confirmation.
                m("button")
                    .class("bubble-fork")
                    .attr("title", "Branch from here — the current answer is kept")
                    .on("click", &format!("$store.live.forkAt({v}.eventId)"))
                    .child(Node::icon("git-branch").class("ph ph-git-branch")),
            ]),
        // Pre-rendered in the store. Safe for `x-html` because the
        // renderer escapes the source before emitting any tag, so every tag
        // here is one it wrote (see helpers::markdown_to_html).
        m("div")
            .class("bubble-assistant md-body")
            .iff(&format!("{v}.kind === 'assistant'"))
            .kids(vec![
                m("div").inner_html(&format!("{v}.html || {v}.text")),
                // Below the answer, not above it: a byline. You read what was
                // said, then who said it — and in a group that attribution is
                // the only thing telling three agents' bubbles apart.
                // Whenever there is one — not only in a group. A single
                // agent still has a name, a colour and a time, and the
                // byline is where the conversation says who is talking.
                name_tag(
                    &format!("{v}.handle"),
                    &format!("{v}.ts"),
                    &format!("{v}.handle"),
                ),
            ]),
        m("div")
            .class("bubble-error")
            .iff(&format!("{v}.kind === 'error'"))
            .text(&format!("{v}.text")),
        // The app talking, not a participant: a slash command's answer, or
        // the server saying nobody is here to reply.
        m("div")
            .class("bubble-system")
            .iff(&format!("{v}.kind === 'system'"))
            .kids(vec![
                Node::icon("info").class("ph ph-info"),
                m("span").text(&format!("{v}.text")),
            ]),
        // The user's own message gets a time but no name — it is theirs, and
        // in a conversation of one human a byline naming them is noise.
        // Right-aligned under the bubble, which the block's `flex-end`
        // handles: both share the block's right edge.
        m("div")
            .class("name-tag name-tag-bare")
            .iff(&format!("{v}.kind === 'user' && {v}.ts"))
            .child(
                m("span")
                    .class("name-tag-when")
                    .text(&format!("$store.live.relAgo({v}.ts)")),
            ),
        // Same pre-rendered markdown as the answer card; safe for `x-html`
        // because the renderer escapes before it emits any tag.
        fold_card(
            v,
            "reasoning",
            "fold-reasoning",
            "brain",
            "Reasoning",
            None,
            m("div")
                .class("fold-body md-body")
                .inner_html(&format!("{v}.html || {v}.text")),
        ),
        // --- the turn's paperwork ---------------------------------------
        //
        // What the model was handed, kept apart from what the user typed:
        // "why did it answer that?" is unanswerable without them, and
        // unaskable when they are mixed together.
        fold_card(
            v,
            "system_prompt",
            "fold-system",
            "scroll",
            "System prompt",
            None,
            m("pre").class("fold-body fold-pre").text(&format!("{v}.text")),
        ),
        fold_card(
            v,
            "agent_prompt",
            "fold-prompt",
            "arrow-line-right",
            "User prompt",
            // How many prior turns went with it, but not its size: the first
            // line of the prompt itself is the useful preview, and a byte
            // count was taking room from it to say something nobody acts on.
            Some(&format!(
                "({v}.historyTurns ? {v}.historyTurns + ' prior' : '')"
            )),
            m("pre").class("fold-body fold-pre").text(&format!("{v}.text")),
        ),
        fold_card(
            v,
            "decision",
            "fold-decision",
            "funnel",
            "Engagement",
            // The decider's own words for the outcome, which say *why* —
            // "only 33% sure (needs 60%)" answers "why was it quiet", where
            // a bare percentage leaves you to guess the bar.
            Some(&format!(
                "(({v}.speak ? 'will reply' : 'stays quiet') + ' · ' + ({v}.reason || ({v}.confidence ? Math.round({v}.confidence * 100) + '%' : '?')))"
            )),
            m("pre")
                .class("fold-body fold-pre")
                .text(&format!("JSON.stringify({v}.verdict, null, 2)")),
        ),
        fold_card(
            v,
            "tool_call",
            "fold-tool-call",
            "wrench",
            "Tool call",
            Some(&format!("{v}.name")),
            m("pre")
                .class("fold-body fold-pre")
                .text(&format!("{v}.args ? JSON.stringify({v}.args, null, 2) : ''")),
        ),
        // A denied or failed call must not wear the green rule.
        m("div")
            .class("fold-card fold-tool-result")
            .iff(&format!("{v}.kind === 'tool_result'"))
            .bind(
                "class",
                &format!(
                    "(({v}.expanded ? '' : 'is-collapsed')                       + ({v}.ok === false || {v}.denied ? ' err' : ' ok'))"
                ),
            )
            .attr("title", "Click to expand or collapse")
            .on("click", &format!("$store.live.toggleCard({v}.id)"))
            .kids(vec![
                m("div").class("fold-head").kids(vec![
                    m("i").bind(
                        "class",
                        &format!(
                            "'ph ' + ({v}.ok === false || {v}.denied ? 'ph-prohibit' : 'ph-check-circle')"
                        ),
                    ),
                    m("span").class("fold-label").text(&format!("{v}.name")),
                    m("span")
                        .class("fold-note")
                        .text(&format!("({v}.denied ? 'denied' : {v}.ok === false ? 'failed' : 'ok')")),
                ]),
                m("pre").class("fold-body fold-pre").text(&format!("{v}.text || ''")),
            ]),
    ]
}

/// The threaded history: the memory tree's view, oldest first.
///
/// One keyed `x-for` over a flat list whose rows carry their own `depth`.
/// A row is either a **thread** (a summary standing in for everything under
/// it, openable one level at a time) or a **leaf** — a real message, drawn
/// with exactly the same cards the flat transcript uses.
pub fn messages_threaded() -> Node {
    let st = "$store.live";
    m("div")
        .class("messages")
        // Deliberately NOT keyed on `threads.length`: opening a thread half
        // way up the history must not drag the reader to the bottom. Only a
        // live turn pins, which is the one case they asked to follow.
        .effect(&format!(
            "({st}.streamText, {st}.waiting, {st}.keepPinned($el))"
        ))
        .on("scroll", &format!("{st}.onScroll($event)"))
        .kids(vec![
            m("div")
                .class("thread-row")
                .for_each(&format!("t in {st}.threads"))
                .key("t.id")
                // The indent is the depth you have descended to.
                .bind("style", "('--d: ' + t.depth)")
                .kids(vec![
                    // a collapsed stretch of conversation
                    m("div")
                        .iff("!t.leaf")
                        .class("thread-slot")
                        .child(thread::row()),
                    // the bottom of the tree: the original message
                    m("div")
                        .iff("t.leaf && t.message")
                        .bind(
                            "class",
                            "('msg-block ' + (t.message.kind === 'user' ? 'msg-user' : ''))",
                        )
                        .kids(card_kids("t.message")),
                ]),
            // The turn in flight still belongs at the bottom, threaded or not.
            m("div")
                .class("msg-block")
                .show(&format!("{st}.streamText"))
                .child(
                    m("div")
                        .class("bubble-assistant md-body streaming")
                        .inner_html(&format!("{st}.streamHtml || {st}.streamText")),
                ),
            m("div")
                .class("msg-block thinking-block")
                .show(&format!("{st}.waiting"))
                .child(thinking::thinking()),
        ])
}

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
            .class("turn-row")
            .for_each(&format!("m in {st}.messages"))
            .key("m.id")
            // A turn is one inference request, and its rows are drawn as one
            // run down a rail. The first and last row of a run round the
            // line's ends; a row with no turn — what the user typed — is
            // indented to the same column but gets no rail and no dot.
            .bind(
                "class",
                "('' + (m.turn ? 'in-turn' : 'no-turn')                   + (m.turnFirst ? ' turn-first' : '')                   + (m.turnLast ? ' turn-last' : ''))",
            )
            // Everything in an agent's turn is drawn in that agent's colour.
            .bind("style", &format!("('--agent-accent: ' + {st}.accentOf(m.handle || ''))"))
            .kids(vec![
                m("div").class("turn-rail").child(m("span").class("turn-dot")),
                m("div")
                    .class("msg-block")
                    .bind("class", "(m.kind === 'user' ? 'msg-user' : '')")
                    .kids(card_kids("m")),
            ]),

        // --- the turn in flight ---------------------------------------------
        // Streamed deltas live in their own store strings and are swapped for
        // a real row when the sealed card arrives, so a 60-per-second token
        // append never rebuilds the message list.
        m("div")
            .class("turn-row in-turn")
            .show(&format!("{st}.streamReasoning"))
            .bind("style", &format!("('--agent-accent: ' + {st}.accentOf({st}.streamHandle))"))
            .kids(vec![
            m("div").class("turn-rail").child(m("span").class("turn-dot")),
            m("div").class("msg-block").child(
                // The turn in flight has no row of its own, so its open/shut
                // state is a store flag and the toggle is called with no id.
                // The same `fold-*` classes the sealed card uses. It had the
                // skin's older `reasoning-*` ones, which neither the
                // collapse rules nor `--agent-accent` reach — so the live
                // card showed up expanded and in the skin's hardcoded
                // purple, and only snapped to the agent's colour once the
                // sealed row replaced it.
                m("div")
                    .class("fold-card fold-reasoning")
                    .bind("class", &format!("({st}.streamReasoningOpen ? '' : 'is-collapsed')"))
                    .attr("title", "Click to expand or collapse")
                    .on("click", &format!("{st}.toggleCard('')"))
                    .kids(vec![
                    m("div").class("fold-head").kids(vec![
                        Node::icon("brain").class("ph ph-brain"),
                        m("span").class("fold-label").child(lit("Reasoning")),
                    ]),
                    m("div")
                        .class("fold-body md-body")
                        .inner_html(&format!(
                            "{st}.streamReasoningHtml || {st}.streamReasoning"
                        )),
                ]),
            )]),
        // The in-flight bubble renders markdown as it arrives. The renderer
        // is prefix-safe, so an unterminated fence, a half-typed `**` or a
        // partial link render as themselves instead of leaking broken tags.
        m("div")
            .class("turn-row in-turn turn-last")
            .show(&format!("{st}.streamText"))
            .bind("style", &format!("('--agent-accent: ' + {st}.accentOf({st}.streamHandle))"))
            .kids(vec![
                m("div").class("turn-rail").child(m("span").class("turn-dot")),
                m("div").class("msg-block").child(
                    m("div")
                        .class("bubble-assistant md-body streaming")
                        .inner_html(&format!("{st}.streamHtml || {st}.streamText")),
                ),
            ]),
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

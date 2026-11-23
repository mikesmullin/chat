//! `session` — the middle column: the transcript for one session, a
//! stack of message cards in `msg-block` rows (user rows right-aligned)
//! plus the thinking indicator.
//!
//! Composed literally from the card builders, so each part renders
//! identically alone and in the transcript. Depth 5: user + assistant +
//! tool call + tool response + thinking.

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::js;
use wasm_bindgen::JsValue;

use crate::components::{
    approval_card, assistant_card, decision_card, error_card, prompt_card, queued_card,
    reasoning_card, system_card,
    thinking, thread, tool_call_card, tool_response_card, user_card,
};
use crate::css;

pub const CSS: &str = r#"
/* ---- the scrolling transcript ----
   Out of `web/styles.css` with the demo page it used to share. */
.mjs-chat .messages {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 20px 24px 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  overscroll-behavior: contain;
}
.mjs-chat.mode-sidebar .messages { padding: 10px; }

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
   The block shrinks to its contents and stays against the right edge. */
:is(#sb, #home) .turn-row .msg-block.msg-user {
  align-items: flex-end;
  flex: 0 1 auto;
  /* The skin caps the block at `min(720px, 85%)`. In a flex row that cap
     leaves free space which the block, sitting at the start, does not use —
     so the bubble right-aligned to 85% across rather than to the row's edge.
     `margin-left: auto` eats the free space and moves the block right. */
  margin-left: auto;
}
/* Stretch the bubble to the block, so even a very short message shares
   its left edge with the timestamp and action. */
:is(#sb, #home) .turn-row .msg-block.msg-user .bubble-user {
  width: auto;
  max-width: 100%;
  align-self: stretch;
}
/* The same message from *someone else* in a shared room (PLAN44 A6): the
   mirror of the rule above — left-aligned, hugging the rail's side. */
:is(#sb, #home) .turn-row .msg-block.msg-user.msg-from-other {
  margin-left: 0;
  margin-right: auto;
  align-items: flex-start;
}
/* The byline hugs its text rather than stretching across the column, and
   sits the same distance below the answer as any two events sit apart. */
:is(#sb, #home) .turn-row .name-tag { align-self: flex-start; margin-top: 4px; }

/* Putting the answer in its own div (so the byline can be a sibling) moved
   the rendered markdown one level down, which put it out of reach of the
   skin's `.md-body > :first-child { margin-top: 0 }`. The first paragraph
   kept its `0.55em` top margin and the last kept its bottom one — about 7px
   of padding at each end that nobody asked for, and the reason the first
   line sat below its dot. */
:is(#sb, #home) .turn-row .bubble-assistant > div > :first-child { margin-top: 0; }
:is(#sb, #home) .turn-row .bubble-assistant > div > :last-child { margin-bottom: 0; }

/* A failed or denied tool result is not in the agent's colour: the point is
   that something went wrong, which a tint cannot say. */

/* Collapsed: one line, no chrome at all — just the tinted text. */
/* The body may be rendered markdown, so it is full of block elements. One
   line only really is one line once they are inline and their own
   white-space rules (a <pre> keeps its newlines) give way to the clamp. */
/* A <br> would break the line regardless of white-space. */

/* When, in the dim the rest of the chrome uses: it is a footnote to the
   byline, not a second thing being announced. */

/* demo framing: transcript column size; the sidebar divider is an edge
   against a neighbouring column, not a property of the column */
.session-demo { width: min(640px, 100%); height: 560px; }
.session-demo .mjs-chat.mode-sidebar { border-left: 0; }
.session-demo .mjs-chat { height: 100%; }
"#;

/// the transcript column: messages in a `section.main`.
pub fn transcript(st: &str) -> Node {
    m("section").class("main").child(messages(st))
}

/// the transcript inside the chat shell.
pub fn session(st: &str) -> Node {
    m("div")
        .class("mjs-chat mode-sidebar")
        .attr("aria-label", "Session")
        .child(transcript(st))
}

/// standalone demo — the transcript the app draws, over the sample
/// conversation. Not a second transcript: the same one, different store.
pub fn demo() -> Node {
    m("div")
        .class("session-demo")
        .child(session(crate::components::samples::EXPR))
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
/// The per-kind message cards, over whichever expression holds the row.
///
/// Factored out because the transcript now renders rows from two places —
/// the flat message list and the leaves of a collapsed thread — and a second
/// copy of these classes is a second place for the design guide and the
/// running app to drift apart. `v` is the expression naming the row (`m`, or
/// `t.message` inside a thread).
fn card_kids(st: &str, v: &str) -> Vec<Node> {
    // Every row kind, each from the component that owns it. Nothing is drawn
    // here: this function decides *which* cards a transcript can hold, and
    // each card decides what it looks like and which `kind` it answers to.
    // That is the whole arrangement — one definition per card, composed in
    // one place, so the design guide and the running app cannot disagree.
    let mut out = Vec::new();
    out.extend(user_card::parts(st, v));
    out.extend(assistant_card::parts(st, v));
    out.extend(error_card::parts(st, v));
    out.extend(system_card::parts(st, v));
    out.extend(queued_card::parts(st, v));
    out.extend(reasoning_card::parts(st, v));
    // The turn's paperwork: what the model was handed, kept apart from what
    // the user typed. "Why did it answer that?" is unanswerable without
    // them, and unaskable when they are mixed together.
    out.extend(prompt_card::system_parts(st, v));
    out.extend(prompt_card::user_parts(st, v));
    out.extend(decision_card::parts(st, v));
    out.extend(tool_call_card::parts(st, v));
    out.extend(tool_response_card::parts(st, v));
    // The one row that is a question: a tool the allowlist stopped.
    out.extend(approval_card::parts(st, v));
    out
}

const SIMPLE_HIDDEN: &[&str] = &["system", "system_prompt", "agent_prompt", "reasoning", "tool_call", "tool_result"];

fn visible_event(v: &str) -> String {
    let kinds = SIMPLE_HIDDEN.iter().map(|kind| format!("'{kind}'")).collect::<Vec<_>>().join(", ");
    format!("($store.mode.advanced || {v}.commandNotice || ![{kinds}].includes({v}.kind))")
}

thread_local! {
    static SIMPLE_EDGES: std::cell::RefCell<Option<(JsValue, std::collections::HashMap<String, (bool, bool)>)>> = const { std::cell::RefCell::new(None) };
}

/// Compute the first and last visible event of each turn. Cache by the
/// transcript array so token redraws do not rescan the whole conversation.
pub(crate) fn turn_edges(this: JsValue, args: js_sys::Array) -> JsValue {
    let row = js::arg(&args, 0);
    if js::arg(&args, 1).as_bool().unwrap_or(false) {
        return JsValue::from_str(&format!("{}{}", if js::get_bool(&row, "turnFirst") { " turn-first" } else { "" }, if js::get_bool(&row, "turnLast") { " turn-last" } else { "" }));
    }
    let messages = js::get(&this, "messages");
    let id = js::get_str(&row, "id");
    SIMPLE_EDGES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.as_ref().is_none_or(|(old, _)| !js_sys::Object::is(old, &messages)) {
            let mut edges = std::collections::HashMap::new();
            let mut group: Vec<String> = Vec::new();
            let flush = |group: &mut Vec<String>, edges: &mut std::collections::HashMap<String, (bool, bool)>| {
                let n = group.len();
                for (i, id) in group.drain(..).enumerate() { edges.insert(id, (i == 0, i + 1 == n)); }
            };
            for item in js_sys::Array::from(&messages).iter() {
                if !js::get(&item, "turn").is_truthy() { flush(&mut group, &mut edges); continue; }
                if js::get_bool(&item, "turnFirst") { flush(&mut group, &mut edges); }
                if !SIMPLE_HIDDEN.contains(&js::get_str(&item, "kind").as_str()) {
                    group.push(js::get_str(&item, "id"));
                }
                if js::get_bool(&item, "turnLast") { flush(&mut group, &mut edges); }
            }
            flush(&mut group, &mut edges);
            *cache = Some((messages, edges));
        }
        let (first, last) = cache.as_ref().and_then(|(_, edges)| edges.get(&id)).copied()
            .unwrap_or((js::get_bool(&row, "turnFirst"), js::get_bool(&row, "turnLast")));
        JsValue::from_str(&format!("{}{}", if first { " turn-first" } else { "" }, if last { " turn-last" } else { "" }))
    })
}

/// Streamed rows are not in `messages` yet. Count the visible rows in the
/// current turn so an answer arriving on its own has no dangling segment.
pub(crate) fn stream_edges(this: JsValue, args: js_sys::Array) -> JsValue {
    let reasoning = js::arg_str(&args, 0) == "reasoning";
    let advanced = js::arg(&args, 1).as_bool().unwrap_or(false);
    let rows = js_sys::Array::from(&js::get(&this, "messages"));
    let mut preceding = false;
    for row in rows.iter().collect::<Vec<_>>().into_iter().rev() {
        let kind = js::get_str(&row, "kind");
        if kind == "user" { break; }
        if js::get(&row, "turn").is_truthy() && (advanced || !SIMPLE_HIDDEN.contains(&kind.as_str())) {
            preceding = true;
            break;
        }
    }
    if js_sys::Array::from(&js::get(&this, "approvals")).length() > 0 { preceding = true; }
    if !reasoning && advanced && !js::get_str(&this, "streamReasoning").is_empty() { preceding = true; }
    let last = !reasoning || js::get_str(&this, "streamText").is_empty();
    JsValue::from_str(&format!("{}{}", if preceding { "" } else { " turn-first" }, if last { " turn-last" } else { "" }))
}

/// One row of the transcript: the rail with its dot, and the message block
/// beside it.
///
/// Public because every card's demo mounts itself in it — a card page in the
/// design guide shows the card exactly as the transcript indents, tints and
/// rails it, rather than floating free in a way it never appears.
pub fn row(st: &str, v: &str, parts: Vec<Node>) -> Node {
    m("div")
        .class("turn-row")
        // A turn is one inference request, and its rows are drawn as one run
        // down a rail. The first and last row of a run round the line's ends;
        // a row with no turn — what the user typed — is indented to the same
        // column but gets no rail and no dot.
        .bind(
            "class",
            &format!(
                "(({v}.turn ? 'in-turn' : 'no-turn') + {st}.turnEdges({v}, $store.mode.advanced))"
            ),
        )
        // Everything in an agent's turn is drawn in that agent's colour.
        .bind("style", &format!("('--agent-accent: ' + {st}.accentOf({v}.handle || ''))"))
        .kids(vec![
            m("div").class("turn-rail").child(m("span").class("turn-dot")),
            m("div")
                .class("msg-block")
                .bind("class", &format!("(({v}.kind === 'user' ? 'msg-user' : '') + (({v}.from && {v}.from !== $store.ident.name) ? ' msg-from-other' : ''))"))
                .kids(parts),
        ])
}

/// The threaded history: the memory tree's view, oldest first.
///
/// One keyed `x-for` over a flat list whose rows carry their own `depth`.
/// A row is either a **thread** (a summary standing in for everything under
/// it, openable one level at a time) or a **leaf** — a real message, drawn
/// with exactly the same cards the flat transcript uses.
pub fn messages_threaded(st: &str) -> Node {
    m("div")
        .class("messages")
        // Deliberately NOT keyed on `threads.length`: opening a thread half
        // way up the history must not drag the reader to the bottom. Only a
        // live turn pins, which is the one case they asked to follow.
        .effect(&format!(
            "({st}.streamText, {st}.waiting, {st}.keepPinned($el))"
        ))
        .on("scroll", &format!("{st}.onScroll($event)"))
        // A markdown image is markup inside innerHTML — the click is
        // delegated here so any attachment opens the lightbox.
        .on(
            "click",
            "$event.target.tagName === 'IMG' && $store.lightbox.open($event.target.src)",
        )
        .kids(vec![
            m("div")
                .class("thread-row")
                .for_each(&format!("t in {st}.threads"))
                .key("t.id")
                .show(&format!("(!t.leaf || !t.message || {})", visible_event("t.message")))
                // The indent is the depth you have descended to.
                .bind("style", "('--d: ' + t.depth)")
                .kids(vec![
                    // a collapsed stretch of conversation
                    m("div")
                        .iff("!t.leaf")
                        .class("thread-slot")
                        .child(thread::row(st)),
                    // the bottom of the tree: the original message
                    m("div")
                        .iff("t.leaf && t.message")
                        .bind(
                            "class",
                            "('msg-block ' + (t.message.kind === 'user' ? 'msg-user' : '') + ((t.message.from && t.message.from !== $store.ident.name) ? ' msg-from-other' : ''))",
                        )
                        .kids(card_kids(st, "t.message")),
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
                .show(&format!("{st}.waiting && !{st}.stopping && !{st}.stopUnconfirmed"))
                .child(thinking::thinking()),
            m("div").class("thinking").show(&format!("{st}.waiting && ({st}.stopping || {st}.stopUnconfirmed)"))
                .text(&format!("({st}.stopping ? 'Stopping…' : 'Stop not confirmed. Try again.')")),
        ])
}

pub fn messages(st: &str) -> Node {
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
        // A markdown image is markup inside innerHTML — the click is
        // delegated here so any attachment opens the lightbox.
        .on(
            "click",
            "$event.target.tagName === 'IMG' && $store.lightbox.open($event.target.src)",
        )
        .kids(vec![
        // --- the transcript ------------------------------------------------
        row(st, "m", card_kids(st, "m"))
            .for_each(&format!("m in {st}.messages"))
            .key("m.id")
            .show(&visible_event("m")),

        // --- what the turn is waiting on -------------------------------------
        // A gated tool call, asking. Its own list rather than a row in
        // `messages`, because it is not part of the transcript yet: the
        // server writes it there once it has been answered, and until then
        // it belongs to the turn in flight.
        row(st, "ap", approval_card::parts(st, "ap"))
            .for_each(&format!("ap in {st}.approvals"))
            .key("ap.id"),

        // --- the turn in flight ---------------------------------------------
        // Streamed deltas live in their own store strings and are swapped for
        // a real row when the sealed card arrives, so a 60-per-second token
        // append never rebuilds the message list.
        m("div")
            .class("turn-row in-turn")
            .bind("class", &format!("{st}.streamEdges('reasoning', $store.mode.advanced)"))
            .show(&format!("$store.mode.advanced && {st}.streamReasoning"))
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
            .class("turn-row in-turn")
            .bind("class", &format!("{st}.streamEdges('answer', $store.mode.advanced)"))
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
            .show(&format!("{st}.waiting && !{st}.stopping && !{st}.stopUnconfirmed"))
            .child(thinking::thinking()),
        m("div").class("thinking").show(&format!("{st}.waiting && ({st}.stopping || {st}.stopUnconfirmed)"))
            .text(&format!("({st}.stopping ? 'Stopping…' : 'Stop not confirmed. Try again.')")),
    ])
}

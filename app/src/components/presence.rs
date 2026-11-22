//! `presence` — who is in the conversation, as a cluster of discs.
//!
//! A chat of two is you and one small circle. A chat of nine is you, five
//! `+4`. **The cluster is the component, not the disc**: what has to be
//! decided — who comes first, where it stops, what an empty room says — are
//! properties of the set, not of one seat.
//!
//! ## No ring, no name
//!
//! Presence means *seated in this conversation*, and each disc already says
//! whose seat it is: the agent's own glyph in the agent's own colour. A ring
//! around that is chrome, and a name beside it repeats what the transcript
//! says under everything they said. So a seat is the disc alone, and the name
//! lives on the disc's `title` for hover.
//!
//! An unseated agent keeps its messages, so it keeps its place in the room
//! that holds them; `roster` said so with a struck-through chip. A disc
//! cannot be struck through, so it empties and its glyph dims rather than
//! the seat vanishing.
//!
//! ## Identity is already owned elsewhere
//!
//! A seat wears `agents.accent_color` and `agents.icon` — the same two fields
//! the rail row, the transcript byline and the agent editor's header draw. A
//! seat is not a new idea of what an agent looks like; it is that idea at
//! 22px.
//!
//! ## Order
//!
//! You first, always on top: the reader opens the row and sits above it.
//! After that `(active, seat)`: people in the room first, invite order
//! within each group.
//! The server already returns seats `ORDER BY seat ASC, id ASC`
//! (`conv_store::participants_in`), so the second half is free.
//!
//! ## No mode word
//!
//! There used to be a `roster-mode` word.
//! It named which turn path the server would take. It went: next to a
//! disc stack it read as a caption for something already visible. The
//! cluster moved into its place at the row right edge instead.

use rustex1_kit::h::{m, Node};

use crate::components::rail_head;
use crate::css;

/// The colour a seat falls back to when its agent has none — the same default
/// `agents::DEFAULT_ACCENT` gives the rail and the byline.
pub const DEFAULT_ACCENT: &str = "#a78bfa";

/// The current user has the same identity in the roster and transcript.
pub const USER_ICON: &str = "user";
pub const USER_ACCENT: &str = "#aaaaaa";
pub fn user_name(st: &str) -> String { format!("({st}.userName || 'You')") }

pub const CSS: &str = r#"
:is(#sb, #home) .presence { display: flex; align-items: center; gap: 8px; }
:is(#sb, #home) .presence-seats {
  display: flex; align-items: center; flex-wrap: wrap; min-width: 0;
}
/* The stack: every seat after the first pulls 6px over its neighbour, and a
   2px cut-out in the surface colour keeps each disc readable. There is no
   unseat button on a seat — removal happens in the session editor dialog —
   so a seat is exactly one disc and the overlap costs no affordance. */
/* First-on-top: every seat carries an explicit z-index, highest on the left,
   because DOM order would stack them the other way (later paints on top).
   The value descends from the row seat count in the guide and from
   (100 - i) under x-for - only the direction matters, not the base. */
:is(#sb, #home) .presence-seat { position: relative; display: inline-flex; }
:is(#sb, #home) .presence-seat + .presence-seat { margin-left: -6px; }

/* One seat: a 22px disc carrying the agent's own glyph in its own colour.
   No ring, no name: the cluster is the indicator, and the two things an agent
   already owns (its colour, its glyph) are what tell six of them apart at a
   glance. The name is the disc's `title`, and it is in the transcript beside
   everything they said. */
:is(#sb, #home) .seat-disc {
  width: 22px; height: 22px; flex: 0 0 auto;
  display: inline-flex; align-items: center; justify-content: center;
  border-radius: 50%;
  background: color-mix(in srgb, var(--agent-accent) 14%, var(--panel-2));
  color: var(--agent-accent);
  box-shadow: 0 0 0 2px var(--panel);
}
:is(#sb, #home) .seat-disc > i { font-size: 12px; line-height: 1; }

/* Unseated: the disc empties and the glyph dims. It stays in the cluster
   because its messages are still attributed to it. Opaque, not transparent:
   a transparent disc would show its neighbour through the overlap. */
:is(#sb, #home) .seat-disc.is-off { background: var(--panel); color: var(--gray-dim); }

/* `+N`: a member of the cluster, not a control beside it — it is one of the
   seats, just not drawn. Mono like every other count in the guide. It stacks
   and cuts out exactly like a seat. */
:is(#sb, #home) .presence-more {
  position: relative;
  width: 22px; height: 22px; flex: 0 0 auto;
  display: inline-flex; align-items: center; justify-content: center;
  border: 0; border-radius: 50%;
  background: var(--panel-2); color: var(--muted);
  font-family: var(--mono); font-size: 10px; line-height: 1;
  cursor: pointer;
  margin-left: -6px;
  box-shadow: 0 0 0 2px var(--panel);
}
:is(#sb, #home) .presence-more:hover { color: var(--accent-soft); background: var(--accent-glow); }

/* the guide's column of variants */
.presence-demo { display: flex; flex-direction: column; gap: 22px; }
"#;

/// One seat.
///
/// Every argument is an m-js **expression**, so the guide's static sample
/// (`"'flask'"`, `"'#0091ff'"`, `"false"`) and a store-bound row
/// (`"s.icon"`, `"s.accentColor"`, `"!s.active"`) are built by the *same*
/// function. There is no second drawing of a seat.
///
/// `accent` is wrapped in a fallback here rather than trusting the caller,
/// because the value lands in a `style` attribute: an empty
/// `--agent-accent: ` makes every declaration that reads it invalid at
/// computed-value time, which falls back to *inherited* — not to the `var()`
/// fallback — so a seat would wear its neighbour's colour.
///
/// `handle` is never drawn — it becomes the disc's `title`, so the cluster
/// still answers "who is that?" on hover.
///
/// There is deliberately no unseat control on a seat: removal happens in the
/// session editor dialog, so the cluster never has to make room for one.
/// `z` is the stacking position, highest on the left - a static number in the
/// guide, `(100 - i)` under `x-for`. Without it the overlap would stack
/// last-on-top and the row would read right-to-left.
pub fn seat(handle: &str, icon: &str, accent: &str, off: &str, z: &str) -> Node {
    let style = format!("('--agent-accent: ' + (({accent}) || '{DEFAULT_ACCENT}'))");
    let disc = m("span")
        .class("seat-disc")
        .bind("class", &format!("({off} ? 'is-off' : '')"))
        .bind("style", &style)
        .bind("title", handle)
        .child(Node::icon_dyn(icon));
    m("span")
        .class("presence-seat")
        .bind("style", &format!("('z-index:' + ({z}))"))
        .child(disc)
}

/// `+N` — the seats the cluster stopped drawing. Hidden while there is
/// nothing behind it: the live count (`others.length - 5`) is zero or
/// negative in a small room.
pub fn more(count: &str) -> Node {
    m("button")
        .class("presence-more")
        .attr("type", "button")
        .show(&format!("(({count}) > 0)"))
        .text(&format!("'+' + ({count})"))
}

/// The discs themselves.
pub fn seats(kids: Vec<Node>) -> Node {
    m("div").class("presence-seats").kids(kids)
}

/// The row: the cluster, then the session edit control beside it.
///
/// That control is `rail_head::edit` - the same 22px ghost button as the
/// rail add, with a pencil. It belongs to the session header, not to the
/// indicator: it edits the session the cluster belongs to.
/// `add` is `(tooltip, click expression)` — the same shape `rail_head::head`
/// takes, and the same button: it is `rail_head`'s, not a second one.
pub fn row(cluster: Node, add: Option<(&str, &str)>) -> Node {
    let mut kids = vec![cluster];
    if let Some((tip, on)) = add {
        kids.push(rail_head::edit(tip, on));
    }
    m("div").class("presence").kids(kids)
}

/// An empty room shows its one member: you. A cluster of nothing is
/// indistinguishable from a component that failed to render, and the person
/// reading an empty conversation is never nobody. All three are expressions,
/// so the guide passes literals and the live roster will pass the settings
/// name. The user wears their own neutral accent.
pub fn empty(handle: &str, icon: &str, accent: &str) -> Node {
    seats(vec![seat(handle, icon, accent, "false", "1")])
}

/// The guide's variants: truncated, short, one unseated, empty.
pub fn demo() -> Node {
    // The seven agents this project actually defines, so the guide shows the
    // room rather than a plausible one.
    let agents: [(&str, &str, &str); 7] = [
        ("researcher", "flask", "#0091ff"),
        ("joker", "mask-happy", "#be5cff"),
        ("fish", "fish", "#3af89f"),
        ("homie", "house", "#de4a91"),
        ("phony", "phone", "#2bc5c2"),
        ("bowser", "browser", "#96949e"),
        ("dummy", "lego-smiley", "#eeff00"),
    ];
    let inert = "$store.samples.noop()";
    // You, first and on top in every row. Takes the row top z so the
    // agents descending numbers never have to know the count.
    let you = |z: &str| seat("'Bob'", &format!("'{USER_ICON}'"), &format!("'{USER_ACCENT}'"), "false", z);
    let built = |n: usize| -> Vec<Node> {
        agents[..n]
            .iter()
            .enumerate()
            .map(|(k, (h, i, c))| {
                seat(
                    &format!("'{h}'"),
                    &format!("'{i}'"),
                    &format!("'{c}'"),
                    "false",
                    &format!("{}", n - k),
                )
            })
            .collect()
    };

    // 7 seated, 5 drawn: the truncation case. You plus five, then `+2`.
    let mut seven = vec![you("6")];
    seven.extend(built(5));
    seven.push(more("2"));

    let mut three = vec![you("4")];
    three.extend(built(3));

    // A seat that has been unseated: emptied disc, dimmed glyph, still in the row.
    let mut mixed = vec![you("5")];
    mixed.extend(built(3));
    mixed.push(seat("'bowser'", "'browser'", "'#96949e'", "true", "0"));

    m("div").class("presence-demo").kids(vec![
        row(seats(seven), Some(("Edit session", inert))),
        row(seats(three), Some(("Edit session", inert))),
        row(seats(mixed), Some(("Edit session", inert))),
        row(
            empty("'Bob'", &format!("'{USER_ICON}'"), &format!("'{USER_ACCENT}'")),
            Some(("Edit session", inert)),
        ),
    ])
}

pub fn register() {
    css::inject("presence", CSS);
}

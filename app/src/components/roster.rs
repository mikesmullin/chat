//! `roster` — who is in the conversation.
//!
//! A chat of two is you and one disc, right-aligned and otherwise out of
//! the way. The cluster is `presence` markup. There is no mode word: next to a disc stack it read as a
//! caption for something already visible.
//!

use rustex1_kit::h::{m, Node};

use crate::components::{notification_toggle, presence};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .roster {
  display: flex; gap: 6px; align-items: center; flex-wrap: wrap;
  padding: 6px 10px; font-size: 11px;
}
/* The cluster shows you plus the first five seats; the rest live behind `+N`
   in the session editor. Cut in CSS, not with `x-show` on the `x-for` row:
   two directives fighting over one element visibility is exactly the kind
   of bind-time surprise this repo documents to avoid. Scoped to `.roster`
   so the editor full list — same markup under `.se-participants` — is
   never cut. `nth-child` counts every sibling, so the user disc in first
   position is what makes this seven and not six. */
:is(#sb, #home) .roster .presence-seat:nth-child(n+7) { display: none; }
/* The cluster sits at the row right edge, where the mode word used to be. */
:is(#sb, #home) .roster .notify-toggle { margin-left: auto; }
"#;

/// The cluster, right-aligned where the mode word used to be.
///
/// The seats are `presence` markup — the same `seat()` the guide features
/// alone at `/storybook/presence` — so the bar and the guide cannot drift.
/// You first: the reader is never nobody, even alone, and the fallback says
/// what is true until the live store learns the settings name.
pub fn roster(st: &str) -> Node {
    m("div")
        .class("roster")
        .show(&format!("{st}.activeId"))
        .child(notification_toggle::button(st))
        .child(
            presence::row(
                presence::seats(vec![
                    presence::seat(
                        &presence::user_name(st),
                        &format!("'{}'", presence::USER_ICON),
                        &format!("'{}'", presence::USER_ACCENT),
                        "false",
                        &format!("({st}.activeOthers.length + 100)"),
                    ),
                    // The room, newest invite last. Truncation is the CSS
                    // `nth-child` rule above, not `x-show` on this row.
                    // The mark resolves through `iconOf`/`accentOf` — the
                    // same source the transcript byline draws — so a seat
                    // and its bylines can never disagree, and a seated
                    // person wears the person's mark like "You" does.
                    presence::seat(
                        "p.handle",
                        &format!("{st}.iconOf(p.handle)"),
                        &format!("{st}.accentOf(p.handle)"),
                        "!p.active",
                        "(100 - i)",
                    )
                    .for_each(&format!("(p, i) in {st}.activeOthers"))
                    .key("p.id"),
                    // Everyone past the first five lives behind this, in the
                    // session editor — which is what `+N` opens.
                    presence::more(&format!("{st}.activeOthers.length - 5")),
                ]),
                Some(("Edit session", &format!("{st}.openSessionEditor()"))),
            ),
        )
}

pub fn register() {
    css::inject("roster", CSS);
}

//! `uiSessionState` — the state symbol as its own unit: the same small ring
//! that marks agent-menu rows, featured alone at `/storybook/session-state`.
//! Every state wears the one `uiPieRing` shape and the state is the
//! **colour** — done = accent indigo, working = amber, idle = dim gray.
//!
//! The ring is not a spinner: it fills with the **context window's
//! fullness** for that session — in every state — the same quantity the
//! composer's context pie draws, so [`fraction_of`] is the one rule both
//! gauges compute it by.
//!
//! Owns its CSS; `css::inject` puts it in `<head>` on register (updated on
//! every HMR swap). The shared builder `symbol()` is also called by
//! `item_group` rows, so a state symbol renders identically whether it
//! lives alone or inside a composite.

use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

pub const NAME: &str = "uiSessionState";

const CSS: &str = r#"
/* feature demo: the three states side by side (done / working / idle) */
.state-demo { display: flex; align-items: center; gap: 36px; }
/* the widget root must be a flex container so its fixed-size child (the
   14px pie ring) sizes correctly — a plain inline root collapses it to a
   zero-width inline box. */
.ui-state-root {
  display: inline-flex;
  align-items: center;
  flex: 0 0 auto;
}
"#;

/// The one formula for "how full is the context window": shared by the
/// composer's context pie and the working symbol's ring, so two gauges of
/// the same quantity cannot drift apart. `tokens` and `window` are
/// expressions — a conversation row's own numbers, or the live store's.
pub fn ctx_fraction(tokens: &str, window: &str) -> String {
    format!("Math.min(1, ({tokens} || 0) / Math.max(1, ({window} || 128000)))")
}

/// The one mapping from a conversation row to the ring's fill — the row's
/// `totalTokens` over its `contextWindow`, in the shape of [`tone_of`], so
/// every caller reads one rule instead of re-typing it and drifting.
pub fn fraction_of(row: &str) -> String {
    ctx_fraction(
        &format!("{row}.totalTokens"),
        &format!("{row}.contextWindow"),
    )
}

/// The one mapping from a conversation row to the symbol's tooltip —
/// `{{status name}} - {{ctx_used}}/{{ctx_cap}}`: the status the ring shows,
/// then the context counts behind its fill, in token units (`3.2K`, `256K`,
/// `1M`).
pub fn hint_of(row: &str) -> String {
    format!(
        "{tone} + ' - ' + {used} + '/' + {cap}",
        tone = tone_of(row),
        used = tokens(&format!("({row}.totalTokens || 0)")),
        cap = tokens(&format!("({row}.contextWindow || 128000)")),
    )
}

/// One token count spelled in token units — `3.2K`, `256K`, `1M`: at most
/// one decimal, the trailing `.0` dropped, raw below a thousand. Repeats
/// its value expression; the reads behind it are cheap.
fn tokens(value: &str) -> String {
    format!(
        "(({value}) >= 1e6 ? ((({value}) / 1e6).toFixed(1).replace('.0', '') + 'M') \
         : ({value}) >= 1e3 ? ((({value}) / 1e3).toFixed(1).replace('.0', '') + 'K') \
         : ('' + ({value})))"
    )
}

/// The one state symbol — the single source of truth for the per-row glyph.
/// It hosts the widget below rather than redrawing its shapes, so a state
/// symbol is literally the same markup wherever it lives; the tone is
/// normalised to the three [`tone_of`] spells.
pub fn symbol(state: &str) -> Node {
    let tone = match state {
        "done" => "done",
        "working" => "working",
        _ => "idle",
    };
    Node::host(NAME, &[]).bind("tone", &format!("'{tone}'"))
}

/// The one mapping from a conversation row to a state symbol.
///
/// `running` is **working**: a turn is in flight, watch it. A conversation
/// with no turns yet is **idle** — there is nothing on screen to review, and
/// the done colour would claim the opposite. Anything else has an answer
/// waiting, which is what **done** means here: ready to look at, including a
/// turn that ended in an error.
///
/// It takes the row's expression so every caller — the tab strip, the project
/// tree — reads one rule instead of re-typing it and drifting.
pub fn tone_of(row: &str) -> String {
    format!(
        "({row}.status === 'running' ? 'working' : {row}.turns === 0 ? 'idle' : 'done')"
    )
}

/// register the widget + its CSS.
pub fn register() {
    css::inject(NAME, CSS);
    mjs::component(NAME, &def().finish());
}

/// Props (host-provided, with defaults):
/// - `tone` — `done` / `working` / `idle` (default `done`)
/// - `fraction` — the ring's fill, 0..1: how full that session's context
///   window is, in every state. Callers bind [`fraction_of`]; the default
///   is the showcase fill for demos that carry no conversation data
///   (default 0.68)
/// - `title` — the symbol's tooltip; callers bind [`hint_of`] (default "")
fn def() -> Def {
    let mut d = Def::new().named(NAME);
    d.prop_str("tone", "done")
        .prop_num("fraction", 0.68)
        .prop_str("title", "")
        .tree(
            m("span")
                .class("ui-state-root")
                .bind("title", "title")
                .child(
                    // One shape in every state — the ring — and the state
                    // is the colour: done = accent, working = amber, idle =
                    // dim gray. The fill is context-window fullness, not
                    // turn progress.
                    Node::host("uiPieRing", &[])
                        .bind(
                            "color",
                            "(tone === 'done' ? 'var(--accent)' : \
                             tone === 'working' ? 'var(--pastel-amber)' : \
                             'var(--gray-dim)')",
                        )
                        .bind("fraction", "fraction"),
                ),
        );
    d
}

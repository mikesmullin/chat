//! `uiSessionState` — the session-status symbol as its own unit: the bouncing
//! ball. Status only — never a gauge. The state is the **colour** and the
//! **pose**: done = accent indigo, working = amber, idle = dim gray, and
//! idle/done rest on the floor; working is one ball in the air.
//!
//! It is the only symbol the projects session list wears — no tabs, no
//! transcript rows, no agents rail — so every caller renders the same ball
//! and a tree row says exactly one thing. The context window's fill is a
//! different question entirely, and it belongs to the roster's context ring
//! (see [`ctx_fraction`]/[`ctx_hint`], the one rule the ring reads).
//!
//! The ball is CSS-animated, not drawn per frame: the bounce lives in one
//! `@keyframes`, the idle and done poses are the resting state of that same
//! element, and the colour comes from the wrapper's `color` so the whole
//! symbol inherits one token.
//!
//! Owns its CSS; `css::inject` puts it in `<head>` on register (updated on
//! every HMR swap).

use rustex1_kit::h::m;
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

pub const NAME: &str = "uiSessionState";

const CSS: &str = r#"
/* feature demo: the three poses side by side (done / working / idle) */
.state-demo { display: flex; align-items: center; gap: 36px; }
/* the widget root is a 14px flex column over a thin floor line, so the ball
   always measures the same box whether it is resting or in the air */
.ui-state-root {
  position: relative; display: inline-flex; align-items: flex-end; justify-content: center;
  width: 14px; height: 14px; flex: 0 0 auto;
}
/* the floor: a pale line the resting ball (done and idle) sits on. The
   working pose has no floor — the ball is in the air, not resting on anything. */
.ui-state-root::after {
  content: ""; position: absolute; bottom: 1px; left: 2px; right: 2px; height: 1.5px;
  border-radius: 1px; background: currentColor; opacity: .32;
}
.ui-state-root.working::after { content: none; }
.ui-state-root.done { color: var(--accent); }
.ui-state-root.working { color: var(--pastel-amber); }
.ui-state-root.idle { color: var(--gray-dim); }
.state-ball {
  position: relative; z-index: 1;
  width: 9px; height: 9px; border-radius: 50%;
  background: currentColor;
  box-shadow: 0 1px 2px rgba(0, 0, 0, .45);
  margin-bottom: 2px;
}
/* the working pose: the ball lifts off the floor and floats back down */
.ui-state-root.working .state-ball {
  animation: state-bounce 0.755s ease-in-out infinite;
}
@keyframes state-bounce {
  0%, 100% { transform: translateY(0) scale(1.18, 0.82); }
  42%      { transform: translateY(-6px) scale(0.9, 1.1); }
}
@media (prefers-reduced-motion: reduce) {
  .ui-state-root.working .state-ball { animation: none; }
}
"#;

/// The one formula for "how full is the context window": the context ring's
/// fill, so every gauge of the same quantity reads one rule. `tokens` and
/// `window` are expressions — a store's own numbers.
pub fn ctx_fraction(tokens: &str, window: &str) -> String {
    format!("Math.min(1, ({tokens} || 0) / Math.max(1, ({window} || 128000)))")
}

/// The one formula for the context ring's tooltip — the counts alone, in
/// token units (`3.2K of 256K`): the ring is a gauge of fullness, and the
/// status it used to carry lives on the ball. `tokens` and `window` are
/// expressions, like [`ctx_fraction`].
pub fn ctx_hint(tokens: &str, window: &str) -> String {
    format!(
        "'Context ' + {used} + ' of ' + {cap}",
        used = token_hint(&format!("({tokens} || 0)")),
        cap = token_hint(&format!("({window} || 128000)")),
    )
}

/// One token count spelled in token units — `3.2K`, `256K`, `1M`: at most
/// one decimal, the trailing `.0` dropped, raw below a thousand. Repeats
/// its value expression; the reads behind it are cheap.
fn token_hint(value: &str) -> String {
    format!(
        "(({value}) >= 1e6 ? ((({value}) / 1e6).toFixed(1).replace('.0', '') + 'M') \
         : ({value}) >= 1e3 ? ((({value}) / 1e3).toFixed(1).replace('.0', '') + 'K') \
         : ('' + ({value})))"
    )
}

/// The one mapping from a conversation row to a tone — the ball's colour and
/// pose. `running` is **working**: a turn is in flight, watch it. A
/// conversation with no turns yet is **idle** — there is nothing on screen to
/// review, and the done colour would claim the opposite. Anything else has an
/// answer waiting, which is what **done** means here: ready to look at,
/// including a turn that ended in an error.
///
/// It takes the row's expression so every caller — the project tree — reads
/// one rule instead of re-typing it and drifting.
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
/// - `tone` — `done` / `working` / `idle` (default `done`); normalised to
///   the three [`tone_of`] spells, so anything unrecognised reads as `idle`
/// - `title` — the symbol's tooltip: the tone itself, the one thing the ball
///   shows (default "")
fn def() -> Def {
    let mut d = Def::new().named(NAME);
    d.prop_str("tone", "done")
        .prop_str("title", "")
        .tree(
            m("span")
                // The one class binding carries the full list: mjs replaces
                // the class attribute per binding, so a base class is lost.
                .bind("class", "(tone === 'working' ? 'ui-state-root working' : tone === 'idle' ? 'ui-state-root idle' : 'ui-state-root done')")
                .bind("title", "title")
                .child(m("span").class("state-ball")),
        );
    d
}

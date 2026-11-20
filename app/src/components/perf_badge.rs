//! `uiPerfBadge` — m-js's own draw instrumentation, on the page.
//!
//! m-js keeps four counters on the `M` namespace. The docs' own "Redraw
//! control & instrumentation" panel names two of them; the other two are in
//! the source (`src/m.js`) and in the live CDN bundle:
//!
//! | counter | meaning | resets? |
//! |---|---|---|
//! | `M.renderCount` | **draw passes completed** — `performRedraw()` ran to the end | never |
//! | `M.refreshCount` | draws *requested* — every `M.redraw()` call, including ones collapsed into a batch or dropped by the cap | never |
//! | `M.redrawsDropped` | draws refused by the debug throttle | never |
//! | `M.takePerfStats()` | `{ flushes, effects, redraws }` since the last call | **yes — reading it zeroes it** |
//!
//! This badge reads the first three and never calls `takePerfStats()`. That
//! is deliberate: the take is destructive, so a second reader (a test, the
//! console, the docs' preview probe) would silently get zeros for the window
//! this badge already consumed. `M.drawCallCount` is the same trap wearing a
//! property's clothes — it is a deprecated alias for `takePerfStats().flushes`
//! and so resets the counters on *read*. An observer must not perturb what it
//! observes, so this one only ever reads the monotonic counters.
//!
//! ## Why there is no store binding
//!
//! The obvious build — sample into `$store.live`, bind the text — measures its
//! own shadow: a store write schedules a redraw, so the counter would count
//! the act of displaying the counter, and at 2 Hz that is most of what it
//! would report.
//!
//! So this follows the kit's fast-update contract (`ui_reltime`, and §6 of
//! `docs/RUST_M_JS.md`): m-js renders the shell **once** from the template,
//! and one `lifecycle::every` tick is the **sole writer** of one text node,
//! changed-only, through direct DOM. There is deliberately no `x-text` on
//! `.perf-read` — a binding there would fight the tick, and whichever lost
//! would leave a stale number on screen that reads as a measurement.
//!
//! Because it reads the runtime rather than a store, it takes no store
//! expression: `/` and `/storybook` mount the identical widget and both show
//! the real numbers. A sampled constant in the guide would be the one thing
//! AGENTS.md forbids — fiction that reads as fact.
//!
//! ## The other half of the debug capability
//!
//! Two switches pair with these counters, both live in the CDN bundle:
//!
//! - `window.__M_REDRAW_CAP__ = 4` — cap redraws at N/sec. Excess calls are
//!   dropped (and counted in `M.redrawsDropped`, which is why this badge shows
//!   a `dropped` segment the moment one appears) with a single trailing redraw
//!   scheduled so the latest state still paints.
//! - `M.debug = true` — per-node create/delete/patch logging from the vdom.

use wasm_bindgen::prelude::*;

use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs::{self, Def};
use rustex1_kit::{js, lifecycle};

use crate::css;

pub const NAME: &str = "uiPerfBadge";

/// The attribute the tick finds its text node by. The widget root carries the
/// class (for CSS); this marks the one node the tick writes.
const ATTR: &str = "data-perf-read";

/// Resample period. Slow enough that the badge is not itself a load, fast
/// enough that a runaway redraw loop shows up as it happens rather than after
/// the fact.
const TICK_MS: f64 = 500.0;

thread_local! {
    /// The boot whose tick is running: a bool would leave a new boot with no
    /// timer at all, so it holds the boot token the tick belongs to.
    static TICK_FOR: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

pub const CSS: &str = r#"
/* A readout, not a control: borderless, so it reads as a status on the
   tab strip, not a pill. Same 30px height as the strip's toggles so it
   sits on their baseline; no hover and no pointer — nothing happens if
   you click it. */
.perf-badge {
  flex: 0 0 auto;
  display: inline-flex; align-items: center; gap: 6px;
  height: 30px; padding: 0 9px;
  background: transparent;
  color: var(--muted);
  font-family: var(--sans);
  font-size: 11px;
  letter-spacing: .02em;
  white-space: nowrap;
  cursor: default;
  user-select: none;
}
.perf-badge .ph { font-size: 13px; line-height: 1; }
/* tabular figures: the count climbs by one per draw call, and proportional
   digits would make the whole badge twitch sideways. */
.perf-badge .perf-read {
  font-variant-numeric: tabular-nums;
  color: var(--mjs-yellow);
}
"#;

/// The badge. One widget, no arguments — its data source is the m-js runtime,
/// so there is no store to thread down and nothing to pass differently in the
/// guide.
pub fn badge() -> Node {
    Node::host(NAME, &[])
}

pub fn register() {
    css::inject("perf-badge", CSS);
    mjs::component(NAME, &def().finish());
}

fn def() -> Def {
    let mut d = Def::new().named(NAME);
    d.tree(
        m("span")
            .class("perf-badge")
            .attr("role", "status")
            .attr("aria-live", "off")
            .attr(
                "title",
                "draw calls",
            )
            .kids(vec![
                Node::icon("paint-bucket"),
                // No x-text: the tick owns this node's text. See the module doc.
                m("span").class("perf-read").attr(ATTR, "").ignore(),
            ]),
    );
    // First paint now rather than up to half a second from now, and the one
    // tick per boot that feeds every instance on the page.
    d.method("init", |_this, _args| {
        ensure_tick();
        write_all();
        JsValue::UNDEFINED
    });
    d
}

/// Start this boot's single tick, once.
fn ensure_tick() {
    let token = lifecycle::token();
    if TICK_FOR.with(|t| *t.borrow() == Some(token.clone())) {
        return;
    }
    TICK_FOR.with(|t| *t.borrow_mut() = Some(token.clone()));
    let for_cb = token.clone();
    lifecycle::every(TICK_MS, move || {
        // A hot swap supersedes this boot; ticking on would let a dead
        // instance keep writing into the live DOM.
        if !lifecycle::am_current_token(&for_cb) {
            return;
        }
        write_all();
    });
}

/// Read the counters once and compose the line they render as.
///
/// `drawn` is the honest answer to "how many times has m-js drawn": the count
/// of passes that completed. `M.refreshCount` (draws *requested*) is
/// deliberately **not** shown: the two diverge permanently the first time a
/// re-entrant redraw collapses into a batch, so an inline gap would grow for
/// the life of the page and read as a leak when it is the coalescing working.
/// It is a cumulative figure, so the console is the right place to read it —
/// the tooltip says so.
fn readout() -> String {
    let mj = mjs::m();
    let drawn = js::get_num(&mj, "renderCount");
    let dropped = js::get_num(&mj, "redrawsDropped");
    if !drawn.is_finite() {
        // An m-js older than these counters: say so rather than print NaN,
        // which would read as a measurement of zero.
        return "n/a".to_string();
    }

    let mut out = format!("{}", drawn as i64);
    // Only ever non-zero once `__M_REDRAW_CAP__` is set, which is how the
    // throttle makes itself discoverable instead of silently eating frames.
    if dropped.is_finite() && dropped > 0.0 {
        out.push_str(&format!(" · {} dropped", dropped as i64));
    }
    out
}

/// Write every mounted badge from one counter read, so two on a page cannot
/// disagree — the same reason `ui_reltime` takes one `Date.now()` per tick.
fn write_all() {
    let Some(nodes) = js::query_all(&format!("[{ATTR}]")) else {
        return;
    };
    // A NodeList is array-like, not an `Array`, so length and index both go
    // through Reflect.
    let count = js_sys::Reflect::get(&nodes, &JsValue::from_str("length"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as usize;
    if count == 0 {
        return;
    }
    let text = readout();
    for i in 0..count {
        let node = js_sys::Reflect::get(&nodes, &JsValue::from(i as u32))
            .unwrap_or(JsValue::UNDEFINED);
        if node.is_null() || node.is_undefined() {
            continue;
        }
        // Changed-only: rewriting an identical string would dirty the DOM for
        // a value that did not move, and a dirty DOM is what this badge is
        // for measuring.
        if js::get_str(&node, "textContent") != text {
            js::set_text(&node, &text);
        }
    }
}

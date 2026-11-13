//! The "Relative Timestamp" storybook component.
//!
//! Hosts the app's `uiRelTime` widget (short form `1s 2m 3h`, shared 1 Hz
//! tick, no "ago") as a 5-slot demo: `3m 20m 45m 1h` plus a live counter
//! starting at the epoch — `0s`, `1s`, `2s`, … — exactly the spec'd sequence.
//!
//! ## Deterministic `:at` binds
//!
//! Each bind pins a single epoch on `window.__rtEpoch` (first evaluation, i.e.
//! when the set first mounts ≈ page load) and derives the instant as
//! `epoch + offset`. On every global redraw m-js re-evaluates the binds, but
//! the value is the same each time, so the slots never jump. The live slot
//! (`offset 0`) keeps aging from the original epoch, and away-and-back keeps
//! its age because the epoch survives on `window`.

use rustex1_kit::h::{m, Node};

/// `:at` bind for a slot offset (ms **behind** the pinned epoch, i.e. a past
/// instant — the state has been running for `offset`):
fn at_expr(offset_ms: i64) -> String {
    format!(
        "(window.__rtEpoch===undefined)&&(window.__rtEpoch=Date.now()),new Date(window.__rtEpoch-{}).toISOString()",
        offset_ms
    )
}

const MIN: i64 = 60 * 1000;

pub fn demo() -> Node {
    m("div")
        .class("relative-timestamp demo-set")
        .kids(vec![
            Node::host("uiRelTime", &[]).bind("at", &at_expr(3 * MIN)),
            Node::host("uiRelTime", &[]).bind("at", &at_expr(20 * MIN)),
            Node::host("uiRelTime", &[]).bind("at", &at_expr(45 * MIN)),
            Node::host("uiRelTime", &[]).bind("at", &at_expr(60 * MIN)),
            Node::host("uiRelTime", &[]).bind("at", &at_expr(0)),
        ])
}

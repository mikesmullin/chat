//! `uiRelTime` — the storybook's relative timestamp, short form, no "ago".
//!
//! `3m 20m 45m 1h` plus a live slot that counts up `0s 1s 2s …`.
//!
//! ## Shape
//!
//! Port of the kit's `ui_reltime` fast-update pattern (one shared 1 Hz tick per
//! boot, direct `textContent` writes, no `x-text` on the node), with the
//! storybook's short grammar in place of the kit's long "2sec ago / 3hr ago"
//! form. The kit's `fmt_rel_short` is hard-coded with " ago" and takes no
//! format prop, so the storybook needs its own widget to hit its spec.
//!
//! ## Redraw-safety
//!
//! The app calls a *global* `mjs::redraw()` on every store poll (~4s). A
//! redraw re-evaluates every bound expression on every live template, so a
//! `:at` bind that reads `Date.now()` would re-derive a fresh instant on each
//! redraw and the live slot would jump back to "0s" every few seconds.
//!
//! Fix: the *epoch* (the instant everything ages from) is pinned to
//! `window.__rtEpoch` on first evaluation. Each slot's `at` derives from that
//! pinned number plus a static per-slot offset, so the `:at` bind is
//! deterministic — the same value on every redraw — and the live slot keeps
//! aging from the original epoch. Away-and-back is free: the epoch survives on
//! `window`, and the DOM is the authority on which instances exist.

use std::cell::Cell;
use std::collections::HashMap;
use std::cell::RefCell;

use rustex1_kit::h::m;
use rustex1_kit::js;
use rustex1_kit::lifecycle;
use rustex1_kit::mjs::{self, Def};
use wasm_bindgen::prelude::*;

use crate::css;

pub const NAME: &str = "uiRelTime";

const TICK_MS: f64 = 1000.0;
const ATTR: &str = "data-reltime";

/// The widget's own label styles + the demo-set showcase chrome.
const CSS: &str = r#"
.ui-reltime {
  font-family: var(--mono);
  font-size: 12px;
  color: var(--muted);
  flex: 0 0 auto;
  white-space: nowrap;
  min-width: 4ch;
}
.relative-timestamp.demo-set { display: flex; gap: 6px; align-items: baseline; }
.relative-timestamp.demo-set span { white-space: nowrap; }
/* live instances in the demo set: reserve width so labels grow without
   shifting the neighbours (the shared tick is the sole writer of text) */
.relative-timestamp.demo-set [data-reltime] { min-width: 8ch; }
"#;

thread_local! {
    /// tag -> instance scope. The shared tick walks the DOM and resolves scopes
    /// by tag (same shape as the kit's `ui_reltime`).
    static SCOPES: RefCell<HashMap<String, JsValue>> = RefCell::new(HashMap::new());
    static NEXT_ID: Cell<u64> = const { Cell::new(0) };
    /// The boot whose shared tick is running; keyed on the lifecycle token so a
    /// hot swap starts a fresh chain rather than concluding one is already going.
    static TICK_FOR: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Factory form: called per instance, which mints a unique tag per node.
pub fn register() {
    css::inject(NAME, CSS);
    mjs::component_factory(NAME, |_props| {
        let id = format!(
            "rel-{}",
            NEXT_ID.with(|n| {
                let v = n.get() + 1;
                n.set(v);
                v
            })
        );
        def(&id).finish()
    });
}

fn def(id: &str) -> Def {
    let mut d = Def::new().named(NAME);
    d.prop_str("at", "").prop_str("title", "").str("tag", id).tree(
        m("span")
            .class("ui-reltime")
            .attr(ATTR, id)
            // no x-text on the node: the tick is the sole writer of the label.
            // The tooltip is the host's to supply (the tree row sends the
            // `created … modified …` instants); the default stays the empty
            // title, which keeps the row's own hover text off the age.
            .bind("title", "title"),
    );
    d.method("init", |this, _args| {
        let tag = js::get_str(&this, "tag");
        SCOPES.with(|s| s.borrow_mut().insert(tag.clone(), this.clone()));
        // subscribe, do not schedule: one chain serves every instance on the page.
        ensure_tick();
        write_tag(&tag); // first paint now, not up to a second from now.
        JsValue::UNDEFINED
    })
    .method("destroy", |this, _args| {
        let tag = js::get_str(&this, "tag");
        SCOPES.with(|s| s.borrow_mut().remove(&tag));
        JsValue::UNDEFINED
    });
    d
}

/// Start the page's single tick, once per boot.
fn ensure_tick() {
    let token = lifecycle::token();
    if TICK_FOR.with(|t| *t.borrow() == Some(token.clone())) {
        return;
    }
    TICK_FOR.with(|t| *t.borrow_mut() = Some(token.clone()));
    let for_cb = token.clone();
    lifecycle::every(TICK_MS, move || {
        if !lifecycle::am_current_token(&for_cb) {
            return;
        }
        tick_all();
    });
}

/// One tick for every instance on the page: one DOM walk, one `Date.now()`.
fn tick_all() {
    let Some(nodes) = js::query_all(&format!("[{ATTR}]")) else {
        return;
    };
    let now = js_sys::Date::now();
    // A NodeList is array-like, not an Array, so length and index come through
    // Reflect.
    let count = js_sys::Reflect::get(&nodes, &JsValue::from_str("length"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as usize;
    let mut live: Vec<String> = Vec::with_capacity(count);
    for i in 0..count {
        let node = js_sys::Reflect::get(&nodes, &JsValue::from(i as u32))
            .unwrap_or_else(|_| JsValue::UNDEFINED);
        if node.is_null() || node.is_undefined() {
            continue;
        }
        let tag = js::call_string(&node, "getAttribute", &[JsValue::from_str(ATTR)]);
        if tag.is_empty() {
            continue;
        }
        let at = SCOPES
            .with(|s| s.borrow().get(tag.as_str()).map(|scope| js::get_str(scope, "at")))
            .unwrap_or_default();
        write_node(&node, &at, now);
        live.push(tag);
    }
    SCOPES.with(|s| {
        let mut m = s.borrow_mut();
        m.retain(|tag, _| live.iter().any(|l| l == tag));
    });
}

/// Write one instance's label from the tick's single `now`.
fn write_node(node: &JsValue, at: &str, now: f64) {
    let text = fmt_short(at, now);
    if js::get_str(node, "textContent") != text {
        js::set_text(node, &text);
    }
}

/// Single-instance write, used for first paint before the tick next fires.
fn write_tag(tag: &str) {
    let at = SCOPES
        .with(|s| s.borrow().get(tag).map(|scope| js::get_str(scope, "at")))
        .unwrap_or_default();
    let Some(node) = js::query(&format!("[{ATTR}=\"{tag}\"")) else {
        return; // not mounted yet
    };
    write_node(&node, &at, js_sys::Date::now());
}

// --- short form, no "ago": 1s 2m 3h 4d --------------------------------

/// Relative short form, storybook grammar: `1s 2m 3h 4d` — no trailing text
/// (no " ago" and no " in "), so the slot is unambiguous in either direction.
fn fmt_short(at: &str, now_ms: f64) -> String {
    if at.is_empty() {
        return "…".to_string();
    }
    let ms = match parse_iso_to_ms(at) {
        Some(ms) if ms.is_finite() => ms,
        _ => return "—".to_string(),
    };
    let diff = ((now_ms - ms) / 1000.0).round();
    let s = diff.abs();
    let unit = if s < 60.0 {
        format!("{}s", s as i64)
    } else if s < 3600.0 {
        format!("{}m", (s / 60.0).floor() as i64)
    } else if s < 86_400.0 {
        format!("{}h", (s / 3600.0).floor() as i64)
    } else if s < 86_400.0 * 30.0 {
        format!("{}d", (s / 86_400.0).floor() as i64)
    } else {
        format!("{}mo", (s / (86_400.0 * 30.0)).floor() as i64)
    };
    unit
}

// --- parsing (Rust-only; the tick path never calls into JS) -------------

/// `YYYY-MM-DD` or `YYYY-MM-DD[THH:MM:SS[.fff][Z|±hh:mm]]` -> epoch ms.
/// Date-only reads as UTC midnight. The tail must be fully consumed; a stray
/// character is a parse failure, never a silent tolerance.
fn parse_iso_to_ms(iso: &str) -> Option<f64> {
    let v = iso.trim();
    let (y, m, d) = (digits(v, 0..4)?, digits(v, 5..7)?, digits(v, 8..10)?);
    if v.len() < 10 || v.as_bytes()[4] != b'-' || v.as_bytes()[7] != b'-' {
        return None;
    }
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let mut secs = 0i64;
    if v.len() > 10 {
        let sep = v.as_bytes()[10];
        if sep != b'T' && sep != b' ' {
            return None;
        }
        let rest = &v[11..];
        let (hh, mm, ss) = (
            digits(rest, 0..2)?,
            digits(rest, 3..5)?,
            digits(rest, 6..8).unwrap_or(0),
        );
        if !(0..=23).contains(&hh) || !(0..=59).contains(&mm) || !(0..=60).contains(&ss) {
            return None;
        }
        secs = hh * 3600 + mm * 60 + ss;
        // Tail: optional fraction, optional zone. JS `toISOString()` always
        // yields `.mmmZ`; anything with leftover is not a shape this widget reads.
        let tail = &rest[8..];
        let tail = if tail.starts_with('.') {
            // optional fraction: one-or-more digits, then the zone (or nothing)
            let frac = &tail[1..];
            let (frac_len, rest) = {
                let mut i = 0;
                while i < frac.len() && frac.as_bytes()[i].is_ascii_digit() {
                    i += 1;
                }
                (i, &frac[i..])
            };
            if frac_len == 0 {
                return None;
            }
            rest
        } else {
            tail
        };
        let offset = if tail.is_empty() {
            0
        } else if tail.as_bytes()[0] == b'Z' || tail.as_bytes()[0] == b'z' {
            // Z: UTC — require exactly one marker, nothing else.
            if !tail.as_bytes()[1..].is_empty() {
                return None;
            }
            0
        } else {
            // ±hh:mm
            if tail.len() != 6 || tail.as_bytes()[3] != b':' {
                return None;
            }
            let sign = tail.as_bytes()[0];
            if sign != b'+' && sign != b'-' {
                return None;
            }
            let (oh, om) = (digits(tail, 1..3)?, digits(tail, 4..6)?);
            if !(0..=23).contains(&oh) || !(0..=59).contains(&om) {
                return None;
            }
            if sign == b'-' {
                -(oh * 3600 + om * 60)
            } else {
                oh * 3600 + om * 60
            }
        };
        secs -= offset;
    }
    Some((days_from_civil(y, m, d) * 86_400 + secs) as f64 * 1000.0)
}

fn digits(v: &str, range: std::ops::Range<usize>) -> Option<i64> {
    let slice = v.get(range)?;
    if slice.is_empty() || !slice.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    slice.parse().ok()
}

/// Howard Hinnant's `days_from_civil`, inlined to stay dependency-free.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let (y, era) = if m <= 2 {
        (y - 1, (y - 1) / 400)
    } else {
        (y, y / 400)
    };
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

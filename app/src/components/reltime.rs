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
.relative-timestamp.demo-set [data-reltime] { min-width: 8ch; }
"#;

thread_local! {
    static SCOPES: RefCell<HashMap<String, JsValue>> = RefCell::new(HashMap::new());
    static NEXT_ID: Cell<u64> = const { Cell::new(0) };
    static TICK_FOR: RefCell<Option<String>> = const { RefCell::new(None) };
}

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
            .bind("title", "title"),
    );
    d.method("init", |this, _args| {
        let tag = js::get_str(&this, "tag");
        SCOPES.with(|s| s.borrow_mut().insert(tag.clone(), this.clone()));
        ensure_tick();
        write_tag(&tag);
        JsValue::UNDEFINED
    })
    .method("destroy", |this, _args| {
        let tag = js::get_str(&this, "tag");
        SCOPES.with(|s| s.borrow_mut().remove(&tag));
        JsValue::UNDEFINED
    });
    d
}

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

fn tick_all() {
    let Some(nodes) = js::query_all(&format!("[{ATTR}]")) else {
        return;
    };
    let now = js_sys::Date::now();
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

fn write_node(node: &JsValue, at: &str, now: f64) {
    let text = fmt_short(at, now);
    if js::get_str(node, "textContent") != text {
        js::set_text(node, &text);
    }
}

fn write_tag(tag: &str) {
    let at = SCOPES
        .with(|s| s.borrow().get(tag).map(|scope| js::get_str(scope, "at")))
        .unwrap_or_default();
    let Some(node) = js::query(&format!("[{ATTR}=\"{tag}\"")) else {
        return;
    };
    write_node(&node, &at, js_sys::Date::now());
}


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
        let tail = &rest[8..];
        let tail = if tail.starts_with('.') {
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
            if !tail.as_bytes()[1..].is_empty() {
                return None;
            }
            0
        } else {
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

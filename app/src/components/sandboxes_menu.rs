//! `sandboxes menu` — the composite "Sandboxes Menu": a dim "Sandboxes" header
//! followed by one group per host computer (attention order here is
//! alphabetical by host). Each group is a neutral `item-group` (host name +
//! inline counter) with one row per sandbox: computer icon + sandbox name.
//!
//! Reuses the shared `.item-group` / `.item-head` / `.item-list` /
//! `.item-row` classes from `item_group` (the CSS contract), plus the
//! standalone `uiCounter` widget inline after each host name — so the only
//! distinct library component it is built from is the counter (depth 1).
//!
//! Owns its header CSS; injected at boot by `components::register`.

use rustex1_kit::h::{lit, m, Node};

/// the sandboxes menu's styles (the "Sandboxes" header).
pub const CSS: &str = r#"
/* the "sandboxes" menu title: normal weight, dim gray (near #666) */
.sandboxes-header {
  font-size: 13px;
  font-weight: 400;
  color: var(--gray-dim);
  margin-bottom: 14px;
}
"#;

/// one sandbox row: computer icon + sandbox name.
fn row(name: &str) -> Node {
    m("li").class("item-row").kids(vec![
        Node::icon("desktop"),
        m("span").class("item-name").child(lit(name)),
    ])
}

/// the group header: host name + the inline counter widget.
fn head(host: &str, count: &str) -> Node {
    m("div").class("item-head").kids(vec![
        m("span").class("item-head-name").child(lit(host)),
        Node::host("uiCounter", &[]).bind("value", count),
    ])
}

/// one host group: neutral `.item-group` (no state tone) + rows.
fn sandbox_group(host: &str, count: &str, names: &[&str]) -> Node {
    m("div").class("item-group").kids(vec![
        head(host, count),
        m("ul")
            .class("item-list")
            .kids(names.iter().map(|n| row(n)).collect()),
    ])
}

pub fn sandboxes_menu() -> Node {
    m("div").class("sandboxes-menu").kids(vec![
        m("p").class("sandboxes-header").child(lit("Sandboxes")),
        sandbox_group("atlas", "2", &["checkout-flow", "billing-replay"]),
        sandbox_group("borealis", "1", &["auth-migration"]),
        sandbox_group(
            "cinder",
            "3",
            &["search-index", "notifications", "cache-warm"],
        ),
    ])
}

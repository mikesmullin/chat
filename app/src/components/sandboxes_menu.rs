//! `sandboxes menu` — the left-rail listing of running sandboxes (PLAN55).
//!
//! The section head is [`rail_head`], the same brick as Projects and Agents:
//! a caret collapse toggle and the section's glyph (`cube`), in the same
//! type — and no create button, because sandboxes are agent-owned and launch
//! themselves (D4: there is nothing to create here).
//!
//! Below it, one group per image, on the project-group row (the tree's
//! `.ptree-row` contract: caret, glyph, name — the glyph is the
//! `shipping-container` mark). Clicking the group collapses its instances,
//! like a project folder. Inside, indented, each instance is its container
//! id — the only
//! identity there is (D3/D10: friendly names are revoked) — and clicking it
//! opens the Sandbox form in the inspector: the row it is, what it runs,
//! and Delete. No counters, no byline; the id is what there is to see.
//!
//! The rows and their grouping are store data: `sandboxes_menu(st)` reads
//! `{st}.sandboxGroups` (computed by [`group_rows`] when rows arrive) and
//! `{st}.sandboxOpen` (which groups are expanded). `/` passes `$store.live`
//! and `/storybook` passes `$store.samples` — the same markup both times,
//! which is the whole point of the thing.
//!
//! Owns its header CSS; the rows ride the shared `.ptree-row` (project_tree)
//! and `.item-row` (item_group) contracts.

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::{js, mjs};
use wasm_bindgen::JsValue;

use crate::components::rail_head;

/// the sandboxes menu's styles (the indent + the notes; the rows ride the
/// tree's and the item group's contracts).
pub const CSS: &str = r#"
/* instances sit under their image group, one tree indent in */
.sandboxes-menu .item-list { padding-left: 24px; }
.sandboxes-menu .item-row { cursor: pointer; }
/* the container id reads at the same size and weight as the tree's names,
   not at the item list's larger bold one — it is a left-rail menu row like
   the rest. */
.sandboxes-menu .item-name { font-size: 12px; font-weight: 400; }
/* …and its glyph wears the tree's grey at the tree's size, like every
   other mark in the rail. */
.sandboxes-menu .item-row > i { font-size: 13px; color: var(--gray-dim); }
/* the one line that says the listing is empty (or the service is down) */
.sandbox-empty {
  font-size: 12px;
  color: var(--gray-dim);
  padding: 6px 8px;
  line-height: 1.5;
}
"#;

/// Is this container state one a turn may still use? Kept in step with
/// angela's sandbox policy: `ready`, or still coming up.
fn is_live(state: &str) -> bool {
    matches!(state, "ready" | "starting" | "cloning")
}

/// Group container rows by image — one group per image, `count` the per-image
/// live rollup (PLAN55 D6), `rows` in listing order.
///
/// Shape: `[{ image, count, rows: [<row>, …] }, …]`, images in first-seen
/// order. Registered as a store method on both the live and the samples
/// store, so there is exactly one grouping and the rollup is always the rows.
///
/// The rows array is kept as the very handle stored on the group and pushed
/// to in place — `js_sys::Array::from` **copies**, and pushing onto a copy
/// leaves the group's own array empty forever.
pub fn group_rows(rows: &JsValue) -> JsValue {
    let out = js_sys::Array::new();
    let mut order: Vec<String> = Vec::new(); // first-seen image order
    let mut members: Vec<js_sys::Array> = Vec::new(); // the stored rows handle per group
    let mut counts: Vec<f64> = Vec::new();
    for row in js_sys::Array::from(rows).iter() {
        let image = js::get_str(&row, "image");
        let key = if image.is_empty() { "unknown".to_string() } else { image };
        let gi = match order.iter().position(|k| *k == key) {
            Some(i) => i,
            None => {
                order.push(key.clone());
                members.push(js_sys::Array::new());
                counts.push(0.0);
                let g = js::obj();
                js::set_str(&g, "image", &key);
                js::set(&g, "count", &JsValue::from_f64(0.0));
                js::set(&g, "rows", members.last().expect("just pushed"));
                out.push(&g);
                order.len() - 1
            }
        };
        if is_live(&js::get_str(&row, "state")) {
            counts[gi] += 1.0;
        }
        members[gi].push(&row);
    }
    for (i, count) in counts.into_iter().enumerate() {
        js::set(&out.get(i as u32), "count", &JsValue::from_f64(count));
    }
    out.into()
}

/// `toggleSandboxGroup(image)` — collapse/expand one image's instances,
/// the way a project group opens and closes. The flag lives on the calling
/// store (`this` is the scope object), so one implementation serves the
/// app's live store and the guide's samples store. Absent = open.
pub fn toggle_group(this: JsValue, args: js_sys::Array) -> JsValue {
    let image = js::arg_str(&args, 0);
    if image.is_empty() {
        return JsValue::UNDEFINED;
    }
    let map = js::get(&this, "sandboxOpen");
    let open = js::get(&map, &image).as_bool().unwrap_or(true);
    if !map.is_object() {
        js::set(&this, "sandboxOpen", &js::obj());
    }
    let map = js::get(&this, "sandboxOpen");
    js::set(&map, &image, &JsValue::from_bool(!open));
    mjs::redraw();
    JsValue::UNDEFINED
}

/// one instance row: the container id, and the Sandbox form on click.
///
/// The id is shown bare: ids minted before this rule carried a `sandbox2-`
/// service prefix, and the prefix says nothing — the hex is the identity
/// (PLAN55 D3). The click still passes the full id.
fn row(st: &str) -> Node {
    m("li")
        .for_each("r in g.rows")
        .key("r.id")
        .class("item-row")
        .on("click", &format!("{st}.editTarget('sandbox', r.id)"))
        .kids(vec![
            Node::icon("desktop"),
            m("span").class("item-name").text(
                "(r.id.startsWith('sandbox2-') ? r.id.slice(9) : r.id)",
            ),
        ])
}

/// one image group: the project-group row (caret + glyph + image) over its
/// instances.
fn group(st: &str) -> Node {
    m("div")
        .for_each(&format!("g in {st}.sandboxGroups"))
        .key("g.image")
        .kids(vec![
            m("div")
                .class("ptree-row")
                .bind("class", &format!("({st}.sandboxOpen[g.image] === false ? '' : 'is-open')"))
                .bind("title", "g.image")
                .on("click", &format!("{st}.toggleSandboxGroup(g.image)"))
                .kids(vec![
                    m("i").class("ph ph-caret-right ptree-caret"),
                    m("i").class("ph ph-shipping-container ptree-icon"),
                    m("span").class("ptree-name").text("g.image"),
                ]),
            m("ul")
                .class("item-list")
                .show(&format!("{st}.sandboxOpen[g.image] !== false"))
                .child(row(st)),
        ])
}

/// The menu. `st` names the store whose `sandboxGroups` / `sandboxOpen` /
/// `sboxUp` it reads (`$store.live` on `/`, `$store.samples` on
/// `/storybook`); the section's collapse flag is the shell's (`$store.home`),
/// like every other rail section.
pub fn sandboxes_menu(st: &str) -> Node {
    m("div")
        .class("sandboxes-menu")
        .bind("class", "($store.home.sandboxesOpen ? '' : 'is-collapsed')")
        .kids(vec![
            rail_head::head("Sandboxes", "cube", None, Some("sandboxes")),
            m("div").show("$store.home.sandboxesOpen").kids(vec![
                m("p")
                    .class("sandbox-empty")
                    .show(&format!("!{st}.sboxUp"))
                    .child(lit("The sandbox service is not answering.")),
                m("p")
                    .class("sandbox-empty")
                    .show(&format!("{st}.sboxUp && {st}.sandboxGroups.length === 0"))
                    .child(lit("No sandboxes yet.")),
                group(st),
            ]),
        ])
}

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
//! Each instance also carries a trash icon-button, the same per-row destroy
//! the Sandbox form's Delete button offers — same endpoint, same two-press
//! mechanic (the first press arms the button and says so, the second does
//! it). The armed state is temporary: opening the Sandbox form clears it,
//! and 3 seconds after the first press it goes back to grey on its own.
//! The button is the row's action, not its content: it appears on hover or
//! focus and stays visible while armed.
//!
//! The rows and their grouping are store data: `sandboxes_menu(st)` reads
//! `{st}.sandboxGroups` (computed by [`group_rows`] when rows arrive) and
//! `{st}.sandboxOpen` (which groups are expanded) — plus `{st}.sboxConfirmId`
//! (the armed row) and `{st}.sboxError` (the last destroy failure). `/`
//! passes `$store.live` and `/storybook` passes `$store.samples` — the same
//! markup both times, which is the whole point of the thing.
//!
//! Owns its header CSS; the rows ride the shared `.ptree-row` (project_tree)
//! and `.item-row` (item_group) contracts.

use std::cell::Cell;

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::{js, mjs};
use wasm_bindgen::JsValue;

use crate::components::rail_head;

/// How long a row's trash button stays armed before it goes back to grey.
/// The arm is the *only* thing a first click does, so it has to expire on
/// its own — a button that says "armed" long after the user has moved on
/// is a lie waiting to be clicked.
const ARM_SECONDS: f64 = 3.0;

thread_local! {
    /// One generation per arm: each arm increments this, and the pending
    /// auto-disarm only clears the arm if it is still the current one. A
    /// stale timer from an earlier arm can never clear a newer arm.
    static ARM_GENERATION: Cell<u32> = const { Cell::new(0) };
}

/// Arm one row's trash button: red glyph on a tinted background (the
/// `is-confirm` state), a fresh 3-second auto-disarm, and the last error
/// gone — the armed button is the only pending verb now.
///
/// `store_name` is the calling store (`live` or `samples`): both stores run
/// the same arm, and the timer lands on the same store it armed on.
pub(crate) fn arm_row(store_name: &str, id: &str) {
    let s = mjs::store_ref(store_name);
    js::set_str(&s, "sboxConfirmId", id);
    js::set_str(&s, "sboxError", "");
    mjs::redraw();
    let gen = ARM_GENERATION.with(|g| {
        let next = g.get() + 1;
        g.set(next);
        next
    });
    let store = store_name.to_string();
    let id = id.to_string();
    // `lifecycle::after` is the one-shot timer the kit keeps the boot-token
    // check on: a hot-swapped instance's stale arm can never clear the
    // current boot's one, because it checks `am_current()` before firing.
    rustex1_kit::lifecycle::after(ARM_SECONDS * 1000.0, move || {
        let current = ARM_GENERATION.with(|g| g.get());
        if current != gen {
            return;
        }
        let s = mjs::store_ref(&store);
        if js::get_str(&s, "sboxConfirmId") != id {
            return;
        }
        js::set_str(&s, "sboxConfirmId", "");
        mjs::redraw();
    });
}


/// the sandboxes menu's styles (the indent + the notes; the rows ride the
/// tree's and the item group's contracts).
pub const CSS: &str = r#"
/* instances sit under their image group, one tree indent in */
.sandboxes-menu .item-list { padding-left: 24px; }
.sandboxes-menu .item-row { cursor: pointer; }
/* the container id reads at the same size and weight as the tree's names,
   not at the item list's larger bold one — it is a left-rail menu row like
   the rest. */
.sandboxes-menu .item-name {
  font-size: 12px; font-weight: 400;
  flex: 1 1 auto; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
/* …and its glyph wears the tree's grey at the tree's size, like every
   other mark in the rail. */
.sandboxes-menu .item-row > i { font-size: 13px; color: var(--gray-dim); }
/* The per-row destroy button — the row's action, not its content: hidden
   until the row is hovered or focused, like the device-revoke button in
   the inspector and the transcript's fork button. Keyboard still reaches
   it (focus keeps it visible), and an armed one never hides: the armed
   state has to be visible even when the pointer is elsewhere. */
.sandboxes-menu .sandbox-del {
  flex: 0 0 auto;
  border: 0; background: transparent; color: var(--gray-dim);
  cursor: pointer; padding: 3px; border-radius: 6px;
  font-size: 12px; line-height: 1;
  opacity: 0; pointer-events: none;
  transition: opacity 100ms ease, color 100ms ease, background 100ms ease;
}
.sandboxes-menu .item-row:hover .sandbox-del,
.sandboxes-menu .item-row:focus-within .sandbox-del {
  opacity: 1; pointer-events: auto;
}
/* Dormant: grey glyph, no background — the hover hint must not read as the
   armed state. The armed-red state is the is-confirm rule below and nothing
   else, so grey is a 1:1 match for "not armed yet". */
.sandboxes-menu .sandbox-del:hover { color: var(--gray, #d1d5db); }
.sandboxes-menu .sandbox-del.is-confirm {
  opacity: 1; pointer-events: auto;
  color: var(--danger, #f87171);
  background: color-mix(in srgb, var(--danger, #f87171) 14%, transparent);
}
/* the one line that says the listing is empty (or the service is down) */
.sandbox-empty {
  font-size: 12px;
  color: var(--gray-dim);
  padding: 6px 8px;
  line-height: 1.5;
}
/* the line that says a destroy failed: red ink on the same tint the forms
   use for a verdict that blocks you. */
.sandbox-error {
  display: flex; align-items: flex-start; gap: 6px;
  font-size: 12px; color: var(--danger, #f87171);
  border: 1px solid color-mix(in srgb, var(--danger, #f87171) 30%, transparent);
  background: color-mix(in srgb, var(--danger, #f87171) 8%, transparent);
  border-radius: 8px;
  padding: 6px 8px;
  line-height: 1.5;
}
.sandbox-error .ph {
  flex: 0 0 auto;
  font-size: 13px; line-height: 1.5;
  margin-top: 0.5px;
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
///
/// The trash icon-button is the row's second click target. It is a sibling
/// of the id, not a child of the name, and it carries `stopPropagation` on
/// its own click so the row's open-the-form never fires from it. The armed
/// state lives on the store (`sboxConfirmId`) rather than in the button,
/// so one arm and one armed source of truth serve both stores.
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
            m("button")
                .attr("type", "button")
                .bind("class", &format!("('sandbox-del ' + ({st}.sboxConfirmId === r.id ? 'is-confirm' : ''))"))
                .bind("title", &format!("({st}.sboxConfirmId === r.id ? 'Click again to destroy' : 'Destroy this container')"))
                .bind("aria-label", &format!("({st}.sboxConfirmId === r.id ? 'Click again to destroy' : 'Destroy this container')"))
                .on("click.stop", &format!("{st}.sboxRemove(r.id)"))
                .child(Node::icon("trash").attr("aria-hidden", "true")),
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
                    .class("sandbox-error")
                    .show(&format!("{st}.sboxError"))
                    .kids(vec![
                        Node::icon("warning-circle"),
                        m("span").text(&format!("{st}.sboxError")),
                    ]),
                // The error and the notices are sibling notes about the same
                // listing — the error says the last destroy failed, and the
                // notes say the listing is empty or the service is down.
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

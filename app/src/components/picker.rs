//! `picker` — the dropdown variation that lists *instances*: a search box, a
//! "none" row, the things themselves, and the actions that make more of them.
//!
//! The plain [`dropdown`](crate::components::dropdown) answers a question with
//! a fixed vocabulary — which model, which reasoning level, allowlist or
//! unrestricted. Its menu is a short list of words that never changes, so it
//! needs no search, no empty state and no way to add a word.
//!
//! These four do not work like that. A sandbox, a project, a worktree and a
//! branch are **instances**: there may be none, there may be forty, and the
//! answer to "which one?" is often "a new one". That is three things the plain
//! menu has nowhere to put — a filter, a null answer, and a create — so this
//! is a second *menu*, not a second dropdown.
//!
//! ## What it reuses, and why that matters
//!
//! The **trigger** is not redrawn here. `.dropdown`, `.dropdown-btn`,
//! `.dropdown-icon` and `.dropdown-value` are the same classes the plain
//! dropdown wears, and open-state is the same `$store.dropdown.open` key — so
//! opening a picker closes a dropdown and the reverse, which is only true
//! because there is one place that state is written. A picker that invented
//! its own open flag would let two menus sit open at once, and the bug would
//! read as a CSS problem.
//!
//! What this file owns is the menu: the search header, the null row, the
//! instance rows (tile, name, hint, check, pencil) and the action footer.
//!
//! ## Anatomy
//!
//! ```text
//!   ┌─────────────────────────────────┐
//!   │ ⌕  Search projects…             │   search  — filters the list below
//!   ├─────────────────────────────────┤
//!   │ ▣  No project          Session  │   null    — the explicit "neither"
//!   │ ▧  Paradev                   ✓  │   item    — ✓ current, ✎ on hover
//!   │ ▧  Graphics                     │
//!   ├─────────────────────────────────┤
//!   │ ⊞  Create new project           │   actions — open a form, not a row
//!   └─────────────────────────────────┘
//! ```
//!
//! ## The null row is a value, not an absence
//!
//! "No project" is a row you can pick, with a hint saying what picking it
//! means — `Session`, i.e. this chat belongs to nothing but itself. The
//! alternative (no row; deselect by picking the current one again) hides the
//! most common answer behind a gesture nobody guesses.
//!
//! ## Why the pencil waits for hover
//!
//! Every row can be edited, so a pencil on every row is four columns of
//! identical glyphs competing with the one mark that carries information —
//! the check. It appears on the row under the pointer, where it is the only
//! thing it could apply to.

use wasm_bindgen::{JsCast, JsValue};

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::css;

/// This component's own store: the search text, and the composite open.
const STORE: &str = "picker";
/// The plain dropdown's store, which owns open-state for *both* menus.
const DD: &str = "dropdown";

pub const CSS: &str = r#"
/* The trigger wears `.dropdown`'s chrome; `.picker` only re-anchors the
   menu, which is wider than the button and would otherwise hang off the
   left edge of a narrow trigger like "main". */
.picker .picker-menu {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 0;
  z-index: 40;
  /* 300, not 288: at 288 the longest null row ("No repo" + "nothing
     mounted") lands a pixel over and clips the name. Sized to the content
     that has to fit, not to a round number. */
  width: 300px;
  max-width: min(300px, calc(100vw - 24px));
  display: flex;
  flex-direction: column;
  background: var(--panel-2);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: 0 16px 40px rgba(0, 0, 0, .6);
  overflow: hidden;
}
/* ---- search ------------------------------------------------------
   Rules below it rather than a box around it: it is the panel's header,
   not a field sitting in the panel. */
.picker-search {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
  flex: 0 0 auto;
}
.picker-search .ph { font-size: 14px; color: var(--gray-dim); flex: 0 0 auto; }
.picker-search input {
  flex: 1 1 auto;
  min-width: 0;
  border: 0;
  background: transparent;
  color: var(--text);
  font-family: var(--sans);
  font-size: 13px;
  line-height: 18px;
  outline: none;
  padding: 0;
}
.picker-search input::placeholder { color: var(--gray-dim); }
/* The search box IS the header, so the storybook's global
   `:is(#sb,#home) input:focus` glow would draw a ring around a thing that
   has no border — same reasoning as the composer's draft box. */
:is(#sb, #home) .picker-search input:focus {
  border: 0;
  outline: none;
  box-shadow: none;
}

/* ---- the list ----------------------------------------------------
   Capped and scrolled: forty branches must not produce a menu taller than
   the window, and the action footer has to stay reachable without it. */
.picker-list {
  flex: 1 1 auto;
  min-height: 0;
  max-height: 264px;
  overflow-y: auto;
  padding: 6px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.picker-row {
  display: flex;
  align-items: center;
  border-radius: 8px;
  /* The pencil is a sibling of the pick button, not a child: a <button>
     inside a <button> is invalid, and the pencil must be focusable. The
     row is what highlights, so the pair still reads as one strip. */
}
.picker-row:hover { background: var(--accent-glow); }
.picker-item {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 8px;
  border: 0;
  background: transparent;
  color: var(--muted);
  font-family: var(--sans);
  font-size: 13px;
  line-height: 20px;
  text-align: left;
  cursor: pointer;
}
.picker-row:hover .picker-item { color: var(--text); }
.picker-item:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; border-radius: 8px; }

/* The tile marks an instance apart from an action: the rows that name a
   thing carry a plate, the rows that *do* something carry a bare glyph. */
.picker-tile {
  flex: 0 0 auto;
  width: 26px;
  height: 26px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border);
  border-radius: 7px;
  background: var(--panel);
  color: var(--muted);
}
.picker-tile .ph { font-size: 13px; }
.picker-row:hover .picker-tile { color: var(--text); }

.picker-item-label {
  flex: 1 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
/* What picking this row would mean — "Session" under No project, the repo
   a branch belongs to. Dim and right-aligned: it qualifies the name, it is
   not a second name. */
.picker-hint {
  /* Shrinkable, unlike the name beside it. Flex distributes a shortfall in
     proportion to basis, so a long hint ("nothing mounted") gives up most
     of it and the name — the thing you are actually choosing — keeps its
     letters. With `0 0 auto` here the name absorbed the whole shortfall
     and read "No r…" next to a fully-spelled qualifier. */
  flex: 0 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--gray-dim);
  font-size: 12px;
  margin-left: 8px;
}
/* Always laid out, only sometimes visible. `x-show` would remove it and
   let the hint slide into the gutter, so a checked row and an unchecked
   row would end their text at different x — which reads as ragged
   alignment, not as a check. */
.picker-check {
  flex: 0 0 auto;
  width: 14px;
  font-size: 14px;
  color: var(--text);
  margin-left: 8px;
  visibility: hidden;
}
.picker-check.is-on { visibility: visible; }

/* Hover-only, and `visibility` rather than `display` so the row does not
   reflow — a name that re-truncates as the pointer crosses it looks like
   a glitch. */
.picker-edit {
  flex: 0 0 auto;
  width: 26px;
  height: 26px;
  margin-right: 6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--gray-dim);
  cursor: pointer;
  visibility: hidden;
}
.picker-row:hover .picker-edit { visibility: visible; }
/* The null row has nothing to edit, but it must still END where the other
   rows end — otherwise its hint sits a pencil-width further right and the
   right-hand column reads as ragged. */
.picker-gutter { flex: 0 0 auto; width: 26px; margin-right: 6px; }
.picker-edit:focus-visible { visibility: visible; outline: 2px solid var(--accent); outline-offset: -2px; }
.picker-edit:hover { background: var(--panel); color: var(--text); }
.picker-edit .ph { font-size: 13px; }

/* Typing into the search until nothing matches must not look like a menu
   that failed to load. */
.picker-empty {
  margin: 0;
  padding: 14px 10px;
  text-align: center;
  color: var(--gray-dim);
  font-size: 12px;
}

/* ---- actions -----------------------------------------------------
   Ruled off and un-tiled, because these do not answer "which one?" —
   they leave the menu and open a form. */
.picker-actions {
  flex: 0 0 auto;
  border-top: 1px solid var(--border);
  padding: 6px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.picker-action {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 7px 8px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--muted);
  font-family: var(--sans);
  font-size: 13px;
  line-height: 20px;
  text-align: left;
  cursor: pointer;
}
.picker-action:hover { background: var(--accent-glow); color: var(--text); }
.picker-action:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
/* Aligned with the tiles above, so the two groups share one text margin
   even though only one of them is plated. */
.picker-action .ph { font-size: 15px; width: 26px; text-align: center; flex: 0 0 auto; }

/* demo framing */
.picker-demo { display: flex; align-items: flex-end; gap: 14px; flex-wrap: wrap; padding-top: 320px; }
"#;

/// A row in the action footer: the things that make a new instance, or
/// name one that already exists on disk.
pub struct Action<'a> {
    pub icon: &'a str,
    pub label: &'a str,
    /// Expression run on click. Closing the menu is this builder's job, not
    /// the caller's.
    pub run: &'a str,
}

/// Everything one picker instance needs.
///
/// A struct rather than positional arguments: there are ten of them, and
/// `picker("branch", "git-branch", "", "Search branches…", …)` is a line
/// nobody can read or safely reorder.
pub struct Picker<'a> {
    /// Open-state id — must be unique on the page.
    pub key: &'a str,
    /// Trigger glyph.
    pub icon: &'a str,
    /// Extra class on the root, for host layout.
    pub extra: &'a str,
    /// Search placeholder, e.g. `Search projects…`.
    pub search: &'a str,
    /// Expression yielding the array of `{ id, name, icon, hint }`.
    pub items: &'a str,
    /// Expression yielding the selected id — `''` when nothing is chosen.
    pub value: &'a str,
    /// The null row: its label and the hint that says what it means. `None`
    /// for a picker where "nothing" is not an answer.
    pub null: Option<(&'a str, &'a str)>,
    /// Glyph for the null row.
    pub null_icon: &'a str,
    /// Expression run on pick; `{}` becomes the chosen id (`''` for null).
    pub on_pick: &'a str,
    /// Expression run on the pencil; `{}` becomes the row's id. Empty
    /// string hides the pencil entirely.
    pub on_edit: &'a str,
    pub actions: &'a [Action<'a>],
}

/// The trigger's text: the chosen item's name, or the null label.
///
/// One definition of "how a picker labels itself", so four triggers cannot
/// disagree about what an empty selection looks like.
fn trigger_label(p: &Picker) -> String {
    let fallback = p.null.map(|(l, _)| l).unwrap_or("—");
    format!(
        "$store.{STORE}.labelOf({}, {}, '{}')",
        p.items,
        p.value,
        fallback.replace('\'', "\\'")
    )
}

/// One instance row: tile, name, hint, check — and the pencil beside it.
fn item_row(p: &Picker) -> Node {
    let selected = format!("({} === pickItem.id)", p.value);
    let mut row = vec![m("button")
        .class("picker-item")
        .attr("type", "button")
        .on("click", &p.on_pick.replace("{}", "pickItem.id"))
        .kids(vec![
            m("span")
                .class("picker-tile")
                .child(Node::icon_dyn("(pickItem.icon || 'cube')")),
            m("span").class("picker-item-label").text("pickItem.name"),
            m("span")
                .class("picker-hint")
                .show("pickItem.hint")
                .text("pickItem.hint"),
            m("i")
                .class("ph ph-check picker-check")
                .bind("class", &format!("({selected} ? 'is-on' : '')")),
        ])];
    if !p.on_edit.is_empty() {
        row.push(
            m("button")
                .class("picker-edit")
                .attr("type", "button")
                .attr("title", "Edit")
                // Without `.stop` the pencil would also pick the row, so
                // editing a thing would silently switch to it first.
                .on("click.stop", &p.on_edit.replace("{}", "pickItem.id"))
                .child(Node::icon("pencil-simple")),
        );
    }
    m("div")
        .class("picker-row")
        .for_each(&format!("pickItem in $store.{STORE}.match({})", p.items))
        .key("pickItem.id")
        .kids(row)
}

/// The null row — an answer, not the absence of one.
fn null_row(p: &Picker) -> Node {
    let (label, hint) = p.null.unwrap_or(("None", ""));
    let mut row = m("div")
        .class("picker-row")
        // Hidden while searching: it is not a match for anything typed, and
        // leaving it pinned above the results reads as a stuck result.
        .show(&format!("!$store.{STORE}.query"))
        .child(
            m("button")
                .class("picker-item")
                .attr("type", "button")
                .on("click", &p.on_pick.replace("{}", "''"))
                .kids(vec![
                    m("span")
                        .class("picker-tile")
                        .child(Node::icon(p.null_icon)),
                    m("span").class("picker-item-label").child(lit(label)),
                    m("span").class("picker-hint").child(lit(hint)),
                    m("i")
                        .class("ph ph-check picker-check")
                        .bind("class", &format!("(!{} ? 'is-on' : '')", p.value)),
                ]),
        );
    if !p.on_edit.is_empty() {
        row = row.child(m("span").class("picker-gutter"));
    }
    row
}

/// The picker: `.dropdown`'s trigger, this file's menu.
pub fn picker(p: &Picker) -> Node {
    let is_open = format!("$store.{DD}.open === '{}'", p.key);

    let mut menu = vec![
        // Search. `x-model` on the shared query is safe because only one
        // menu is open at a time — the open one owns the box.
        m("div").class("picker-search").kids(vec![
            Node::icon("magnifying-glass"),
            m("input")
                .attr("type", "text")
                .attr("spellcheck", "false")
                .attr("placeholder", p.search)
                .model(&format!("$store.{STORE}.query")),
        ]),
        m("div").class("picker-list").kids(vec![
            null_row(p),
            item_row(p),
            m("p")
                .class("picker-empty")
                .show(&format!(
                    "$store.{STORE}.match({}).length === 0",
                    p.items
                ))
                .text(&format!(
                    "($store.{STORE}.query ? 'No matches' : 'Nothing here yet')"
                )),
        ]),
    ];
    if !p.actions.is_empty() {
        menu.push(
            m("div").class("picker-actions").kids(
                p.actions
                    .iter()
                    .map(|a| {
                        m("button")
                            .class("picker-action")
                            .attr("type", "button")
                            // Every action leaves for a form, so every action
                            // closes the menu. Spelled once, here.
                            .on("click", &format!("$store.{STORE}.close(); {}", a.run))
                            .kids(vec![
                                Node::icon(a.icon),
                                m("span").child(lit(a.label)),
                            ])
                    })
                    .collect(),
            ),
        );
    }

    m("div")
        .class(&format!("dropdown picker {}", p.extra))
        .bind("class", &format!("({is_open} ? 'open' : '')"))
        .on("click.outside", &format!("$store.{DD}.closeIf('{}')", p.key))
        .kids(vec![
            m("button")
                .class("dropdown-btn")
                .attr("type", "button")
                .on("click", &format!("$store.{STORE}.openAt('{}')", p.key))
                .kids(vec![
                    m("i").class(&format!("ph ph-{} dropdown-icon", p.icon)),
                    m("span").class("dropdown-value").text(&trigger_label(p)),
                    // The screenshot's stacked caret: this control cycles
                    // through a list, it does not only drop down.
                    m("i").class("ph ph-caret-up-down dropdown-caret"),
                ]),
            m("div").class("picker-menu").show(&is_open).kids(menu),
        ])
}

// ── the three instances ──────────────────────────────────────────────
//
// Named builders beside the generic one, as `dropdown::model` /
// `::level` / `::policy` already are. The composer asks for a project
// picker; it does not spell out what a project picker is.
//
// There is deliberately no sandbox picker (PLAN55 D2): sandboxes belong to
// agents, one per (session → agent), launched at the agent's first turn.
// There is nothing for a session to pick.

/// Which folder this conversation is filed under.
pub fn project(st: &str) -> Node {
    picker(&Picker {
        key: "project",
        icon: "brackets-angle",
        extra: "dd-project",
        search: "Search projects…",
        items: &format!("{st}.projects"),
        value: &format!("{st}.projectId"),
        // Straight from the reference: the hint says what "no project"
        // leaves you with, rather than leaving it to be inferred.
        null: Some(("No project", "Session")),
        null_icon: "cube",
        on_pick: &format!("{st}.pickTarget('project', {{}})"),
        on_edit: &format!("{st}.editTarget('project', {{}})"),
        actions: &[Action {
            icon: "folder-plus",
            label: "Create new project",
            run: &format!("{st}.createTarget('project')"),
        }],
    })
}

/// Which checkout the sandbox mounts.
///
/// The null row is the whole "local" strategy: no worktree, mount the clone
/// itself. So the two choices from the original sketch (worktree | local)
/// survive as "a worktree" and "no worktree", and the list in between names
/// *which* worktree — which the sketch had nowhere to say.
pub fn worktree(st: &str) -> Node {
    picker(&Picker {
        key: "worktree",
        icon: "tree-structure",
        extra: "dd-worktree",
        search: "Search worktrees…",
        items: &format!("{st}.worktrees"),
        value: &format!("{st}.worktreeId"),
        null: Some(("Local", "no worktree")),
        null_icon: "hard-drives",
        on_pick: &format!("{st}.pickTarget('worktree', {{}})"),
        on_edit: &format!("{st}.editTarget('worktree', {{}})"),
        actions: &[Action {
            icon: "plus",
            label: "Create new worktree",
            run: &format!("{st}.createTarget('worktree')"),
        }],
    })
}

/// Which repo and branch the sandbox mounts.
///
/// One control, two facts: the name is the branch, the hint is the repo it
/// belongs to. The field is `repoBranches`, not `branches`: both stores
/// already use `branches` for the *conversation's* branches, and the second
/// definition silently won. `Open from folder` is how a repo gets into the list at all —
/// there is no scan, so an absolute path typed into that form is the only
/// way a clone becomes selectable.
pub fn branch(st: &str) -> Node {
    picker(&Picker {
        key: "branch",
        icon: "git-branch",
        extra: "dd-branch",
        search: "Search branches…",
        items: &format!("{st}.repoBranches"),
        value: &format!("{st}.branchId"),
        null: Some(("No repo", "nothing mounted")),
        null_icon: "git-branch",
        on_pick: &format!("{st}.pickTarget('branch', {{}})"),
        on_edit: &format!("{st}.editTarget('branch', {{}})"),
        actions: &[
            Action {
                icon: "plus",
                label: "Create new branch",
                run: &format!("{st}.createTarget('branch')"),
            },
            Action {
                icon: "folder-open",
                label: "Open from folder",
                run: &format!("{st}.createTarget('repo')"),
            },
        ],
    })
}

/// The row the composer puts under its envelope: the three, in order.
///
/// Reading left to right it narrows — the filing, the checkout
/// strategy, the checkout — which is also the order in which a later choice
/// depends on an earlier one.
pub fn target_row(st: &str) -> Node {
    m("div").class("composer-targets").kids(vec![
        project(st),
        worktree(st),
        branch(st),
    ])
}

// ── store ───────────────────────────────────────────────────────────────

/// `openAt(key)`: open this menu (or close it if it is already the open
/// one), and clear whatever was typed into the last one.
///
/// A composite of two stores: open-state belongs to `dropdown`, because the
/// plain dropdowns share it and only one menu may be open at a time; the
/// query belongs here. Doing both in one method is what keeps a stale filter
/// from greeting the next menu that opens.
fn open_at(_this: JsValue, args: js_sys::Array) -> JsValue {
    let key = js::arg_str(&args, 0);
    let dd = mjs::store_ref(DD);
    let now = js::get_str(&dd, "open");
    js::set_str(&dd, "open", if now == key { "" } else { &key });
    js::set_str(&mjs::store_ref(STORE), "query", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

/// `close()`: dismiss whatever is open, and forget the search.
fn close(_this: JsValue, _args: js_sys::Array) -> JsValue {
    js::set_str(&mjs::store_ref(DD), "open", "");
    js::set_str(&mjs::store_ref(STORE), "query", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

/// Does this item match the current query? Case-insensitive, over the name
/// and the hint — searching branches by their repo is the obvious thing to
/// try, and the repo is in the hint.
fn hit(item: &JsValue, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let name = js::get_str(item, "name").to_lowercase();
    let hint = js::get_str(item, "hint").to_lowercase();
    name.contains(needle) || hint.contains(needle)
}

/// `match(items)`: the list as the open menu should show it.
///
/// Filtering in the store rather than in a template expression means the
/// four pickers cannot drift into four different ideas of what "matches"
/// means — and the empty-state check calls the same function the list does,
/// so "No matches" cannot disagree with the rows above it.
fn match_items(_this: JsValue, args: js_sys::Array) -> JsValue {
    let list = args.get(0);
    let out = js_sys::Array::new();
    let Ok(arr) = list.dyn_into::<js_sys::Array>() else {
        return out.into();
    };
    let needle = js::get_str(&mjs::store_ref(STORE), "query")
        .trim()
        .to_lowercase();
    for item in arr.iter() {
        if hit(&item, &needle) {
            out.push(&item);
        }
    }
    out.into()
}

/// `labelOf(items, id, fallback)`: what the trigger reads.
///
/// An id with no row — a sandbox destroyed from elsewhere, a branch deleted
/// on disk — falls back rather than showing a raw id, which is the one case
/// where the trigger would otherwise leak a key at the user.
fn label_of(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 1);
    let fallback = js::arg_str(&args, 2);
    if !id.is_empty() {
        if let Ok(arr) = args.get(0).dyn_into::<js_sys::Array>() {
            for item in arr.iter() {
                if js::get_str(&item, "id") == id {
                    return JsValue::from_str(&js::get_str(&item, "name"));
                }
            }
        }
    }
    JsValue::from_str(&fallback)
}

/// standalone demo — the four, over the samples store, open-able.
///
/// The top padding is not decoration: the menu opens *upward*, as it does in
/// the composer, so a demo flush to the top of the page would draw it off
/// screen.
pub fn demo() -> Node {
    m("div")
        .class("picker-demo")
        .child(target_row(crate::components::samples::EXPR))
}

pub fn register() {
    css::inject("picker", CSS);
    let mut d = Def::new().named("store picker");
    d.str("query", "")
        .method("openAt", open_at)
        .method("close", close)
        .method("match", match_items)
        .method("labelOf", label_of);
    mjs::store(STORE, &d.finish());
}

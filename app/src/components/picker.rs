use wasm_bindgen::{JsCast, JsValue};

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::css;

const STORE: &str = "picker";
const DD: &str = "dropdown";

pub const CSS: &str = r#"
.picker .picker-menu {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 0;
  z-index: 40;
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
:is(#sb, #home) .picker-search input:focus {
  border: 0;
  outline: none;
  box-shadow: none;
}

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
.picker-hint {
  flex: 0 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--gray-dim);
  font-size: 12px;
  margin-left: 8px;
}
.picker-check {
  flex: 0 0 auto;
  width: 14px;
  font-size: 14px;
  color: var(--text);
  margin-left: 8px;
  visibility: hidden;
}
.picker-check.is-on { visibility: visible; }

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
.picker-gutter { flex: 0 0 auto; width: 26px; margin-right: 6px; }
.picker-edit:focus-visible { visibility: visible; outline: 2px solid var(--accent); outline-offset: -2px; }
.picker-edit:hover { background: var(--panel); color: var(--text); }
.picker-edit .ph { font-size: 13px; }

.picker-empty {
  margin: 0;
  padding: 14px 10px;
  text-align: center;
  color: var(--gray-dim);
  font-size: 12px;
}

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
.picker-action .ph { font-size: 15px; width: 26px; text-align: center; flex: 0 0 auto; }

.picker-demo { display: flex; align-items: flex-end; gap: 14px; flex-wrap: wrap; padding-top: 320px; }
"#;

pub struct Action<'a> {
    pub icon: &'a str,
    pub label: &'a str,
    pub run: &'a str,
}

pub struct Picker<'a> {
    pub key: &'a str,
    pub icon: &'a str,
    pub extra: &'a str,
    pub search: &'a str,
    pub items: &'a str,
    pub value: &'a str,
    pub null: Option<(&'a str, &'a str)>,
    pub null_icon: &'a str,
    pub on_pick: &'a str,
    pub on_edit: &'a str,
    pub actions: &'a [Action<'a>],
}

fn trigger_label(p: &Picker) -> String {
    let fallback = p.null.map(|(l, _)| l).unwrap_or("—");
    format!(
        "$store.{STORE}.labelOf({}, {}, '{}')",
        p.items,
        p.value,
        fallback.replace('\'', "\\'")
    )
}

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

fn null_row(p: &Picker) -> Node {
    let (label, hint) = p.null.unwrap_or(("None", ""));
    let mut row = m("div")
        .class("picker-row")
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

pub fn picker(p: &Picker) -> Node {
    let is_open = format!("$store.{DD}.open === '{}'", p.key);

    let mut menu = vec![
        m("div").class("picker-search").kids(vec![
            Node::icon("magnifying-glass"),
            m("input")
                .attr("type", "text")
                .attr("spellcheck", "false")
                .attr("placeholder", p.search)
                .model(&format!("$store.{STORE}.query")),
        ]),
        m("div").class("picker-list").kids({
            let mut rows: Vec<Node> = Vec::new();
            if p.null.is_some() {
                rows.push(null_row(p));
            }
            rows.push(item_row(p));
            rows.push(
                m("p")
                    .class("picker-empty")
                    .show(&format!(
                        "$store.{STORE}.match({}).length === 0",
                        p.items
                    ))
                    .text(&format!(
                        "($store.{STORE}.query ? 'No matches' : 'Nothing here yet')"
                    )),
            );
            rows
        }),
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
                    m("i").class("ph ph-caret-up-down dropdown-caret"),
                ]),
            m("div").class("picker-menu").show(&is_open).kids(menu),
        ])
}


pub fn project(st: &str) -> Node {
    picker(&Picker {
        key: "project",
        icon: "brackets-angle",
        extra: "dd-project",
        search: "Search projects…",
        items: &format!("{st}.projects"),
        value: &format!("{st}.projectId"),
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

pub fn repo(st: &str, draft: &str) -> Node {
    picker(&Picker {
        key: "repo",
        icon: "folder",
        extra: "dd-repo",
        search: "Search repos…",
        items: &format!("{st}.repos"),
        value: &format!("{draft}.worktreeRepoId"),
        null: Some(("No repo", "no worktree")),
        null_icon: "folder",
        on_pick: &format!("{draft}.pickRepo({{}})"),
        on_edit: "",
        actions: &[Action {
            icon: "folder-open",
            label: "Open from folder",
            run: &format!("{st}.createTarget('repo')"),
        }],
    })
}

pub fn worktree(st: &str) -> Node {
    picker(&Picker {
        key: "worktree",
        icon: "git-fork",
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

pub fn target_row(st: &str) -> Node {
    m("div").class("composer-targets").kids(vec![
        project(st),
        worktree(st),
        branch(st),
    ])
}


fn open_at(_this: JsValue, args: js_sys::Array) -> JsValue {
    let key = js::arg_str(&args, 0);
    let dd = mjs::store_ref(DD);
    let now = js::get_str(&dd, "open");
    js::set_str(&dd, "open", if now == key { "" } else { &key });
    js::set_str(&mjs::store_ref(STORE), "query", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

fn close(_this: JsValue, _args: js_sys::Array) -> JsValue {
    js::set_str(&mjs::store_ref(DD), "open", "");
    js::set_str(&mjs::store_ref(STORE), "query", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

fn hit(item: &JsValue, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let name = js::get_str(item, "name").to_lowercase();
    let hint = js::get_str(item, "hint").to_lowercase();
    name.contains(needle) || hint.contains(needle)
}

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

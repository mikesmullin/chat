use wasm_bindgen::{JsCast, JsValue};

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::components::rail_head;
use crate::components::session_state;
use crate::css;

const STORE: &str = "ptree";

pub const CSS: &str = r#"
.project-tree { display: flex; flex-direction: column; }

.ptree-list { display: flex; flex-direction: column; gap: 1px; }

.ptree-row {
  display: flex; align-items: center; gap: 6px;
  min-width: 0;
  padding: 4px 6px;
  padding-left: calc(6px + var(--d, 0) * 14px);
  border-radius: 7px;
  border: 1px solid transparent;
  color: var(--gray);
  font-size: 12px;
  line-height: 18px;
  cursor: pointer;
  user-select: none;
}
.ptree-row:hover { background: var(--panel-2); color: var(--fg); }

.ptree-caret {
  flex: 0 0 auto;
  width: 12px;
  font-size: 10px;
  color: var(--gray-dim);
  transition: transform .13s;
}
.ptree-row.is-leaf .ptree-caret { visibility: hidden; }
.ptree-row.is-open .ptree-caret { transform: rotate(90deg); }

.ptree-icon { flex: 0 0 auto; font-size: 13px; color: var(--gray-dim); }
.ptree-name {
  flex: 1 1 auto; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) input.ptree-edit {
  flex: 1 1 auto; min-width: 0; width: 0;
  box-sizing: border-box; min-height: 0; height: 18px;
  padding: 0; border: 0; border-radius: 0;
  font: inherit; line-height: inherit; color: inherit;
  background: transparent; user-select: text;
}
:is(#sb, #home) input.ptree-edit:focus {
  outline: none; box-shadow: inset 0 -1px var(--gray-dim);
}
.ptree-time {
  flex: 0 0 auto;
  font-size: 11px;
  color: var(--gray-dim);
  font-variant-numeric: tabular-nums;
}
.ptree-time .ui-reltime { min-width: 0; font: inherit; color: inherit; }
@media (max-width: 900px) {
  .ptree-row { justify-content: center; padding: 4px 0; }
  .ptree-row .ptree-caret,
  .ptree-row .ptree-name,
  .ptree-row .ptree-time { display: none; }
}

.ptree-row.is-session .ptree-name { color: var(--gray); }
.ptree-row.is-session.is-active {
  background: var(--accent-tint);
  border-color: color-mix(in srgb, var(--accent) 40%, transparent);
}
.ptree-row.is-session.is-active .ptree-name { color: var(--fg); }
.ptree-row.is-unread .ptree-name { font-weight: bold; }

.ptree-row.is-dragging { opacity: .4; }
.ptree-row.is-drop {
  background: var(--accent-glow);
  border-color: var(--accent);
  color: var(--fg);
}
.ptree-row.is-drop .ptree-icon { color: var(--accent); }

.ptree-empty {
  margin: 0; padding: 10px 6px;
  color: var(--gray-dim); font-size: 11px;
}

.ptree-demo { width: 280px; }
"#;

pub fn tree(st: &str, on_open: &str, on_new: &str) -> Node {
    let rows = format!("$store.{STORE}.rows({st}.projects, {st}.sessions)");
    m("div").class("project-tree").bind("class", "($store.home.projectsOpen ? '' : 'is-collapsed')").kids(vec![
        rail_head::head("Projects", "folders", Some(("New project", on_new, "$store.ident.can('project.create')")), Some("projects")),
        m("div")
            .class("ptree-list")
            .show("$store.home.projectsOpen")
            .kids(vec![
            row(st, on_open),
            m("p")
                .class("ptree-empty")
                .show(&format!("{rows}.length === 0"))
                .child(lit("No projects yet.")),
        ]),
    ])
}

fn row(st: &str, on_open: &str) -> Node {
    let rows = format!("$store.{STORE}.rows({st}.projects, {st}.sessions)");
    let click = format!(
        "(r.kind === 'session' ? {} : (r.kind === 'unfiled' && $store.{STORE}.toggle(r.drop)))",
        on_open.replace("{}", "r.id")
    );
    let group_click = format!(
        "(r.kind === 'project' && {st}.openTreeFolder(r.kind, r.drop, '{}'))",
        st.trim_start_matches("$store.")
    );
    m("div")
        .class("ptree-row")
        .for_each(&format!("r in {rows}"))
        .key("r.key")
        .bind("style", "('--d:' + r.depth)")
        .bind(
            "class",
            &format!(
                "((r.kind === 'session' ? 'is-session' : '') \
                 + (r.leaf ? ' is-leaf' : '') \
                 + (r.open ? ' is-open' : '') \
                 + (r.id && r.id === {st}.activeId ? ' is-active' : '') \
                 + (r.kind === 'session' && {st}.unreadSessions.includes(r.id) ? ' is-unread' : '') \
                 + ($store.{STORE}.dragId === r.id && r.kind === 'session' ? ' is-dragging' : '') \
                 + ($store.{STORE}.dropName === r.drop && r.kind !== 'session' ? ' is-drop' : ''))"
            ),
        )
        .bind("title", "r.name")
        .bind("draggable", "(r.kind === 'session' ? 'true' : 'false')")
        .on("click", &click)
        .on("dblclick", &format!("r.kind !== 'unfiled' && $store.{STORE}.edit(r.key, r.label)"))
        .on("dragstart", &format!("$store.{STORE}.dragStart(r.id)"))
        .on("dragend", &format!("$store.{STORE}.dragEnd()"))
        .on(
            "dragover",
            &format!(
                "$event.preventDefault(); \
                 (r.kind !== 'session' && $store.{STORE}.over(r.drop))"
            ),
        )
        .on("dragleave", &format!("$store.{STORE}.out(r.drop)"))
        .on(
            "drop",
            &format!(
                "$event.preventDefault(); \
                 (r.kind !== 'session' && {st}.dropSession($store.{STORE}.take(), r.drop))"
            ),
        )
        .kids(vec![
            m("i").class("ph ph-caret-right ptree-caret")
                .on("click", &format!("$event.stopPropagation(); $store.{STORE}.toggle(r.drop)")),
            Node::icon_dyn("r.icon").class("ph ptree-icon").on("click", &group_click),
            m("span").class("ptree-name")
                .on("click", &group_click)
                .show(&format!("$store.{STORE}.editing !== r.key"))
                .bind("title", "r.name")
                .text("r.label"),
            m("input")
                .class("ptree-edit")
                .attr("type", "text")
                .attr("aria-label", "Edit name")
                .iff(&format!("$store.{STORE}.editing === r.key"))
                .model(&format!("$store.{STORE}.editDraft"))
                .init("$el.focus(); $el.select()")
                .on("click.stop", "")
                .on("dblclick.stop", "")
                .on("keydown.enter", &format!("$event.preventDefault(); $store.{STORE}.editing === r.key && ({st}.renameTreeRow(r.kind, r.name, r.id, $el.value), $store.{STORE}.finish())"))
                .on("keydown.escape", &format!("$store.{STORE}.finish()"))
                .on("focusout", &format!("$store.{STORE}.editing === r.key && ({st}.renameTreeRow(r.kind, r.name, r.id, $el.value), $store.{STORE}.finish())")),
            Node::host("uiSessionState", &[])
                .bind("tone", &session_state::tone_of("r"))
                .bind("title", &session_state::tone_of("r"))
                .iff("r.kind === 'session'"),
            Node::host("uiRelTime", &[])
                .class("ptree-time relative-timestamp")
                .bind("at", "r.lastEventAt")
                .bind("title", &stamp_hint("r"))
                .iff("r.lastEventAt"),
        ])
}


fn folders(projects: &js_sys::Array) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in projects.iter() {
        let name = js::get_str(&p, "name");
        let mut acc = String::new();
        for seg in name.split('/').filter(|s| !s.is_empty()) {
            if !acc.is_empty() {
                acc.push('/');
            }
            acc.push_str(seg);
            if !out.contains(&acc) {
                out.push(acc.clone());
            }
        }
    }
    out.sort();
    out
}

fn row_obj(kind: &str, key: &str, depth: usize) -> JsValue {
    let o = js::obj();
    js::set_str(&o, "kind", kind);
    js::set_str(&o, "key", key);
    js::set_num(&o, "depth", depth as f64);
    o.into()
}

fn rows(_this: JsValue, args: js_sys::Array) -> JsValue {
    let out = js_sys::Array::new();
    let projects = args
        .get(0)
        .dyn_into::<js_sys::Array>()
        .unwrap_or_else(|_| js_sys::Array::new());
    let mut sessions: Vec<JsValue> = args
        .get(1)
        .dyn_into::<js_sys::Array>()
        .unwrap_or_else(|_| js_sys::Array::new())
        .iter()
        .collect();
    sessions.sort_by(|a, b| {
        let time = |s: &JsValue| {
            let iso = session_event_time(s);
            let n = js_sys::Date::parse(&iso);
            if n.is_finite() { n } else { f64::NEG_INFINITY }
        };
        time(b).total_cmp(&time(a)).then_with(|| js::get_str(a, "id").cmp(&js::get_str(b, "id")))
    });

    let st = mjs::store_ref(STORE);
    let collapsed: Vec<String> = js::get(&st, "collapsed")
        .dyn_into::<js_sys::Array>()
        .map(|a| a.iter().map(|v| v.as_string().unwrap_or_default()).collect())
        .unwrap_or_default();
    let is_collapsed = |name: &str| collapsed.iter().any(|c| c == name);
    let hidden_under = |name: &str| {
        collapsed
            .iter()
            .any(|c| name.len() > c.len() && name.starts_with(&format!("{c}/")))
    };

    let session_project = |s: &JsValue| js::get_str(s, "project");

    for name in folders(&projects) {
        if hidden_under(&name) {
            continue;
        }
        let depth = name.matches('/').count();
        let shut = is_collapsed(&name);
        let label = name.rsplit('/').next().unwrap_or(&name).to_string();
        let mut n = 0usize;
        for s in sessions.iter() {
            if session_project(&s) == name {
                n += 1;
            }
        }
        let has_child_folder = folders(&projects)
            .iter()
            .any(|f| f.len() > name.len() && f.starts_with(&format!("{name}/")));
        let r = row_obj("project", &format!("p:{name}"), depth);
        js::set_str(&r, "name", &name);
        js::set_str(&r, "label", &label);
        js::set_str(&r, "drop", &name);
        js::set_str(&r, "id", "");
        js::set_str(&r, "icon", if shut { "folder" } else { "folder-open" });
        js::set_bool(&r, "open", !shut);
        js::set_bool(&r, "leaf", n == 0 && !has_child_folder);
        out.push(&r);

        if shut {
            continue;
        }
        for s in sessions.iter() {
            if session_project(&s) != name {
                continue;
            }
            out.push(&session_row(&s, depth + 1));
        }
    }

    let loose: Vec<JsValue> = sessions
        .iter()
        .filter(|s| session_project(s).is_empty())
        .cloned()
        .collect();
    if !loose.is_empty() {
        let shut = is_collapsed("");
        let r = row_obj("unfiled", "p:__unfiled", 0);
        js::set_str(&r, "name", "(ungrouped)");
        js::set_str(&r, "label", "(ungrouped)");
        js::set_str(&r, "drop", "");
        js::set_str(&r, "id", "");
        js::set_str(&r, "icon", "tray");
        js::set_bool(&r, "open", !shut);
        js::set_bool(&r, "leaf", false);
        out.push(&r);
        if !shut {
            for s in &loose {
                out.push(&session_row(s, 1));
            }
        }
    }
    out.into()
}

fn session_event_time(s: &JsValue) -> String {
    let last = js::get_str(s, "lastEventAt");
    if last.is_empty() { js::get_str(s, "createdAt") } else { last }
}

fn stamp_hint(row: &str) -> String {
    format!(
        "'created ' + {created} + ' modified ' + {modified}",
        created = long_stamp(&format!("{row}.createdAt")),
        modified = long_stamp(&format!("{row}.updatedAt")),
    )
}

fn long_stamp(iso: &str) -> String {
    format!(
        "(({iso}) ? (new Date({iso}).toLocaleDateString('en-US', \
         {{weekday: 'short', month: 'short', day: 'numeric'}}) + ' ' + \
         new Date({iso}).toLocaleTimeString('en-US', \
         {{hour: 'numeric', minute: '2-digit', timeZoneName: 'short'}}) \
         .replace(' ', '')) : '—')"
    )
}

fn session_row(s: &JsValue, depth: usize) -> JsValue {
    let id = js::get_str(s, "id");
    let title = js::get_str(s, "title");
    let r = row_obj("session", &format!("s:{id}"), depth);
    js::set_str(&r, "id", &id);
    js::set_str(&r, "name", &title);
    js::set_str(
        &r,
        "label",
        if title.trim().is_empty() {
            "Untitled"
        } else {
            &title
        },
    );
    js::set_str(&r, "drop", "");
    js::set_str(&r, "icon", "chat-circle");
    js::set_str(&r, "lastEventAt", &session_event_time(s));
    js::set_str(&r, "status", &js::get_str(s, "status"));
    let turns = js::get(s, "turns");
    if !turns.is_undefined() && !turns.is_null() {
        js::set_num(&r, "turns", turns.as_f64().unwrap_or(0.0));
    }
    js::set(&r, "createdAt", &js::get(s, "createdAt"));
    js::set(&r, "updatedAt", &js::get(s, "updatedAt"));
    js::set_bool(&r, "open", false);
    js::set_bool(&r, "leaf", true);
    r
}


const COLLAPSED_KEY: &str = "chat.ptree.collapsed";

fn collapsed_from_storage() -> JsValue {
    let storage = js::get(&js::global(), "localStorage");
    let text = js::call(&storage, "getItem", &[JsValue::from_str(COLLAPSED_KEY)])
        .as_string()
        .unwrap_or_default();
    let out = js_sys::Array::new();
    if !text.is_empty() {
        let json = js::get(&js::global(), "JSON");
        let parsed = js::call(&json, "parse", &[JsValue::from_str(&text)]);
        if js_sys::Array::is_array(&parsed) {
            for v in js_sys::Array::from(&parsed).iter() {
                if let Some(s) = v.as_string() {
                    out.push(&JsValue::from_str(&s));
                }
            }
        }
    }
    out.into()
}

fn collapsed_to_storage(collapsed: &JsValue) {
    let storage = js::get(&js::global(), "localStorage");
    let json = js::get(&js::global(), "JSON");
    let text = js::call(&json, "stringify", &[collapsed.clone()]);
    js::call(
        &storage,
        "setItem",
        &[JsValue::from_str(COLLAPSED_KEY), text],
    );
}

fn toggle(_this: JsValue, args: js_sys::Array) -> JsValue {
    let name = js::arg_str(&args, 0);
    let st = mjs::store_ref(STORE);
    let cur = js::get(&st, "collapsed")
        .dyn_into::<js_sys::Array>()
        .unwrap_or_else(|_| js_sys::Array::new());
    let next = js_sys::Array::new();
    let mut found = false;
    for v in cur.iter() {
        if v.as_string().unwrap_or_default() == name {
            found = true;
        } else {
            next.push(&v);
        }
    }
    if !found {
        next.push(&JsValue::from_str(&name));
    }
    let merged: JsValue = next.into();
    js::set(&st, "collapsed", &merged);
    collapsed_to_storage(&merged);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn edit(_this: JsValue, args: js_sys::Array) -> JsValue {
    let st = mjs::store_ref(STORE);
    js::set_str(&st, "editDraft", &js::arg_str(&args, 1));
    js::set_str(&st, "editing", &js::arg_str(&args, 0));
    mjs::redraw();
    JsValue::UNDEFINED
}

fn finish(_this: JsValue, _args: js_sys::Array) -> JsValue {
    js::set_str(&mjs::store_ref(STORE), "editing", "");
    mjs::redraw();
    JsValue::UNDEFINED
}

fn drag_start(_this: JsValue, args: js_sys::Array) -> JsValue {
    js::set_str(&mjs::store_ref(STORE), "dragId", &js::arg_str(&args, 0));
    mjs::redraw();
    JsValue::UNDEFINED
}

fn drag_end(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let st = mjs::store_ref(STORE);
    js::set_str(&st, "dragId", "");
    js::set_str(&st, "dropName", "\u{0}");
    mjs::redraw();
    JsValue::UNDEFINED
}

fn over(_this: JsValue, args: js_sys::Array) -> JsValue {
    let st = mjs::store_ref(STORE);
    if js::get_str(&st, "dragId").is_empty() {
        return JsValue::UNDEFINED;
    }
    let name = js::arg_str(&args, 0);
    if js::get_str(&st, "dropName") != name {
        js::set_str(&st, "dropName", &name);
        mjs::redraw();
    }
    JsValue::UNDEFINED
}

fn out(_this: JsValue, args: js_sys::Array) -> JsValue {
    let st = mjs::store_ref(STORE);
    if js::get_str(&st, "dropName") == js::arg_str(&args, 0) {
        js::set_str(&st, "dropName", "\u{0}");
        mjs::redraw();
    }
    JsValue::UNDEFINED
}

fn take(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let st = mjs::store_ref(STORE);
    let id = js::get_str(&st, "dragId");
    js::set_str(&st, "dragId", "");
    js::set_str(&st, "dropName", "\u{0}");
    JsValue::from_str(&id)
}

pub fn demo() -> Node {
    let st = crate::components::samples::EXPR;
    m("div").class("ptree-demo").child(tree(
        st,
        &format!("{st}.previewSession({{}})"),
        &format!("{st}.createTarget('project')"),
    ))
}

pub fn register() {
    css::inject("project-tree", CSS);
    let mut d = Def::new().named("store ptree");
    d.val("collapsed", &collapsed_from_storage())
        .str("editing", "")
        .str("editDraft", "")
        .str("dragId", "")
        .str("dropName", "\u{0}")
        .method("rows", rows)
        .method("toggle", toggle)
        .method("edit", edit)
        .method("finish", finish)
        .method("dragStart", drag_start)
        .method("dragEnd", drag_end)
        .method("over", over)
        .method("out", out)
        .method("take", take);
    mjs::store(STORE, &d.finish());
}

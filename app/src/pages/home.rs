use wasm_bindgen::JsValue;

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::components::{
    agent_editor, agents_menu, browser_pane, identity_switcher, job_panel, jobs_menu, lightbox,
    mode_toggle, perf_badge, pipelines_menu, profile_panel, project_tree, pr_panel, prs_menu,
    repo_panel, repos_menu, run_panel, sandboxes_menu, session_editor, session_panel, share_dialog,
    tab, target_form, worktrees_menu,
};
use crate::css;

const LIVE: &str = session_panel::LIVE_STORE;

const STORE: &str = "home";

pub const CSS: &str = r#"
#home {
  display: grid;
  grid-template-columns: var(--rail-w, 300px) 1fr var(--aside-w, 320px);
  height: 100vh;
  height: 100dvh;
  min-height: 0;
  position: relative;
  background: var(--bg);
  color: var(--text);
  font-family: var(--sans, ui-sans-serif, system-ui, sans-serif);
  transition: grid-template-columns .3s ease-in-out;
}
#home.home-resizing { transition: none; }
#home.aside-collapsed { grid-template-columns: var(--rail-w, 300px) 1fr 0px; }
#home.aside-collapsed .home-aside { display: none; }
#home.rail-collapsed { grid-template-columns: 0px 1fr var(--aside-w, 320px); }
#home.rail-collapsed .home-rail { display: none; }
#home.rail-collapsed .home-col { margin-left: 10px; }
#home.rail-collapsed.aside-collapsed { grid-template-columns: 0px 1fr 0px; }

.home-gutter {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 7px;
  cursor: col-resize;
  touch-action: none;
  user-select: none;
  z-index: 40;
  background: transparent;
}
.home-gutter:hover, .home-gutter:active { background: var(--accent); opacity: .4; }
.home-gutter.g-left { left: calc(var(--rail-w, 300px) - 3px); }
.home-gutter.g-right { right: calc(var(--aside-w, 320px) - 3px); }
#home.rail-collapsed .home-gutter.g-left { display: none; }
#home.aside-collapsed .home-gutter.g-right { display: none; }
.home-rail { grid-column: 1; }

.home-rail { --group-gap: 8px; }
.home-col { grid-column: 2; }
.home-aside { grid-column: 3; }

.home-rail {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--group-gap);
  overflow: hidden;
  padding: 10px 0 10px 4px;
  margin-right: 10px;
}
.home-rail-content {
  flex: 1 1 auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--group-gap);
  overflow-y: auto;
}
.home-agents { padding: 0 14px; }
.home-sandboxes { padding: 0 14px; }
.home-automations { padding: 0 14px; }
.home-repos { padding: 0 14px; }
.home-worktrees { padding: 0 14px; }
.home-pipelines { padding: 0 14px; }
.home-prs { padding: 0 14px; }
.home-jobs { padding: 0 14px; }

.home-col {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  margin-right: 10px;
}

.home-tabs {
  flex: 0 0 auto;
  display: flex; align-items: center; gap: 6px;
  padding: 10px 2px 8px;
}
.home-tabs .tab-bar {
  flex: 1 1 auto; min-width: 0;
  display: flex; overflow-x: auto; overflow-y: hidden;
}
.tabs-toggle {
  flex: 0 0 auto;
  width: 30px; height: 30px;
  display: inline-flex; align-items: center; justify-content: center;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--muted);
  font-size: 16px;
  cursor: pointer;
  transition: color .13s, border-color .13s, background-color .13s;
}
.tabs-toggle:hover { color: var(--text); background: var(--panel-2); border-color: var(--border); }
.tabs-toggle:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
#home.aside-collapsed .tabs-toggle.for-aside .ph { transform: scaleX(-1); }
#home.rail-collapsed .tabs-toggle.for-rail .ph { transform: scaleX(-1); }

.home-main,
.home-aside {
  min-width: 0;
  min-height: 0;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 14px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.home-main { flex: 1 1 auto; margin-bottom: 10px; }
.home-aside { margin: 10px 10px 10px 0; }

.home-main .mjs-chat { height: 100%; background: transparent; border-left: 0; }

.home-main .composer-inner { padding: 0 10px 10px; }
.home-main .messages { padding-left: 10px; padding-right: 10px; }

.home-aside-body { flex: 1 1 auto; min-height: 0; overflow-y: auto; padding: 12px 12px 12px; }
.home-aside-toggle-row { display: none; }

.home-projects { padding: 0 14px; }
.home-settings {
  flex: 0 0 auto; display: flex; flex-direction: column;
  gap: 0; padding-top: 0;
}
.home-settings-row { display: flex; align-items: center; gap: 6px; }
.home-settings-spacer { margin-left: auto; }

.home-rail-content > div { flex: 0 0 auto; }
.rail-settings {
  display: inline-flex; align-items: center; gap: 8px;
  flex: 1 1 auto; min-width: 0; border: 0; border-radius: 8px;
  background: transparent; color: var(--gray-dim);
  padding: 7px 8px; cursor: pointer; font: inherit; font-size: 13px;
  text-align: left;
}
.rail-settings:hover { background: var(--panel-2); color: var(--fg); }
.rail-settings i { font-size: 15px; line-height: 18px; }
#home.aside-collapsed .home-aside-body { display: none; }

@media (max-width: 1180px) {
  #home { grid-template-columns: var(--rail-w, 248px) minmax(0, 1fr) var(--aside-w, 288px); }
  #home.aside-collapsed { grid-template-columns: var(--rail-w, 248px) minmax(0, 1fr) 0px; }
  .home-gutter.g-left { left: calc(var(--rail-w, 248px) - 3px); }
  .home-gutter.g-right { right: calc(var(--aside-w, 288px) - 3px); }
}
@media (max-width: 1020px) {
  #home { grid-template-columns: var(--rail-w, 216px) minmax(0, 1fr) var(--aside-w, 256px); }
  #home.aside-collapsed { grid-template-columns: var(--rail-w, 216px) minmax(0, 1fr) 0px; }
  .home-gutter.g-left { left: calc(var(--rail-w, 216px) - 3px); }
  .home-gutter.g-right { right: calc(var(--aside-w, 256px) - 3px); }
}
@media (max-width: 900px) {
  .home-gutter { display: none; }
  #home,
  #home.aside-collapsed { grid-template-columns: 60px minmax(0, 1fr); }
  #home.rail-collapsed,
  #home.rail-collapsed.aside-collapsed { grid-template-columns: 0px minmax(0, 1fr); }
  .home-rail { padding: 10px 0; margin-right: 8px; gap: 12px; }
  .home-rail-content { gap: 12px; }
  .home-projects { padding: 10px 6px 4px; }
  .home-agents { padding: 0 6px 10px; }
  .home-sandboxes { padding: 0 6px 10px; }
  .home-automations { padding: 0 6px 10px; }
  .home-repos { padding: 0 6px 10px; }
  .home-worktrees { padding: 0 6px 10px; }
  .home-pipelines { padding: 0 6px 10px; }
  .home-prs { padding: 0 6px 10px; }
  .home-jobs { padding: 0 6px 10px; }
  .home-col { margin-right: 8px; isolation: isolate; }
  .home-aside-toggle-row {
    display: flex; justify-content: flex-end; flex: 0 0 auto;
    padding: 6px 8px 0;
  }
  .home-aside {
    position: fixed; top: 10px; right: 10px; bottom: 10px;
    width: min(340px, calc(100vw - 100px));
    margin: 0; z-index: 60;
    box-shadow: 0 16px 48px rgba(0, 0, 0, .55);
  }
  .rail-settings { justify-content: center; padding-left: 0; padding-right: 0; }
  .rail-settings span { display: none; }
}
"#;

fn rail() -> Node {
    m("aside")
        .class("home-rail")
        .attr("aria-label", "Projects, agents and automations")
        .kids(vec![
            m("div").class("home-rail-content").kids(vec![
                m("div").class("home-projects")
                    .show(&format!(
                        "($store.ptree.rows({LIVE}.projects, {LIVE}.sessions).length > 0 \
                          || $store.ident.can('project.create'))"
                    ))
                    .child(project_tree::tree(
                        LIVE,
                        &format!("{LIVE}.openSessionRow({{}})"),
                        &format!("{LIVE}.createTarget('project')"),
                    )),
                m("div").class("home-agents")
                    .show("($store.agents.agents.length > 0 || $store.ident.can('agent.create'))")
                    .child(agents_menu::agents_menu_live()),
                m("div").class("home-sandboxes")
                    .show(&format!("{LIVE}.sandboxGroups.length > 0"))
                    .child(sandboxes_menu::sandboxes_menu(LIVE)),
                m("div").class("home-repos")
                    .show("($store.vcs.repos.length > 0 || $store.ident.can('repo.manage'))")
                    .child(repos_menu::menu(LIVE)),
                m("div").class("home-worktrees")
                    .show("($store.vcs.worktrees.length > 0)")
                    .child(worktrees_menu::menu(LIVE)),
                m("div").class("home-prs")
                    .show("($store.vcs.prs.length > 0 || $store.ident.can('pr.create'))")
                    .child(prs_menu::menu()),
                m("div").class("home-pipelines")
                    .show("($store.jobs.runs.length > 0 || $store.jobs.jobs.length > 0)")
                    .child(pipelines_menu::menu()),
                m("div").class("home-jobs")
                    .show("($store.jobs.jobs.length > 0 || $store.live.automations.length > 0 || $store.ident.can('job.manage'))")
                    .child(jobs_menu::menu(LIVE)),
            ]),
            m("div").class("home-settings").kids(vec![
                m("div").class("home-settings-row").child(
                    m("button")
                        .class("rail-settings")
                        .attr("type", "button")
                        .attr("aria-label", "Settings")
                        .on("click", "$store.settings.openPanel()")
                        .kids(vec![
                            Node::icon("gear").class("ph ph-gear"),
                            m("span").child(lit("Settings")),
                        ]),
                ),
                m("div").class("home-settings-row").kids(vec![
                    identity_switcher::widget(),
                    m("span").class("home-settings-spacer"),
                    mode_toggle::button(),
                ]),
            ]),
        ])
}

fn aside_toggle() -> Node {
    m("button")
        .class("tabs-toggle for-aside")
        .attr("type", "button")
        .attr("title", "Toggle inspector")
        .attr("aria-label", "Toggle inspector")
        .attr("aria-controls", "home-aside")
        .bind("aria-expanded", "($store.home.asideOpen ? 'true' : 'false')")
        .on("click", "$store.home.toggleAside()")
        .child(Node::icon("sidebar-simple"))
}

fn main_col() -> Node {
    m("div").class("home-col").kids(vec![
        m("div").class("home-tabs").kids(vec![
                m("button")
                    .class("tabs-toggle for-rail")
                    .attr("type", "button")
                    .attr("title", "Toggle left sidebar")
                    .attr("aria-label", "Toggle left sidebar")
                    .bind("aria-expanded", "($store.home.railOpen ? 'true' : 'false')")
                    .on("click", "$store.home.toggleRail()")
                    .child(Node::icon("sidebar-simple")),
                tab::tab_bar(LIVE),
                perf_badge::badge().show("$store.mode.advanced"),
                aside_toggle(),
            ]),
        m("section")
            .class("home-main")
            .show("$store.live.tabs.length > 0")
            .child(session_panel::session_panel()),
    ])
}

fn aside() -> Node {
    m("aside")
        .class("home-aside")
        .attr("id", "home-aside")
        .attr("aria-label", "Details")
        .kids(vec![
            m("div").class("home-aside-toggle-row").child(aside_toggle()),
            m("div").class("home-aside-body").kids(vec![
                m("div").show("$store.home.browserOpen").child(browser_pane::panel(LIVE)),
                m("div").class("aside-forms").show("!$store.home.browserOpen").kids(vec![
                agent_editor::agent_editor(),
                agent_editor::settings_panel(),
                profile_panel::panel(),
                target_form::panel(),
                session_editor::panel(LIVE),
                crate::components::group_editor::panel(LIVE),
                pr_panel::panel(),
                repo_panel::panel(),
                run_panel::panel(),
                job_panel::panel(),
                share_dialog::panel(),
                ]),
            ]),
        ])
}

pub fn shell() -> Node {
    m("div")
        .attr("id", "home")
        .class("home")
        .init("$store.live.loadProjects(); $store.live.loadSessions(); $store.live.loadTargets(); $store.live.loadAutomations(); $store.vcs.loadRepos(); $store.vcs.loadPrs(); $store.jobs.loadJobs(); $store.jobs.loadRuns()")
        .bind(
            "class",
            &format!(
                "(($store.{STORE}.asideOpen ? '' : 'aside-collapsed') + ($store.{STORE}.railOpen ? '' : ' rail-collapsed') + ($store.{STORE}.dragSide ? ' home-resizing' : ''))"
            ),
        )
        .bind(
            "style",
            &format!(
                "('--rail-w:' + ($store.{STORE}.railWidth ? $store.{STORE}.railWidth + 'px' : 'initial') + '; --aside-w:' + ($store.{STORE}.asideWidth ? $store.{STORE}.asideWidth + 'px' : 'initial'))"
            ),
        )
        .kids(vec![rail(), gutter("left"), main_col(), gutter("right"), aside(), lightbox::lightbox()])
}

fn gutter(side: &str) -> Node {
    m("div")
        .class(&format!("home-gutter g-{side}"))
        .attr("title", "drag to resize")
        .on("pointerdown", &format!("$store.home.gutterDown('{side}', $event)"))
        .on("pointermove", "$store.home.gutterMove($event)")
        .on("pointerup", "$store.home.gutterUp($event)")
        .on("pointercancel", "$store.home.gutterUp($event)")
}

thread_local! {
    static DRAG: std::cell::RefCell<Drag> = std::cell::RefCell::new(Drag::default());
}

#[derive(Default)]
struct Drag {
    side: String,
    x: f64,
    start_w: f64,
}

fn gutter_down(_this: JsValue, args: js_sys::Array) -> JsValue {
    let side = js::arg_str(&args, 0);
    if side != "left" && side != "right" {
        return JsValue::UNDEFINED;
    }
    let ev = args.get(1);
    js::call(&ev, "preventDefault", &[]);
    let col = if side == "left" { ".home-rail" } else { ".home-aside" };
    let doc = js::get(&js::global(), "document");
    let el = js::call(&doc, "querySelector", &[JsValue::from_str(col)]);
    let rect = if js::is_missing(&el) { JsValue::UNDEFINED } else { js::call(&el, "getBoundingClientRect", &[]) };
    let start_w = js::get_num(&rect, "width");
    DRAG.with(|d| {
        *d.borrow_mut() = Drag { side: side.clone(), x: js::get_num(&ev, "clientX"), start_w }
    });
    let s = mjs::store_ref(STORE);
    js::set_str(&s, "dragSide", &side);
    let target = js::get(&ev, "currentTarget");
    if !js::is_missing(&target) {
        js::call(&target, "setPointerCapture", &[js::get(&ev, "pointerId")]);
    }
    mjs::redraw();
    JsValue::UNDEFINED
}

fn gutter_move(_this: JsValue, args: js_sys::Array) -> JsValue {
    let ev = args.get(0);
    let (side, x0, w0) = DRAG.with(|d| {
        let g = d.borrow();
        (g.side.clone(), g.x, g.start_w)
    });
    if side.is_empty() {
        return JsValue::UNDEFINED;
    }
    let delta = js::get_num(&ev, "clientX") - x0;
    let w = if side == "left" { w0 + delta } else { w0 - delta };
    let w = w.clamp(220.0, 1000.0);
    let s = mjs::store_ref(STORE);
    if side == "left" {
        js::set_num(&s, "railWidth", w);
    } else {
        js::set_num(&s, "asideWidth", w);
    }
    mjs::redraw();
    JsValue::UNDEFINED
}

fn gutter_up(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let side = DRAG.with(|d| {
        let mut g = d.borrow_mut();
        let s = std::mem::take(&mut g.side);
        s
    });
    if side.is_empty() {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    js::set_str(&s, "dragSide", "");
    let (key, v) = if side == "left" {
        ("chat.home.railWidth", js::get_num(&s, "railWidth"))
    } else {
        ("chat.home.asideWidth", js::get_num(&s, "asideWidth"))
    };
    if v > 0.0 {
        let storage = js::get(&js::global(), "localStorage");
        js::call(&storage, "setItem", &[JsValue::from_str(key), JsValue::from_str(&format!("{v:.0}"))]);
    }
    mjs::redraw();
    JsValue::UNDEFINED
}

fn saved_width(key: &str) -> f64 {
    let storage = js::get(&js::global(), "localStorage");
    let v = js::call(&storage, "getItem", &[JsValue::from_str(key)]);
    if js::is_missing(&v) || v.is_null() {
        return 0.0;
    }
    v.as_string()
        .and_then(|s| s.trim().parse::<f64>().ok())
        .filter(|n| *n > 0.0)
        .unwrap_or(0.0)
}

fn toggle_aside(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let open = js::get_bool(&s, "asideOpen");
    js::set_bool(&s, "asideOpen", !open);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn toggle_rail(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let open = js::get_bool(&s, "railOpen");
    js::set_bool(&s, "railOpen", !open);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn section_open(key: &str) -> bool {
    let storage = js::get(&js::global(), "localStorage");
    let value = js::call(&storage, "getItem", &[JsValue::from_str(&format!("chat.rail.{key}"))]);
    value.as_string().as_deref() != Some("false")
}

fn toggle_section(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let key = match js::arg_str(&args, 0).as_str() {
        "projects" => "projectsOpen",
        "agents" => "agentsOpen",
        "sandboxes" => "sandboxesOpen",
        "automations" => "automationsOpen",
        "repos" => "reposOpen",
        "worktrees" => "worktreesOpen",
        "pipelines" => "pipelinesOpen",
        "prs" => "prsOpen",
        "jobs" => "jobsOpen",
        _ => return JsValue::UNDEFINED,
    };
    let open = !js::get_bool(&s, key);
    js::set_bool(&s, key, open);
    let storage = js::get(&js::global(), "localStorage");
    js::call(&storage, "setItem", &[JsValue::from_str(&format!("chat.rail.{key}")), JsValue::from_str(if open { "true" } else { "false" })]);
    mjs::redraw();
    JsValue::UNDEFINED
}

fn toggle_browser(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let next = !js::get_bool(&s, "browserOpen");
    js::set_bool(&s, "browserOpen", next);
    js::set_bool(&s, "asideOpen", next);
    mjs::redraw();
    JsValue::UNDEFINED
}

pub fn register() {
    css::inject("home", CSS);
    let mut d = Def::new().named("store home");
    d.bool("asideOpen", false)
        .bool("railOpen", true)
        .bool("browserOpen", false)
        .num("railWidth", saved_width("chat.home.railWidth"))
        .num("asideWidth", saved_width("chat.home.asideWidth"))
        .str("dragSide", "")
        .bool("projectsOpen", section_open("projectsOpen"))
        .bool("agentsOpen", section_open("agentsOpen"))
        .bool("sandboxesOpen", section_open("sandboxesOpen"))
        .bool("automationsOpen", section_open("automationsOpen"))
        .bool("reposOpen", section_open("reposOpen"))
        .bool("worktreesOpen", section_open("worktreesOpen"))
        .bool("pipelinesOpen", section_open("pipelinesOpen"))
        .bool("prsOpen", section_open("prsOpen"))
        .bool("jobsOpen", section_open("jobsOpen"))
        .method("toggleAside", toggle_aside)
        .method("toggleRail", toggle_rail)
        .method("toggleBrowser", toggle_browser)
        .method("toggleSection", toggle_section)
        .method("gutterDown", gutter_down)
        .method("gutterMove", gutter_move)
        .method("gutterUp", gutter_up);
    mjs::store(STORE, &d.finish());
}

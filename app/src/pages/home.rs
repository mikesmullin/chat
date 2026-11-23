//! `/` — the app shell: three columns, assembled entirely from storybook
//! components.
//!
//! ```text
//!  +-------------+------------------------------+--------+
//!  | projects    |  tab bar (one per session)   |        |
//!  |             +------------------------------+ right  |
//!  | agents menu |  session panel               | aside  |
//!  |             |                              | (empty)|
//!  +-------------+------------------------------+--------+
//!    transparent        rounded --panel card      rounded
//!    (part of the                                 --panel
//!     black page)                                 card
//! ```
//!
//! The columns reuse the storybook builders — `project_tree`,
//! `agents_menu`, `tab::tab_bar`, the session cards and the composer — in
//! their **live** variants, which carry the same class names bound to
//! `$store.live` instead of demo consts. The design guide and the app stay
//! literally the same markup (STORYBOOKING.md §6). The palette
//! comes from the shared `:is(#sb, #home)` token ramp in `web/storybook.css`.
//!
//! The aside is drawn above as it looks once something has opened it; on a
//! fresh load it is collapsed and the centre column has its track.
//!
//! Either side column collapses through the `home` store rather than an
//! imperative class toggle, because the grid track has to shrink with it —
//! `x-show` alone would leave the column's width behind. Both toggles sit in
//! the tab strip flanking the tabs, so they stay visible when their column
//! is gone; a collapsed column is `display: none`, not a narrow strip, so
//! the transcript can take the whole width.

use wasm_bindgen::JsValue;

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::components::{
    agent_editor, agents_menu, automations_menu, identity_switcher, lightbox, mode_toggle,
    perf_badge, profile_panel, project_tree, sandboxes_menu, session_editor, session_panel,
    share_dialog, tab, target_form,
};
use crate::css;

/// The store every live column on this page reads.
const LIVE: &str = session_panel::LIVE_STORE;

const STORE: &str = "home";

pub const CSS: &str = r#"
/* ---- the three-column shell ------------------------------------
   The page is the black base; the rail sits on it with no surface of
   its own, and each content column is a rounded --panel card. */
#home {
  display: grid;
  grid-template-columns: 300px 1fr 320px;
  height: 100vh;
  height: 100dvh;
  min-height: 0;
  background: var(--bg);
  color: var(--text);
  font-family: var(--sans, ui-sans-serif, system-ui, sans-serif);
  transition: grid-template-columns .3s ease-in-out;
}
/* A collapsed column is gone, not narrow: `display: none` takes it out of
   flow (with explicit `grid-column` below, the rest stay in their tracks),
   and its track collapses to zero so no space is reserved. */
#home.aside-collapsed { grid-template-columns: 300px 1fr 0px; }
#home.aside-collapsed .home-aside { display: none; }
#home.rail-collapsed { grid-template-columns: 0px 1fr 320px; }
#home.rail-collapsed .home-rail { display: none; }
#home.rail-collapsed .home-col { margin-left: 10px; }
#home.rail-collapsed.aside-collapsed { grid-template-columns: 0px 1fr 0px; }
.home-rail { grid-column: 1; }

/* The sidebar's group rhythm: the visible gap between menu groups (and
   between the bottom block's rows) is ONE number — `--group-gap` — so
   every section and every future one spaces alike, collapsed or expanded.
   The container gap carries it; an earlier "tuck collapsed groups flush"
   hack (negative root margins) dragged the next group up over the head
   and made every gap a liar. 8px is the default for anything new added
   to this menu. */
.home-rail { --group-gap: 8px; }
.home-col { grid-column: 2; }
.home-aside { grid-column: 3; }

/* ---- left rail: part of the black page -------------------------- */
.home-rail {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--group-gap);
  overflow: hidden;
  padding: 10px 0 10px 4px;
  /* clears the rail's scrollbar from the content card beside it */
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

/* ---- centre column ----------------------------------------------
   A plain flex column holding the tab strip and, beneath it, the card.
   The column itself has no surface — only `.home-main` does. */
.home-col {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  margin-right: 10px;
}

/* the tab strip: on the black page, ABOVE the card it labels. A row now:
   a toggle for each side column flanking the tabs, so both stay reachable
   when their column is gone. The strip itself scrolls, never the buttons. */
.home-tabs {
  flex: 0 0 auto;
  display: flex; align-items: center; gap: 6px;
  padding: 10px 2px 8px;
}
.home-tabs .tab-bar {
  flex: 1 1 auto; min-width: 0;
  display: flex; overflow-x: auto; overflow-y: hidden;
}
/* One shared shape for both toggles; position says which column each owns.
   Flipped means "click to bring it back". */
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
/* the card starts under the tabs and runs to the bottom inset */
.home-main { flex: 1 1 auto; margin-bottom: 10px; }
.home-aside { margin: 10px 10px 10px 0; }

.home-main .mjs-chat { height: 100%; background: transparent; border-left: 0; }

/* the composer sits inset from the card edges, matching the 10px gutter
   the rail and the tab strip already use */
.home-main .composer-inner { padding: 0 10px 10px; }
.home-main .messages { padding-left: 10px; padding-right: 10px; }

/* ---- right aside ------------------------------------------------- */
.home-aside-body { flex: 1 1 auto; min-height: 0; overflow-y: auto; padding: 12px 12px 12px; }
.home-aside-toggle-row { display: none; }

/* The projects section opens the rail, with the same gutters as the
   sections below it so they read as one column. */
/* The head carries no side padding of its own any more — `rail_head` is the
   same brick in both sections — so the section owns the whole inset. 14px
   puts the title where it always was and the rows 6px in, the way the
   agents list sits. */
.home-projects { padding: 0 14px; }
/* The bottom block: two menu rows — Settings on its own line above the
   account chip. The rows touch (gap 0): the button and the chip carry
   their own padding, and stacked padding is how these two ended up apart. */
.home-settings {
  flex: 0 0 auto; display: flex; flex-direction: column;
  gap: 0; padding-top: 0;
}
.home-settings-row { display: flex; align-items: center; gap: 6px; }
.home-settings-spacer { margin-left: auto; }

/* Settings stays outside the scrolling content and keeps its intrinsic height. */
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

/* ---- responsive: the shell gives way in stages; content never scrolls
   sideways. Breakpoints are tuned for tablet widths (a Kindle Fire HD
   reports roughly 960-1280 CSS px in landscape, 600-800 in portrait).
   These rules only narrow tracks and, at the last step, lift the aside out
   of flow — no component's look is redefined here, so each component keeps
   owning its own rules (its narrow mode lives in its own file). */
@media (max-width: 1180px) {
  #home { grid-template-columns: 248px minmax(0, 1fr) 288px; }
  #home.aside-collapsed { grid-template-columns: 248px minmax(0, 1fr) 0px; }
}
@media (max-width: 1020px) {
  #home { grid-template-columns: 216px minmax(0, 1fr) 256px; }
  #home.aside-collapsed { grid-template-columns: 216px minmax(0, 1fr) 0px; }
}
/* Narrow: the rail becomes a 60px icon strip and the aside becomes an overlay
   driven by the same store flag as ever — anything that opens it (agent
   select, settings, a target form, the roster pencil) still opens it, and its
   own toggle still closes it. Collapsed means gone at every width: the aside
   is `display: none` when collapsed, so a reserved track holds nothing — it
   was 48px of dead gap beside the transcript, not a strip. */
@media (max-width: 900px) {
  /* `.aside-collapsed` is spelled out alongside the bare `#home`, and it is
     not redundant: the 1020px block above sets `#home.aside-collapsed`, which
     is id+class (0,2,0) and so outranks a bare `#home` (0,1,0) here no matter
     that this query is narrower and later. Without it a collapsed aside — the
     default — kept the 216px rail and a 48px dead track at phone width, which
     is precisely the layout this block exists to replace. Any rule added here
     that the wider tiers also set by class needs the same treatment. */
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
  /* Keep the center column's internal z-index layers beneath the overlay. */
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

/// left column: projects over the agents menu, on the black page.
fn rail() -> Node {
    m("aside")
        .class("home-rail")
        .attr("aria-label", "Projects, agents and automations")
        .kids(vec![
            // The filing cabinet, first: conversations grouped by where they
            // are filed rather than by when they were touched, so the top of
            // the rail always answers "what am I working in" at a glance.
            m("div").class("home-rail-content").kids(vec![
                // A section with nothing in it recedes (progressive
                // disclosure): a user who can see no projects, agents,
                // sandboxes or automations gets a clean rail, and each
                // section surfaces as the admin grants access. One exception,
                // so the rule cannot lock anyone out of creation: a section
                // whose `+` the user may use stays — it is the only way to
                // make the first one. The Sandboxes section never has a `+`
                // (agent-owned, PLAN55 D4), so its rule is simply "has a
                // row".
                m("div").class("home-projects")
                    .show(&format!(
                        "($store.ptree.rows({LIVE}.projects, {LIVE}.sessions).length > 0 \
                          || $store.ident.can('project.create'))"
                    ))
                    .child(project_tree::tree(
                        LIVE,
                        &format!("{LIVE}.previewSession({{}})"),
                        &format!("{LIVE}.createTarget('project')"),
                    )),
                m("div").class("home-agents")
                    .show("($store.agents.agents.length > 0 || $store.ident.can('agent.create'))")
                    .child(agents_menu::agents_menu_live()),
                // The sandboxes listing (PLAN55): each running container with
                // the (session, agent) it serves, grouped per image. Read-only
                // — sandboxes are agent-owned and launch themselves, so there
                // is nothing to create or name here (D3/D4/D10).
                m("div").class("home-sandboxes")
                    .show(&format!("{LIVE}.sandboxGroups.length > 0"))
                    .child(sandboxes_menu::sandboxes_menu(LIVE)),
                // Schedules below the people: each row is a named prompt on a
                // cron, and firing one opens a fresh session that runs it.
                m("div").class("home-automations")
                    .show("($store.live.automations.length > 0 || $store.ident.can('automation.create'))")
                    .child(automations_menu::automations_menu_live()),
            ]),
            // Pinned to the bottom of the rail: it is not part of the
            // conversation, and it is the one item whose position should
            // never move as the lists above it grow.
            m("div").class("home-settings").kids(vec![
                // Settings on its own line, above the account chip — two
                // menu rows in the same rhythm as the groups above.
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
                // Who you are (PLAN44 D4): the account chip owns the bottom
                // left — presence dot, name, and the menu to profile, switch
                // and log out. The mode toggle rides the same line, pushed
                // to the far right.
                m("div").class("home-settings-row").kids(vec![
                    identity_switcher::widget(),
                    m("span").class("home-settings-spacer"),
                    mode_toggle::button(),
                ]),
            ]),
        ])
}

/// The same inspector toggle is used in the tab strip and the narrow overlay.
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

/// centre column: the tab bar above the session panel, in one card.
fn main_col() -> Node {
    // The tab strip sits OUTSIDE the card: tabs label the panel, so the
    // rounded surface begins beneath them, the way a tabbed window works.
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
                // m-js's own draw counters, at the top of the page. It reads
                // the runtime, not the store, so it is the same widget the
                // guide mounts at /storybook/perf-badge — see that module for
                // why it writes its own text instead of binding it.
                perf_badge::badge().show("$store.mode.advanced"),
                aside_toggle(),
            ]),
        // With no tabs open there is nothing for the card to hold, so the
        // column falls back to the bare page: a slate rectangle wrapping an
        // empty panel reads as something that failed to load. The `+` lives
        // in the tab strip above, which stays.
        m("section")
            .class("home-main")
            .show("$store.live.tabs.length > 0")
            // The panel itself lives in `session_panel`, which the design
            // guide mounts too — see that module for why the tree is not
            // written out here.
            .child(session_panel::session_panel()),
    ])
}

/// right column: the inspector, with its collapse toggle.
fn aside() -> Node {
    m("aside")
        .class("home-aside")
        .attr("id", "home-aside")
        .attr("aria-label", "Details")
        .kids(vec![
            m("div").class("home-aside-toggle-row").child(aside_toggle()),
            // The inspector: contextual, so its contents are whatever is
            // selected elsewhere. Today that is an agent definition.
            m("div").class("home-aside-body").kids(vec![
                agent_editor::agent_editor(),
                agent_editor::settings_panel(),
                profile_panel::panel(),
                target_form::panel(),
                session_editor::panel(),
                share_dialog::panel(),
            ]),
        ])
}

/// the shell. The bound class drops the aside's grid track when collapsed.
pub fn shell() -> Node {
    m("div")
        .attr("id", "home")
        .class("home")
        // Pull the session, project and target lists once the shell exists.
        // `x-init` runs from the deferred lifecycle queue, so the store is
        // safe to write here. `loadSessions` backs the project tree's
        // "(ungrouped)" group and the tab strip; the flat history rail is gone.
        .init("$store.live.loadProjects(); $store.live.loadSessions(); $store.live.loadTargets(); $store.live.loadAutomations()")
        .bind(
            "class",
            &format!(
                "(($store.{STORE}.asideOpen ? '' : 'aside-collapsed') + ($store.{STORE}.railOpen ? '' : ' rail-collapsed'))"
            ),
        )
        .kids(vec![rail(), main_col(), aside(), lightbox::lightbox()])
}

/// `toggleAside`: flip the right column open/closed.
fn toggle_aside(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let open = js::get_bool(&s, "asideOpen");
    js::set_bool(&s, "asideOpen", !open);
    mjs::redraw();
    JsValue::UNDEFINED
}

/* `toggleRail`: flip the left column open/closed. Same shape as its
   right-hand twin, so the two buttons stay one idea. */
fn toggle_rail(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let open = js::get_bool(&s, "railOpen");
    js::set_bool(&s, "railOpen", !open);
    mjs::redraw();
    JsValue::UNDEFINED
}

/// Section visibility is a per-browser preference, independent of sessions.
fn section_open(key: &str) -> bool {
    let storage = js::get(&js::global(), "localStorage");
    let value = js::call(&storage, "getItem", &[JsValue::from_str(&format!("chat.rail.{key}"))]);
    value.as_string().as_deref() != Some("false")
}

/// `toggleSection(which)` — collapse/expand one left-rail section.
///
/// `which` is `"projects"`, `"agents"`, `"sandboxes"` or `"automations"`;
/// anything else is ignored.
/// Shell chrome, like the rail toggles above — the flag lives here rather
/// than in the data stores because collapsing a section hides a view, it
/// does not change what the view holds.
fn toggle_section(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let key = match js::arg_str(&args, 0).as_str() {
        "projects" => "projectsOpen",
        "agents" => "agentsOpen",
        "sandboxes" => "sandboxesOpen",
        "automations" => "automationsOpen",
        _ => return JsValue::UNDEFINED,
    };
    let open = !js::get_bool(&s, key);
    js::set_bool(&s, key, open);
    let storage = js::get(&js::global(), "localStorage");
    js::call(&storage, "setItem", &[JsValue::from_str(&format!("chat.rail.{key}")), JsValue::from_str(if open { "true" } else { "false" })]);
    mjs::redraw();
    JsValue::UNDEFINED
}

pub fn register() {
    css::inject("home", CSS);
    let mut d = Def::new().named("store home");
    // The inspector starts **closed**. It is contextual — it holds whatever
    // was last selected — so with nothing selected yet it has nothing to
    // show, and 320px of empty card beside the transcript reads as a panel
    // that failed to load. Everything that puts something in it opens it on
    // the way (see `agents::select`, `agents::new_agent`,
    // `settings::open_panel`, `live::open_session_editor`,
    // `target_form::open_with`), so the column appears exactly when it has an
    // answer. Its toggle in the tab strip stays visible either way.
    d.bool("asideOpen", false)
        .bool("railOpen", true)
        .bool("projectsOpen", section_open("projectsOpen"))
        .bool("agentsOpen", section_open("agentsOpen"))
        .bool("sandboxesOpen", section_open("sandboxesOpen"))
        .bool("automationsOpen", section_open("automationsOpen"))
        .method("toggleAside", toggle_aside)
        .method("toggleRail", toggle_rail)
        .method("toggleSection", toggle_section);
    mjs::store(STORE, &d.finish());
}

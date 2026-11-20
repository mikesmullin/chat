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
//! The right aside collapses through the `home` store rather than an
//! imperative class toggle, because the grid track has to shrink with it —
//! `x-show` alone would leave the column's width behind.

use wasm_bindgen::JsValue;

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::components::{
    agent_editor, agents_menu, project_tree, session_panel, tab,
    target_form,
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
  min-height: 0;
  background: var(--bg);
  color: var(--text);
  font-family: var(--sans, ui-sans-serif, system-ui, sans-serif);
  transition: grid-template-columns .3s ease-in-out;
}
#home.aside-collapsed { grid-template-columns: 300px 1fr 48px; }

/* ---- left rail: part of the black page -------------------------- */
.home-rail {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 18px;
  overflow-y: auto;
  padding: 10px 0 10px 4px;
  /* clears the rail's scrollbar from the content card beside it */
  margin-right: 10px;
}
.home-agents { padding: 0 14px 10px; }

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

/* the tab strip: on the black page, ABOVE the card it labels */
.home-tabs {
  flex: 0 0 auto;
  padding: 10px 2px 8px;
  overflow-x: auto;
  overflow-y: hidden;
}

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
.home-aside-head {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 8px;
}
.aside-toggle {
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
.aside-toggle:hover { color: var(--text); background: var(--panel-2); border-color: var(--border); }
.aside-toggle:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
#home.aside-collapsed .aside-toggle .ph { transform: scaleX(-1); }

.home-aside-body { flex: 1 1 auto; min-height: 0; overflow-y: auto; padding: 0 12px 12px; }

/* The projects section opens the rail, with the same gutters as the
   sections below it so they read as one column. */
.home-projects { padding: 14px 10px 4px; }

/* The rail is a column so Settings can be pinned under everything else. */
.home-rail { display: flex; flex-direction: column; }
.home-rail .home-agents { flex: 0 0 auto; }
.home-settings { margin-top: auto; padding-top: 12px; }
.rail-settings {
  display: inline-flex; align-items: center; gap: 8px;
  width: 100%; border: 0; border-radius: 8px;
  background: transparent; color: var(--gray-dim);
  padding: 7px 8px; cursor: pointer; font: inherit; font-size: 13px;
  text-align: left;
}
.rail-settings:hover { background: var(--panel-2); color: var(--fg); }
.rail-settings i { font-size: 15px; line-height: 18px; }
#home.aside-collapsed .home-aside-body { display: none; }
"#;

/// left column: projects over the agents menu, on the black page.
fn rail() -> Node {
    m("aside")
        .class("home-rail")
        .attr("aria-label", "Projects and agents")
        .kids(vec![
            // The filing cabinet, first: conversations grouped by where they
            // are filed rather than by when they were touched, so the top of
            // the rail always answers "what am I working in" at a glance.
            m("div").class("home-projects").child(project_tree::tree(
                LIVE,
                &format!("{LIVE}.openSession({{}})"),
                &format!("{LIVE}.createTarget('project')"),
            )),
            m("div").class("home-agents").child(agents_menu::agents_menu_live()),
            // Pinned to the bottom of the rail: it is not part of the
            // conversation, and it is the one item whose position should
            // never move as the lists above it grow.
            m("div").class("home-settings").child(
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
        ])
}

/// centre column: the tab bar above the session panel, in one card.
fn main_col() -> Node {
    // The tab strip sits OUTSIDE the card: tabs label the panel, so the
    // rounded surface begins beneath them, the way a tabbed window works.
    m("div").class("home-col").kids(vec![
        m("div").class("home-tabs").child(tab::tab_bar(LIVE)),
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
        .attr("aria-label", "Details")
        .kids(vec![
            m("div").class("home-aside-head").child(
                m("button")
                    .class("aside-toggle")
                    .attr("type", "button")
                    .attr("aria-label", "Toggle right sidebar")
                    .bind(
                        "aria-expanded",
                        &format!("($store.{STORE}.asideOpen ? 'true' : 'false')"),
                    )
                    .on("click", &format!("$store.{STORE}.toggleAside()"))
                    .child(Node::icon("sidebar-simple")),
            ),
            // The inspector: contextual, so its contents are whatever is
            // selected elsewhere. Today that is an agent definition.
            m("div").class("home-aside-body").kids(vec![
                agent_editor::agent_editor(),
                agent_editor::settings_panel(),
                target_form::panel(),
                agent_editor::inspector_empty(),
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
        // "Unfiled" group and the tab strip; the flat history rail is gone.
        .init("$store.live.loadProjects(); $store.live.loadSessions(); $store.live.loadTargets()")
        .bind(
            "class",
            &format!("($store.{STORE}.asideOpen ? '' : 'aside-collapsed')"),
        )
        .kids(vec![rail(), main_col(), aside()])
}

/// `toggleAside`: flip the right column open/closed.
fn toggle_aside(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let open = js::get_bool(&s, "asideOpen");
    js::set_bool(&s, "asideOpen", !open);
    mjs::redraw();
    JsValue::UNDEFINED
}

pub fn register() {
    css::inject("home", CSS);
    let mut d = Def::new().named("store home");
    d.bool("asideOpen", true).method("toggleAside", toggle_aside);
    mjs::store(STORE, &d.finish());
}

//! `/` — the app shell: three columns, assembled entirely from storybook
//! components.
//!
//! ```text
//!  +-------------+------------------------------+--------+
//!  | session     |  tab bar (one per session)   |        |
//!  | history     +------------------------------+ right  |
//!  |             |                              | aside  |
//!  | agents menu |  session panel               | (empty)|
//!  +-------------+------------------------------+--------+
//!    transparent        rounded --panel card      rounded
//!    (part of the                                 --panel
//!     black page)                                 card
//! ```
//!
//! The columns reuse the storybook builders — `session_history`,
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

use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::components::{agents_menu, composer, session, session_history, tab};
use crate::css;

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
/* the history panel is a section of the rail, not a floating card: it
   gives up the --panel fill and the divider it carries in the chat app,
   so it reads as part of the black page like the agents menu below it. */
.home-rail .mjs-chat { background: transparent; height: auto; }
.home-rail .history {
  background: transparent;
  border-right: 0;
  width: auto;
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
#home.aside-collapsed .home-aside-body { display: none; }
"#;

/// left column: session history over the agents menu, on the black page.
fn rail() -> Node {
    m("aside")
        .class("home-rail")
        .attr("aria-label", "Sessions and agents")
        .kids(vec![
            session_history::session_history_live(),
            m("div").class("home-agents").child(agents_menu::agents_menu()),
        ])
}

/// centre column: the tab bar above the session panel, in one card.
fn main_col() -> Node {
    // The tab strip sits OUTSIDE the card: tabs label the panel, so the
    // rounded surface begins beneath them, the way a tabbed window works.
    m("div").class("home-col").kids(vec![
        m("div").class("home-tabs").child(tab::tab_bar_live()),
        // With no tabs open there is nothing for the card to hold, so the
        // column falls back to the bare page: a slate rectangle wrapping an
        // empty panel reads as something that failed to load. The `+` lives
        // in the tab strip above, which stays.
        m("section")
            .class("home-main")
            .show("$store.live.tabs.length > 0")
            .child(
            m("div")
                .class("mjs-chat mode-sidebar")
                .attr("aria-label", "Session panel")
                .child(
                    m("section")
                        .class("main")
                        .kids(vec![session::messages_live(), composer::panel_live()]),
                ),
        ),
    ])
}

/// right column: empty for now, with its collapse toggle.
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
            m("div").class("home-aside-body"),
        ])
}

/// the shell. The bound class drops the aside's grid track when collapsed.
pub fn shell() -> Node {
    m("div")
        .attr("id", "home")
        .class("home")
        // Pull the session list once the shell exists. `x-init` runs from the
        // deferred lifecycle queue, so the store is safe to write here.
        .init("$store.live.loadSessions()")
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

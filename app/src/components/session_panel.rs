//! `session panel` — the middle column whole: the session transcript
//! with the composer under it, as the live app stacks them in `section.main`.
//!
//! Assembled from the parts, not rewritten: `session::messages` + the
//! composer's `panel`. Depth 8: session + composer + task list + the five
//! cards.

use rustex1_kit::h::{m, Node};

use crate::components::{composer, session};
use crate::css;

pub const CSS: &str = r#"
/* demo framing: full column; the transcript scrolls, the composer pins */
.session-panel-demo { width: min(640px, 100%); height: 640px; }
.session-panel-demo .mjs-chat.mode-sidebar { border-left: 0; }
.session-panel-demo .mjs-chat { height: 100%; }
"#;

/// the column: transcript above, composer below.
pub fn session_panel() -> Node {
    m("div")
        .class("mjs-chat mode-sidebar")
        .attr("aria-label", "Session panel")
        .child(
            m("section")
                .class("main")
                .kids(vec![session::messages(), composer::panel()]),
        )
}

/// standalone demo.
pub fn demo() -> Node {
    m("div").class("session-panel-demo").child(session_panel())
}

pub fn register() {
    css::inject("session-panel", CSS);
}

//! `uiCounter` — a small mono numeral, placed inline by its host.
//!
//! The storybook's "Counter" component: same size and line-height as the group
//! name, muted, no self-alignment — placed by its host.
//!
//! Owns `.counter-demo` (the showcase layout); the shared `.counter` base lives
//! in the global layer (it's also used by the rail and item heads).

use rustex1_kit::h::m;
use rustex1_kit::mjs::{self, Def};
use crate::css;

const NAME: &str = "uiCounter";

const CSS: &str = r#"
/* counter demo: single-space (6px at 12px mono) between instances */
.counter-demo { display: flex; gap: 6px; align-items: baseline; }
"#;

fn def() -> Def {
    let mut d = Def::new().named(NAME);
    d.prop_num("value", 0.0)
        .tree(m("span").class("counter").text("value"));
    d
}

pub fn register() {
    css::inject(NAME, CSS);
    mjs::component(NAME, &def().finish());
}

//! `demo` — the shared demo store for the standalone input routes.
//!
//! The kit widgets only persist through a bound `model` object (an unbound
//! host drops every keystroke), so each input demo binds to `$store.demo`
//! under its own key. One store keeps the demos independent of the
//! automation form's `automationDraft` — typing in a demo never touches the
//! form, and vice versa.

use rustex1_kit::mjs::Def;
use rustex1_kit::mjs;

pub const MODEL: &str = "$store.demo";

pub fn register() {
    let mut d = Def::new().named("store demo");
    d.str("text1", "")
        .str("text2", "")
        .str("toggle", "false")
        .str("range", "")
        .str("radio", "")
        .str("area", "")
        .str("tags", "")
        .str("field", "");
    mjs::store("demo", &d.finish());
}



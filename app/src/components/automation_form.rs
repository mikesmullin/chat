//! `automation form` — the "New automation" form: nine `field` rows
//! (label + bare input + hint) plus a submit row hosting the kit `uiButton`
//! and the live `validation info` line, all bound to the `automationDraft`
//! store.
//!
//! Row → control map: name / schedule / repo → `text input`; worktree →
//! `toggle button`; sandbox / model → `radio input` (sandbox options reuse
//! three sandbox names from the Sandboxes Menu, so the two agree); prompt →
//! `text area`; timeout → `range input`; tool allowlist → `tag input` (its
//! comma-joined value stays a plain string).
//!
//! Depth 10: ten distinct storybook components (field + its eight parts +
//! validation info). Owns only the form layout CSS; the kit field-chrome
//! retint lives in the global layer (`web/storybook.css`).

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::{js, mjs};
use rustex1_kit::mjs::Def;
use wasm_bindgen::JsValue;

use crate::components::{
    field, radio_input, range_input, tag_input, text_area, text_input, toggle_button,
    validation_info,
};
use crate::css;

const STORE: &str = "automationDraft";
const MODEL: &str = "$store.automationDraft";
const MESSAGE: &str = "$store.automationDraft.message";
const MESSAGE_TONE: &str = "$store.automationDraft.messageTone";
const CREATE: &str = "$store.automationDraft.create()";

const SANDBOXES: &[&str] = &["checkout-flow", "auth-migration", "search-index"];
const MODELS: &[&str] = &["opus", "sonnet", "haiku"];

pub const CSS: &str = r#"
.automation-form {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  max-width: 560px;
  width: 100%;
}
.automation-title {
  grid-column: 1 / -1;
  font-size: 15px;
  font-weight: 700;
  color: var(--text);
  margin: 0 0 2px;
}
.automation-actions {
  grid-column: 1 / -1;
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 2px;
}
"#;

pub fn automation_form() -> Node {
    m("div").class("automation-form").kids(vec![
        m("h2").class("automation-title").child(lit("New automation")),
        field::field(
            "Name",
            true,
            "",
            text_input::input("name", MODEL, "my-automation"),
            false,
        ),
        field::field(
            "Schedule",
            true,
            "crontab",
            text_input::input("schedule", MODEL, "0 9 * * *"),
            false,
        ),
        field::field(
            "Repo",
            true,
            "absolute path",
            text_input::input("repo", MODEL, "/abs/path/to/repo"),
            true,
        ),
        field::field(
            "Worktree",
            false,
            "new worktree for the repo",
            toggle_button::input("worktree", MODEL),
            false,
        ),
        field::field(
            "Timeout",
            true,
            "minutes",
            range_input::input("timeout", MODEL),
            false,
        ),
        field::field(
            "Sandbox",
            true,
            "",
            radio_input::input("sandbox", MODEL, SANDBOXES),
            false,
        ),
        field::field(
            "Model",
            true,
            "",
            radio_input::input("model", MODEL, MODELS),
            false,
        ),
        field::field(
            "Prompt",
            true,
            "",
            text_area::input("prompt", MODEL, "What should be done on schedule…", 4.0),
            true,
        ),
        field::field(
            "Tool allowlist",
            true,
            "",
            tag_input::input("tools", MODEL, "read, bash…"),
            true,
        ),
        m("div").class("automation-actions").kids(vec![
            // the click lives on this wrapper (route-tree element, like a
            // tab-bar row) — the button inside is the kit widget, whose
            // template m-js owns, so a handler there would be inert.
            m("span")
                .class("automation-submit")
                .on("click", CREATE)
                .child(Node::host(
                    "uiButton",
                    &[("label", "Create automation"), ("variant", "primary")],
                )),
            validation_info::live(MESSAGE_TONE, MESSAGE),
        ]),
    ])
}

/// `create`: require a name, else confirm the queue. Writes `message` +
/// `messageTone`, which the validation line shows.
fn create(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let name = js::get_str(&s, "name").trim().to_string();
    if name.is_empty() {
        crate::components::validation_info::note("automationDraft", "message", "Name is required");
        js::set_str(&s, "messageTone", "error");
    } else {
        let schedule = js::get_str(&s, "schedule").trim().to_string();
        crate::components::validation_info::note("automationDraft", "message", &format!("Queued {name} · {schedule}"));
        js::set_str(&s, "messageTone", "ok");
    }
    mjs::redraw();
    JsValue::UNDEFINED
}

fn store_register() {
    let mut d = Def::new().named("store automationDraft");
    d.str("name", "nightly-docs")
        .str("schedule", "0 9 * * *")
        .str("repo", "/workspace/psicom/chat")
        .str("worktree", "true")
        .str("sandbox", "checkout-flow")
        .str("model", "sonnet")
        .str("prompt", "Summarise yesterday's commits.")
        .str("timeout", "30")
        .str("tools", "read,bash")
        .str("message", "")
        .str("messageTone", "")
        .method("create", create);
    mjs::store(STORE, &d.finish());
}

/// inject the layout CSS and register the `automationDraft` store.
pub fn register() {
    css::inject("automation-form", CSS);
    store_register();
}

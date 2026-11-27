use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::{js, mjs};
use wasm_bindgen::JsValue;

use crate::components::dropdown;
use crate::components::samples;
use crate::css;

pub const CSS: &str = r#"
.mjs-chat .composer {
  border-top: 1px solid var(--mjs-line);
  background: var(--mjs-panel);
  padding: 12px 16px 16px;
  flex-shrink: 0;
}
.mjs-chat.mode-sidebar .composer { padding: 8px; }
.mjs-chat .composer-inner {
  max-width: 900px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.mjs-chat.mode-sidebar .composer-inner { max-width: none; }
.mjs-chat .draft-ta {
  width: 100%;
  min-height: 72px;
  max-height: 200px;
  resize: vertical;
  border-radius: 12px;
  border: 1px solid var(--mjs-line);
  background: var(--mjs-bg);
  color: var(--mjs-ink);
  padding: 12px 14px;
  font: 14px/1.4 var(--mjs-sans);
  outline: none;
}
.mjs-chat.mode-sidebar .draft-ta { min-height: 56px; font-size: 13px; padding: 8px 10px; border-radius: 8px; }
.mjs-chat .draft-ta:focus { border-color: var(--mjs-accent); }
.mjs-chat .composer-row {
  display: flex;
  justify-content: flex-start;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}
.mjs-chat .btn-primary,
.mjs-chat .btn-stop {
  appearance: none;
  border-radius: 10px;
  padding: 8px 16px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.mjs-chat .btn-primary { background: var(--mjs-accent); color: #fff; border: 0; margin-left: auto; }
.mjs-chat .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
.mjs-chat .btn-stop { background: #7f1d1d; color: #fecaca; border: 1px solid #991b1b; }
.mjs-chat .composer-attach {
  appearance: none; border: 0; background: transparent; cursor: pointer;
  width: 28px; height: 28px; border-radius: 8px; padding: 0;
  display: inline-flex; align-items: center; justify-content: center;
  color: var(--mjs-dim, var(--muted));
}
.mjs-chat .composer-attach:hover { background: var(--panel-2); color: var(--text); }
.mjs-chat .composer-attach-chip {
  display: flex; align-items: center; gap: 8px; padding: 6px 8px;
  margin-bottom: 6px; width: fit-content;
  border: 1px solid var(--line); border-radius: 10px; background: var(--panel-2);
}
.mjs-chat .composer-attach-chip img {
  width: 56px; height: 56px; object-fit: cover;
  border-radius: 8px; display: block; cursor: zoom-in;
}
.mjs-chat .composer-attach-chip button {
  appearance: none; border: 0; background: transparent; cursor: pointer;
  color: var(--mjs-dim, var(--muted)); width: 22px; height: 22px;
  border-radius: 6px; display: inline-flex; align-items: center;
  justify-content: center;
}
.mjs-chat .composer-attach-chip button:hover { background: var(--panel); color: var(--text); }
.mjs-chat.mode-sidebar .btn-primary,
.mjs-chat.mode-sidebar .btn-stop {
  height: 30px;
  padding: 0 12px;
  border-radius: 7px;
  font-size: 12px;
  background: var(--mjs-panel-2);
}
.mjs-chat.mode-sidebar .btn-primary { border: 1px solid var(--mjs-accent); color: var(--mjs-accent-2); }
.mjs-chat.mode-sidebar .btn-stop { border: 1px solid var(--mjs-err); color: var(--mjs-err); }

.composer-demo { width: min(560px, 100%); }
.composer-demo .mjs-chat,
.composer-demo .mjs-chat.mode-sidebar {
  background: transparent;
  border-left: 0;
}

.mjs-chat .composer.composer-flat,
.mjs-chat.mode-sidebar .composer.composer-flat {
  background: transparent;
  border-top: 0;
  padding: 0;
}

.mjs-chat .composer-box {
  background: var(--mjs-panel-2);
  border: 1px solid var(--mjs-line);
  border-radius: 14px;
  padding: 2px 6px 6px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.mjs-chat .composer-box .draft-ta,
.mjs-chat.mode-sidebar .composer-box .draft-ta {
  background: transparent;
  border: 0;
  border-radius: 0;
  padding: 8px 6px 2px;
  min-height: 46px;
  resize: none;
}
.mjs-chat .composer-box .draft-ta:focus { border: 0; outline: none; box-shadow: none; }
:is(#sb, #home) .composer-box .draft-ta:focus {
  border: 0;
  outline: none;
  box-shadow: none;
}

.mjs-chat .composer-box .composer-row {
  gap: 4px;
  flex-wrap: nowrap;
  padding: 0 2px;
}


.mjs-chat .composer-box .dropdown.dd-model { margin-left: auto; }
.mjs-chat .composer-box .dropdown-value { max-width: 150px; }

.mjs-chat .allowlist-panel { display: flex; flex-direction: column; gap: 4px; }
.mjs-chat .allowlist-label {
  font-size: 11px;
  color: var(--mjs-dim);
  display: flex;
  align-items: center;
  gap: 6px;
}
.mjs-chat .allowlist-source { font-size: 10px; opacity: 0.75; }
.mjs-chat .allowlist-revert {
  margin-left: auto;
  background: transparent;
  border: 0;
  color: var(--mjs-accent);
  font-size: 10px;
  cursor: pointer;
  padding: 0 2px;
}
.mjs-chat .allowlist-revert:hover { text-decoration: underline; }
.mjs-chat .allowlist-ta {
  width: 100%;
  min-height: 72px;
  max-height: 160px;
  resize: vertical;
  border-radius: 10px;
  border: 1px solid var(--mjs-line);
  background: var(--mjs-bg);
  color: var(--mjs-dim);
  padding: 8px 10px;
  font-family: var(--mjs-mono);
  font-size: 12px;
  outline: none;
}
.mjs-chat .allowlist-ta:focus { border-color: var(--mjs-accent); color: var(--mjs-ink); }
.mjs-chat .composer-metrics {
  display: flex;
  justify-content: flex-start;
  align-items: center;
  flex-wrap: wrap;
  gap: 0;
  padding: 2px 4px 4px 2px;
  font-size: 11px;
  color: var(--mjs-dim);
  user-select: none;
}
.mjs-chat .composer-metrics .metric-chip:last-child { margin-right: 0; }

.mjs-chat .composer-box .dropdown.dd-policy { margin-right: 2px; }

.mjs-chat .composer-box .btn-primary,
.mjs-chat .composer-box .btn-stop,
.mjs-chat.mode-sidebar .composer-box .btn-primary,
.mjs-chat.mode-sidebar .composer-box .btn-stop {
  margin-left: 0;
  flex: 0 0 auto;
  width: 30px;
  height: 30px;
  padding: 0;
  border-radius: 999px;
  font-size: 15px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0;
}
.mjs-chat .composer-box .btn-primary,
.mjs-chat.mode-sidebar .composer-box .btn-primary {
  background: var(--mjs-accent);
  border: 0;
  color: #fff;
}
.mjs-chat .composer-box .btn-primary:disabled { opacity: 0.4; }
.mjs-chat .composer-box .btn-stop,
.mjs-chat.mode-sidebar .composer-box .btn-stop {
  background: color-mix(in srgb, var(--mjs-err) 18%, transparent);
  border: 1px solid var(--mjs-err);
  color: var(--mjs-err);
  cursor: pointer;
}
.mjs-chat .composer-box .btn-stop:hover {
  background: color-mix(in srgb, var(--mjs-err) 30%, transparent);
}
.mjs-chat .composer-box .btn-stop:disabled {
  opacity: 0.5;
  cursor: default;
}
"#;

pub fn composer(st: &str) -> Node {
    m("div").class("mjs-chat mode-sidebar").child(panel(st))
}

pub fn demo() -> Node {
    m("div").class("composer-demo").child(composer(samples::EXPR))
}

pub fn register() {
    css::inject("composer", CSS);
}

pub fn reset_history(st: &JsValue) {
    js::set_num(st, "historyPosition", -1.0);
    js::set_str(st, "historyDraft", "");
}

pub fn draft_input(st: &JsValue, args: js_sys::Array) -> JsValue {
    let event = js::arg(&args, 0);
    js::set_str(st, "draft", &js::get_str(&js::get(&event, "target"), "value"));
    mjs::redraw();
    JsValue::UNDEFINED
}

pub fn history_move(st: &JsValue, args: js_sys::Array) -> JsValue {
    if js::get_str(st, "activeId").is_empty() { return JsValue::UNDEFINED; }
    let items: Vec<String> = js_sys::Array::from(&js::get(st, "messages"))
        .iter()
        .filter(|row| js::get_str(row, "kind") == "user")
        .map(|row| js::get_str(&row, "text"))
        .filter(|text| !text.trim().is_empty())
        .collect();
    let count = items.len() as i64;
    if count == 0 { return JsValue::UNDEFINED; }
    let stored = js::get(st, "historyPosition").as_f64().unwrap_or(-1.0) as i64;
    let position = if stored < 0 { count } else { stored.min(count) };
    let direction = js::arg(&args, 0).as_f64().unwrap_or(0.0) as i64;
    if stored < 0 && direction > 0 { return JsValue::UNDEFINED; }
    let next = (position + direction).clamp(0, count);
    if position == count && next < count {
        js::set_str(st, "historyDraft", &js::get_str(st, "draft"));
    }
    let text = if next == count { js::get_str(st, "historyDraft") }
        else { items[next as usize].clone() };
    js::set_num(st, "historyPosition", if next == count { -1.0 } else { next as f64 });
    js::set_str(st, "draft", &text);
    let event = js::arg(&args, 1);
    js::call(&event, "preventDefault", &[]);
    mjs::redraw();
    let target = js::get(&event, "target");
    let end = js_sys::JsString::from(text).length() as f64;
    js::call(&target, "setSelectionRange", &[JsValue::from_f64(end), JsValue::from_f64(end)]);
    JsValue::UNDEFINED
}

pub fn panel(st: &str) -> Node {
    m("footer").class("composer composer-flat").child(
        m("div").class("composer-inner").kids(vec![
            m("div")
                .class("composer-metrics")
                .show(&format!("$store.mode.advanced && {st}.lastMetrics"))
                .inner_html(&format!("{st}.lastMetrics")),
            allowlist_panel(st),
            m("div").class("composer-box").kids(vec![
                m("textarea")
                    .class("draft-ta")
                    .model(&format!("{st}.draft"))
                    .on("input", &format!("{st}.draftInput($event)"))
                    .attr("placeholder", "Message… (Enter send, Shift+Enter newline)")
                    .on("click", &format!("{st}.markSessionRead({st}.activeId)"))
                    .on("keydown", &format!("$event.key !== 'Escape' && {st}.treeResetEscape()"))
                    .on("keydown.enter", &format!("{st}.treeOpen ? ($event.preventDefault(), {st}.treeChooseSelected()) : ($event.shiftKey || ($event.preventDefault(), {st}.send()))"))
                    .on("keydown.escape", &format!("{st}.treeEscape($event)"))
                    .on("keydown.up", &format!("{st}.treeOpen ? ($event.preventDefault(), {st}.treeMove(-1)) : (!$event.shiftKey && !$event.ctrlKey && !$event.altKey && !$event.metaKey && $el.selectionStart === $el.selectionEnd && !$el.value.slice(0, $el.selectionStart).includes('\\n') && {st}.historyMove(-1, $event))"))
                    .on("keydown.down", &format!("{st}.treeOpen ? ($event.preventDefault(), {st}.treeMove(1)) : (!$event.shiftKey && !$event.ctrlKey && !$event.altKey && !$event.metaKey && $el.selectionStart === $el.selectionEnd && !$el.value.slice($el.selectionEnd).includes('\\n') && {st}.historyMove(1, $event))")),
                m("div").class("composer-attach-chip").show(&format!("{st}.image")).kids(vec![
                    m("img")
                        .bind("src", &format!("{st}.image"))
                        .attr("alt", "attached")
                        .on("click", &format!("$store.lightbox.open({st}.image)")),
                    m("button")
                        .attr("type", "button")
                        .attr("title", "Remove image")
                        .on("click", &format!("{st}.dropImage()"))
                        .child(Node::icon("x")),
                ]),
                m("div").class("composer-row").kids(vec![
                    m("button")
                        .class("composer-attach")
                        .attr("type", "button")
                        .attr("title", "Attach an image")
                        .attr("aria-label", "Attach an image")
                        .on("click", &format!("{st}.pickImage()"))
                        .child(Node::icon("image-square")),
                    m("input")
                        .class("composer-image-input")
                        .attr("type", "file")
                        .attr("accept", "image/png,image/jpeg,image/gif,image/webp")
                        .attr("style", "display:none")
                        .on("change", &format!("{st}.imageChosen($event)")),
                    dropdown::policy("dd-policy", st),
                    dropdown::model("dd-model", st),
                    dropdown::level("", st),
                    m("button")
                        .class("btn-stop")
                        .attr("type", "button")
                        .show(&format!("{st}.waiting"))
                        .on("click", &format!("{st}.stop()"))
                        .bind("disabled", &format!("{st}.stopping"))
                        .bind(
                            "title",
                            &format!("({st}.stopping ? 'Stopping…' : 'Stop')"),
                        )
                        .child(Node::icon("stop")),
                    m("button")
                        .class("btn-primary")
                        .attr("type", "button")
                        .show(&format!("!{st}.waiting"))
                        .bind("disabled", &format!("{st}.treeOpen || !{st}.draft.trim()"))
                        .on("click", &format!("{st}.send()"))
                        .attr("title", "Send")
                        .child(Node::icon("arrow-up")),
                ]),
            ]),
        ]),
    )
}

fn allowlist_panel(st: &str) -> Node {
    m("div")
        .class("allowlist-panel")
        .show(&format!("{st}.allowlistOpen && !{st}.unrestricted"))
        .kids(vec![
            m("label").class("allowlist-label").kids(vec![
                lit("session allowed commands (one rule per line)"),
                m("span")
                    .class("allowlist-source")
                    .text(&format!(
                        "{st}.allowlistOverridden ? '· session rules' : '· no session rules'"
                    )),
                m("button")
                    .class("allowlist-revert")
                    .attr("type", "button")
                    .attr("title", "Clear session rules")
                    .show(&format!("{st}.allowlistOverridden"))
                    .on("click", &format!("{st}.revertAllowlist()"))
                    .child(lit("clear")),
            ]),
            m("textarea")
                .class("allowlist-ta")
                .attr("spellcheck", "false")
                .attr(
                    "placeholder",
                    "file_io__read_file\nterm_local__bash:ls.*\n\n(empty = deny everything)",
                )
                .model(&format!("{st}.allowlist"))
                .on("blur", &format!("{st}.saveAllowlist()")),
        ])
}

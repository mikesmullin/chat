//! `composer` — the prompt bar under the transcript: the `task list`
//! panel, then one rounded dark-gray envelope holding the draft box and
//! its control row (allowlist toggle, the model and reasoning-level
//! dropdowns, context pie, send/stop).
//!
//! Verbatim app classes. The draft is two-way-bound to the shared `demo`
//! store so typing works; send/stop flip the waiting flag through it; the
//! two `dropdown` units carry their own `dropdown` store. The
//! allowlist toggle is resting chrome here; its panel is dynamic app state
//! and lives outside this showcase.
//!
//! The envelope (`.composer-box`) is the storybook's own shape: the task
//! list sits *outside* it, above. Every override below is scoped under
//! `.composer-box`, so the live app's composer — which shares these class
//! names but has no such wrapper — keeps the old flat row.
//!
//! Depth 2: the task list, the dropdown.

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::{js, mjs};
use wasm_bindgen::JsValue;

use crate::components::dropdown;
use crate::components::picker;
use crate::components::samples;
use crate::components::session_state;
use crate::css;

pub const CSS: &str = r#"
/* ---- the composer envelope, the draft box, the context pie and the send/stop pair ----
   Out of `web/styles.css`, where these were the only rules dressing
   this component — a look kept in a global sheet outlives the markup
   it was written for. Copied unchanged, still scoped under
   `.mjs-chat`, so neither specificity nor the cascade moved with
   them. */
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
.mjs-chat .ctx-pie {
  position: relative;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
}
.mjs-chat .ctx-pie-track { color: var(--mjs-panel-2); }
.mjs-chat .ctx-pie-fill { color: var(--mjs-accent); }
.mjs-chat .ctx-pie-label {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  color: #fff;
  font-size: 9px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  text-shadow: 0 0 2px rgba(0, 0, 0, 0.85);
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
/* Attach an image (PLAN44 A5): an icon button in the policy cluster, and
   the pending attachment's chip above the row. */
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
/* The pending attachment: a square tile, filled and cropped — a preview
   you can recognize at a glance, not the whole picture squeezed in. Click
   opens the lightbox. */
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

/* demo framing: the component floats alone (design rule 5 — no panel box
   around it), so the chat wrapper gives up its panel fill and side rule
   here. The Session Panel builds its own `.mjs-chat` and keeps the fill,
   because there the panel genuinely is the component. */
.composer-demo { width: min(560px, 100%); }
.composer-demo .mjs-chat,
.composer-demo .mjs-chat.mode-sidebar {
  background: transparent;
  border-left: 0;
}

/* ---- the outer element -----------------------------------------
   The app skin paints `.composer` as a bottom bar: panel fill, a
   top rule and padding. Here the envelope *is* the component, so the
   footer is reduced to a transparent, zero-padding wrapper. Scoped to
   `.composer-flat` so the live app's composer keeps the bar. */
.mjs-chat .composer.composer-flat,
.mjs-chat.mode-sidebar .composer.composer-flat {
  background: transparent;
  border-top: 0;
  padding: 0;
}

/* ---- the envelope ----------------------------------------------
   one rounded dark-gray card around the draft and its controls, so
   they read as a single surface. The task list is a sibling above
   it, deliberately outside the card. */
.mjs-chat .composer-box {
  background: var(--mjs-panel-2);
  border: 1px solid var(--mjs-line);
  border-radius: 14px;
  padding: 2px 6px 6px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
/* deliberately no `:focus-within` accent: the draft box is not its own
   control here, it is the envelope's surface, so lighting the border on
   focus would break the one-uniform-rectangle read. The caret is the
   focus indicator. (The interactive children — dropdowns, buttons —
   keep their own `:focus-visible` rings.) */

/* the draft gives up its own chrome — the envelope is the surface */
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
/* the `#sb` prefix is load-bearing: the storybook's shared form chrome
   (`:is(#sb,#home) textarea:focus`, specificity 1-1-1) paints a 3px glow on
   every textarea, and a class-only selector cannot outrank an id. The
   draft box is the envelope's surface, not its own control, so it takes
   no ring at all. */
:is(#sb, #home) .composer-box .draft-ta:focus {
  border: 0;
  outline: none;
  box-shadow: none;
}

/* ---- the control row -------------------------------------------
   allowlist on the left; model + context + send pushed right. */
.mjs-chat .composer-box .composer-row {
  gap: 4px;
  flex-wrap: nowrap;
  padding: 0 2px;
}

/* borderless until touched — chrome, not a boxed control */

/* `margin-left:auto` on the first of the pair is what splits the row
   into two clusters: allowlist left, model + level + context + send right. */
.mjs-chat .composer-box .dropdown.dd-model { margin-left: auto; }
.mjs-chat .composer-box .dropdown-value { max-width: 150px; }

.mjs-chat .composer-box .ctx-pie { width: 28px; height: 28px; }

/* ---- the allowlist editor ---------------------------------------
   Ported from cowork's `chat.css`: a labelled mono textarea above the
   envelope, shown only while the shield is toggled on. */
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
/* The turn's metrics, under the composer envelope. Same chip vocabulary as
   the per-card gutter, so the numbers read as one family. */
.mjs-chat .composer-metrics {
  display: flex;
  justify-content: flex-start;
  align-items: center;
  flex-wrap: wrap;
  gap: 0;
  /* Left padding matches `.allowlist-label`, so the metrics and the
     allowlist heading below them share one left edge. */
  padding: 2px 4px 4px 2px;
  font-size: 11px;
  color: var(--mjs-dim);
  user-select: none;
}
.mjs-chat .composer-metrics .metric-chip:last-child { margin-right: 0; }

.mjs-chat .composer-box .dropdown.dd-policy { margin-right: 2px; }

/* ---- the target row ---------------------------------------------
   Where this session works: project, worktree, branch. It sits
   OUTSIDE the envelope, under it — these are not part of composing a
   message, they are the standing context the message will be composed
   in, and putting them inside the plate would say otherwise.

   No surface of its own: bare chrome on the page, like the reference. */
.mjs-chat .composer-targets {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-wrap: wrap;
  padding: 2px 2px 0;
}
/* The row is the one place these triggers appear, and it is the full width
   of the composer, so a long worktree name may have the space a composer-row
   dropdown may not. */
.mjs-chat .composer-targets .dropdown-value { max-width: 180px; }
/* The last two menus sit near the right edge of a narrow sidebar, where a
   288px panel anchored left would hang off the page. */
.mjs-chat.mode-sidebar .composer-targets .dd-worktree .picker-menu,
.mjs-chat.mode-sidebar .composer-targets .dd-branch .picker-menu {
  left: auto;
  right: 0;
}

/* ---- send / stop: one round button ------------------------------ */
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
  /* It is a control, not a status light: it should say so on hover. */
  cursor: pointer;
}
.mjs-chat .composer-box .btn-stop:hover {
  background: color-mix(in srgb, var(--mjs-err) 30%, transparent);
}
/* Pressed, and waiting for the turn's last rows to land. */
.mjs-chat .composer-box .btn-stop:disabled {
  opacity: 0.5;
  cursor: default;
}
"#;

/// the composer inside the chat shell.
pub fn composer(st: &str) -> Node {
    m("div").class("mjs-chat mode-sidebar").child(panel(st))
}

/// standalone demo — the composer the app draws, over the sample store, so
/// the dropdowns and the send button are the real ones and persist nothing.
pub fn demo() -> Node {
    m("div").class("composer-demo").child(composer(samples::EXPR))
}

pub fn register() {
    css::inject("composer", CSS);
}

/// Return to the current draft after sending or changing sessions.
pub fn reset_history(st: &JsValue) {
    js::set_num(st, "historyPosition", -1.0);
    js::set_str(st, "historyDraft", "");
}

/// The model binding commits on change; a draft must also be current while
/// typing, before Enter or a session switch can read it.
pub fn draft_input(st: &JsValue, args: js_sys::Array) -> JsValue {
    let event = js::arg(&args, 0);
    js::set_str(st, "draft", &js::get_str(&js::get(&event, "target"), "value"));
    mjs::redraw();
    JsValue::UNDEFINED
}

/// Navigate the user messages already loaded for the active session.
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

/// The composer wired to angela: the draft, the send/stop flip and the
/// context pie all read the live store.
///
/// Same markup as [`panel`] — the envelope, the dropdowns, the round send
/// button — so the design guide and the running composer stay one thing.
pub fn panel(st: &str) -> Node {
    m("footer").class("composer composer-flat").child(
        m("div").class("composer-inner").kids(vec![
            // Metrics for the last turn, above the composer and above where
            // the allowlist appears. Left-aligned to the same edge as the
            // allowlist label below it, so the two stack on one margin.
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
                    // Clicking into the composer is "I am looking at this
                    // session": its unread mark goes — the same rule the tab
                    // click applies, to the store this composer writes to. A
                    // repeat click counts as well; focus alone cannot say
                    // that, so this listens for click, not focus.
                    .on("click", &format!("{st}.markSessionRead({st}.activeId)"))
                    .on("keydown", &format!("$event.key !== 'Escape' && {st}.treeResetEscape()"))
                    // Enter sends, Shift+Enter is a newline — the placeholder
                    // promises it, so the binding has to honour it.
                    .on("keydown.enter", &format!("{st}.treeOpen ? ($event.preventDefault(), {st}.treeChooseSelected()) : ($event.shiftKey || ($event.preventDefault(), {st}.send()))"))
                    .on("keydown.escape", &format!("{st}.treeEscape($event)"))
                    // Preserve normal cursor movement within multiline drafts.
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
                    // Attach an image (PLAN44 A5): left of the policy
                    // dropdown, per the composer's cluster order. The
                    // hidden input does the choosing.
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
                    // Tool policy. Was a bare shield toggle; a dropdown makes
                    // the two states nameable, and "Unrestricted" visible
                    // rather than implied by an absent panel.
                    dropdown::policy("dd-policy", st),
                    dropdown::model("dd-model", st),
                    dropdown::level("", st),
                    ctx_pie(st),
                    // Stop. It was an indicator with a title that said so;
                    // a round button where Send had been, in a composer
                    // that is waiting, is read as a control — so it is one.
                    m("button")
                        .class("btn-stop")
                        .attr("type", "button")
                        .show(&format!("{st}.waiting"))
                        .on("click", &format!("{st}.stop()"))
                        // Disabled only once the stop is on its way, so the
                        // second press cannot be mistaken for the first not
                        // having worked.
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
            // Under the envelope, not in it: the standing context the next
            // message will be sent with.
            picker::target_row(st).show("$store.mode.advanced"),
        ]),
    )
}

/// Context pie driven by the provider's own token counts.
///
/// `stroke-dasharray` is bound rather than animated: usage lands once per
/// turn, which is a low-frequency change the diff handles fine.
fn ctx_pie(st: &str) -> Node {
    // r=15.5 → circumference ≈ 97.4, as in the app.
    // The fill is context-window fullness — the one formula the state
    // symbol's ring fills by, so the two gauges cannot disagree.
    let frac = session_state::ctx_fraction(
        &format!("{st}.totalTokens"),
        &format!("{st}.session.contextWindow"),
    );
    m("div")
        .class("ctx-pie")
        .bind(
            "title",
            &format!(
                "({st}.promptTokens || 0) + ' prompt / ' + \
                 ({st}.totalTokens || 0) + ' total tokens'"
            ),
        )
        .kids(vec![
            m("svg")
                .at(&[("viewBox", "0 0 36 36"), ("width", "28"), ("height", "28")])
                .kids(vec![
                    m("circle").at(&[
                        ("cx", "18"), ("cy", "18"), ("r", "15.5"),
                        ("fill", "none"), ("stroke", "currentColor"),
                        ("class", "ctx-pie-track"), ("stroke-width", "4"),
                    ]),
                    m("circle")
                        .at(&[
                            ("cx", "18"), ("cy", "18"), ("r", "15.5"),
                            ("fill", "none"), ("stroke", "currentColor"),
                            ("class", "ctx-pie-fill"), ("stroke-width", "4"),
                            ("stroke-linecap", "round"),
                            ("transform", "rotate(-90 18 18)"),
                        ])
                        .bind(
                            "stroke-dasharray",
                            &format!("({frac} * 97.4).toFixed(1) + ' 97.4'"),
                        ),
                ]),
            m("span").class("ctx-pie-label").text(
                &format!(
                    "({st}.totalTokens || 0) >= 1000 ? \
                     Math.round(({st}.totalTokens || 0) / 1000) + 'k' : \
                     ({st}.totalTokens || 0)"
                ),
            ),
        ])
}

/// The allowlist editor, toggled by the shield button.
///
/// Mirrors cowork's `composerTpl`: a labelled textarea that saves on **blur**
/// — persisting per keystroke would rewrite the session file on every
/// character. The label says whether the agent default or a UI override is in
/// force, because "why was that denied?" is otherwise invisible.
fn allowlist_panel(st: &str) -> Node {
    m("div")
        .class("allowlist-panel")
        // Hidden entirely while unrestricted: the rules have no effect then,
        // and an editable box whose contents are ignored invites the reader
        // to trust it. The rules are kept, not cleared — switching back to
        // Allowlist brings the panel and its text straight back.
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

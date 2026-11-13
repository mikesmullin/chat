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

use crate::components::dropdown;
use crate::components::task_list;
use crate::css;

pub const CSS: &str = r#"
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
.mjs-chat .composer-box .allowlist-toggle {
  width: 30px;
  height: 30px;
  background: transparent;
  border-color: transparent;
  color: var(--mjs-dim);
  font-size: 15px;
}
.mjs-chat .composer-box .allowlist-toggle:hover {
  color: var(--mjs-ink);
  border-color: var(--mjs-line);
}

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
/* the shield reads as lit while the panel is open */
.mjs-chat .composer-box .allowlist-toggle.active {
  color: var(--mjs-accent);
  border-color: var(--mjs-accent);
  background: color-mix(in srgb, var(--mjs-accent) 14%, transparent);
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
}
"#;

const DRAFT: &str = "$store.demo.composerDraft";
const WAITING: &str = "$store.demo.composerWaiting";

/// context window: static 38% pie (r=15.5 → C≈97.4, like the app).
fn ctx_pie() -> Node {
    m("div")
        .class("ctx-pie")
        .attr("title", "38000 / 100000 tokens")
        .kids(vec![
            m("svg")
                .at(&[("viewBox", "0 0 36 36"), ("width", "28"), ("height", "28")])
                .kids(vec![
                    m("circle").at(&[
                        ("cx", "18"), ("cy", "18"), ("r", "15.5"),
                        ("fill", "none"), ("stroke", "currentColor"),
                        ("class", "ctx-pie-track"), ("stroke-width", "4"),
                    ]),
                    m("circle").at(&[
                        ("cx", "18"), ("cy", "18"), ("r", "15.5"),
                        ("fill", "none"), ("stroke", "currentColor"),
                        ("class", "ctx-pie-fill"), ("stroke-width", "4"),
                        ("stroke-linecap", "round"),
                        ("transform", "rotate(-90 18 18)"),
                        ("stroke-dasharray", "37.0 97.4"),
                    ]),
                ]),
            m("span").class("ctx-pie-label").child(lit("38k")),
        ])
}

/// the control row: allowlist on the left, model + context + send right.
fn composer_row() -> Node {
    m("div").class("composer-row").kids(vec![
        m("button")
            .class("icon-btn allowlist-toggle")
            .attr("type", "button")
            .attr("title", "Allowlist")
            .child(Node::icon("shield-check")),
        dropdown::model("dd-model"),
        dropdown::level(""),
        ctx_pie(),
        // waiting flag flips through the demo store: stop shows while
        // waiting, send (disabled on an empty draft) otherwise.
        m("button")
            .class("btn-stop")
            .attr("type", "button")
            .show(WAITING)
            .on("click", "$store.demo.composerStop()")
            .attr("title", "Stop agent")
            .child(Node::icon("stop")),
        m("button")
            .class("btn-primary")
            .attr("type", "button")
            .show(&format!("!{WAITING}"))
            .bind("disabled", &format!("!{DRAFT}.trim()"))
            .on("click", "$store.demo.composerSend()")
            .attr("title", "Send")
            .child(Node::icon("arrow-up")),
    ])
}

/// the prompt bar alone — the part the panel reuses.
pub fn panel() -> Node {
    m("footer").class("composer composer-flat").child(
        m("div").class("composer-inner").kids(vec![
            task_list::task_list(task_list::SAMPLES),
            m("div").class("composer-box").kids(vec![
                m("textarea")
                    .class("draft-ta")
                    .model(DRAFT)
                    .attr("placeholder", "Message… (Enter send, Shift+Enter newline)"),
                composer_row(),
            ]),
        ]),
    )
}

/// the composer, bound to the demo store.
pub fn composer() -> Node {
    m("div").class("mjs-chat mode-sidebar").child(panel())
}

/// standalone demo.
pub fn demo() -> Node {
    m("div").class("composer-demo").child(composer())
}

pub fn register() {
    css::inject("composer", CSS);
}

// ===========================================================================
// Live variant — bound to `$store.live` instead of the demo store.
// ===========================================================================

const LIVE_DRAFT: &str = "$store.live.draft";
const LIVE_WAITING: &str = "$store.live.waiting";

/// The composer wired to angela: the draft, the send/stop flip and the
/// context pie all read the live store.
///
/// Same markup as [`panel`] — the envelope, the dropdowns, the round send
/// button — so the design guide and the running composer stay one thing.
pub fn panel_live() -> Node {
    m("footer").class("composer composer-flat").child(
        m("div").class("composer-inner").kids(vec![
            allowlist_panel(),
            m("div").class("composer-box").kids(vec![
                m("textarea")
                    .class("draft-ta")
                    .model(LIVE_DRAFT)
                    .attr("placeholder", "Message… (Enter send, Shift+Enter newline)")
                    // Enter sends, Shift+Enter is a newline — the placeholder
                    // promises it, so the binding has to honour it.
                    .on("keydown.enter", "$event.shiftKey || ($event.preventDefault(), $store.live.send())"),
                m("div").class("composer-row").kids(vec![
                    m("button")
                        .class("icon-btn allowlist-toggle")
                        .attr("type", "button")
                        .attr("title", "Allowed commands")
                        .attr("aria-label", "Toggle the allowlist")
                        .bind(
                            "class",
                            "($store.live.allowlistOpen ? 'active' : '')",
                        )
                        .on("click", "$store.live.toggleAllowlist()")
                        .child(Node::icon("shield-check")),
                    dropdown::model("dd-model"),
                    dropdown::level(""),
                    ctx_pie_live(),
                    m("button")
                        .class("btn-stop")
                        .attr("type", "button")
                        .show(LIVE_WAITING)
                        .attr("title", "Waiting for the model")
                        .child(Node::icon("stop")),
                    m("button")
                        .class("btn-primary")
                        .attr("type", "button")
                        .show(&format!("!{LIVE_WAITING}"))
                        .bind("disabled", &format!("!{LIVE_DRAFT}.trim()"))
                        .on("click", "$store.live.send()")
                        .attr("title", "Send")
                        .child(Node::icon("arrow-up")),
                ]),
            ]),
        ]),
    )
}

/// Context pie driven by the provider's own token counts.
///
/// `stroke-dasharray` is bound rather than animated: usage lands once per
/// turn, which is a low-frequency change the diff handles fine.
fn ctx_pie_live() -> Node {
    // r=15.5 → circumference ≈ 97.4, as in the app.
    let frac = "Math.min(1, ($store.live.totalTokens || 0) / \
                Math.max(1, ($store.live.session.contextWindow || 128000)))";
    m("div")
        .class("ctx-pie")
        .bind(
            "title",
            "($store.live.promptTokens || 0) + ' prompt / ' + \
             ($store.live.totalTokens || 0) + ' total tokens'",
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
                "($store.live.totalTokens || 0) >= 1000 ? \
                 Math.round(($store.live.totalTokens || 0) / 1000) + 'k' : \
                 ($store.live.totalTokens || 0)",
            ),
        ])
}


/// The allowlist editor, toggled by the shield button.
///
/// Mirrors cowork's `composerTpl`: a labelled textarea that saves on **blur**
/// — persisting per keystroke would rewrite the session file on every
/// character. The label says whether the agent default or a UI override is in
/// force, because "why was that denied?" is otherwise invisible.
fn allowlist_panel() -> Node {
    m("div")
        .class("allowlist-panel")
        .show("$store.live.allowlistOpen")
        .kids(vec![
            m("label").class("allowlist-label").kids(vec![
                lit("allowed commands (one rule per line)"),
                m("span")
                    .class("allowlist-source")
                    .text(
                        "$store.live.allowlistOverridden ? '· ui override' : '· agent default'",
                    ),
                m("button")
                    .class("allowlist-revert")
                    .attr("type", "button")
                    .attr("title", "Restore the agent default")
                    .show("$store.live.allowlistOverridden")
                    .on("click", "$store.live.revertAllowlist()")
                    .child(lit("revert")),
            ]),
            m("textarea")
                .class("allowlist-ta")
                .attr("spellcheck", "false")
                .attr(
                    "placeholder",
                    "file_io__read_file\nterm_local__bash:ls.*\n\n(empty = deny everything)",
                )
                .model("$store.live.allowlist")
                .on("blur", "$store.live.saveAllowlist()"),
        ])
}

//! `agent-editor` — an agent definition, in the inspector column.
//!
//! The third column is contextual: what it shows is whatever is selected
//! elsewhere on the page. Selecting an agent in the rail puts its definition
//! here, and `+` puts an empty one here.
//!
//! The form's shape follows what the fields *are*, not what type they have.
//! `soul`, `goals` and `memory` are three boxes rather than one, because they
//! are three different kinds of writing with three different authors: the
//! soul is written once by whoever defined the agent, the goals are the
//! user's steering, and the memory is the agent's own notes — which it may
//! rewrite and you may read. Flattening them into one "system prompt" box
//! would lose exactly that distinction.
//!
//! `engagement` is a three-way switch with its meaning spelled out under it,
//! because it is the field that decides whether the agent ever speaks, and a
//! bare word like "relevant" does not say what will happen.

use rustex1_kit::h::{lit, m, Node};

use crate::css;
use crate::components::{dropdown, validation_info};

pub const CSS: &str = r#"
:is(#sb, #home) .agent-editor {
  display: flex; flex-direction: column; gap: 14px;
  font-size: 12px; color: var(--gray);
}
:is(#sb, #home) .agent-editor-head {
  display: flex; align-items: center; gap: 8px;
}
:is(#sb, #home) .agent-editor-title {
  flex: 1; min-width: 0; font-size: 13px; color: var(--fg);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
:is(#sb, #home) .ae-field { display: flex; flex-direction: column; gap: 5px; }
:is(#sb, #home) .ae-label {
  font-size: 11px; color: var(--gray-dim); line-height: 16px;
  /* Title + hint belong on different lines: the label box dissolves
     (display: contents) so its two spans join the field's column, and the
     hint is ordered below every control in the field — the same
     label/control/hint stacking the shared `field` brick already does. */
  display: contents;
}
:is(#sb, #home) .ae-label .hint {
  order: 1; display: block; margin-top: 2px;
  color: var(--gray-dim); opacity: .7; font-size: 10px;
}
:is(#sb, #home) .ae-field input,
:is(#sb, #home) .ae-field textarea {
  width: 100%; box-sizing: border-box;
  background: var(--panel-2); color: var(--fg);
  border: 1px solid transparent; border-radius: 8px;
  padding: 7px 9px; font: inherit; line-height: 1.5;
  resize: vertical;
}
:is(#sb, #home) .ae-field textarea { min-height: 64px; font-size: 12px; }
/* A turn count is two or three digits; a full-width box for it reads as a
   text field and invites prose. The spinner is noise at this size. */
:is(#sb, #home) .ae-field input.ae-num { width: 7em; }
:is(#sb, #home) .ae-field input.ae-num::-webkit-outer-spin-button,
:is(#sb, #home) .ae-field input.ae-num::-webkit-inner-spin-button {
  -webkit-appearance: none; margin: 0;
}
:is(#sb, #home) .ae-field input.ae-num { -moz-appearance: textfield; appearance: textfield; }
:is(#sb, #home) .ae-field input:focus,
:is(#sb, #home) .ae-field textarea:focus {
  outline: none; border-color: var(--gray-dim);
}
:is(#sb, #home) .ae-field input::placeholder,
:is(#sb, #home) .ae-field textarea::placeholder { color: var(--gray-dim); }

/* the three-way engagement switch */
:is(#sb, #home) .ae-modes { display: flex; gap: 4px; }
:is(#sb, #home) .ae-mode {
  flex: 1; display: inline-flex; align-items: center; justify-content: center; gap: 5px;
  border: 0; border-radius: 7px; background: var(--panel-2); color: var(--gray-dim);
  padding: 6px 4px; cursor: pointer; font: inherit; font-size: 11px; line-height: 18px;
}
:is(#sb, #home) .ae-mode:hover { color: var(--fg); }
:is(#sb, #home) .ae-mode.is-on { color: var(--accent); }
:is(#sb, #home) .ae-mode i { line-height: 18px; }
:is(#sb, #home) .ae-mode-note {
  font-size: 11px; color: var(--gray-dim); line-height: 1.5; min-height: 16px;
}

:is(#sb, #home) .ae-color { display: flex; gap: 8px; align-items: center; }
:is(#sb, #home) .ae-color input[type="color"] {
  width: 38px; height: 30px; padding: 0; flex: 0 0 auto;
  background: var(--panel-2); border: 1px solid transparent; border-radius: 8px;
  cursor: pointer;
}
:is(#sb, #home) .ae-color input[type="color"]::-webkit-color-swatch-wrapper { padding: 3px; }
:is(#sb, #home) .ae-color input[type="color"]::-webkit-color-swatch {
  border: 0; border-radius: 5px;
}
:is(#sb, #home) .ae-color-hex { flex: 1; font-family: var(--mono, monospace); }

/* the icon row: a live glyph the size of the colour swatch, then its name */
:is(#sb, #home) .ae-icon { display: flex; gap: 8px; align-items: center; }
:is(#sb, #home) .ae-icon .ae-icon-preview {
  width: 38px; height: 30px; flex: 0 0 auto;
  display: inline-flex; align-items: center; justify-content: center;
  background: var(--panel-2); border-radius: 8px;
  /* Drawn in the agent's own colour, so the two fields preview together. */
  color: var(--agent-accent, var(--accent));
  font-size: 17px; line-height: 1;
}
:is(#sb, #home) .ae-icon input { flex: 1; font-family: var(--mono, monospace); }

:is(#sb, #home) .ae-actions {
  display: flex; align-items: center; gap: 8px; padding-top: 2px;
}
:is(#sb, #home) .ae-actions .spacer { flex: 1; }
:is(#sb, #home) .ae-btn {
  display: inline-flex; align-items: center; gap: 5px;
  border: 0; border-radius: 7px; background: var(--panel-2); color: var(--gray);
  padding: 6px 11px; cursor: pointer; font: inherit; font-size: 12px; line-height: 18px;
}
:is(#sb, #home) .ae-btn:hover { color: var(--fg); }
:is(#sb, #home) .ae-btn.primary { color: var(--accent); }
:is(#sb, #home) .ae-btn.danger:hover { color: var(--danger, #d9534f); }
:is(#sb, #home) .ae-btn.is-confirm { color: var(--danger, #d9534f); }
:is(#sb, #home) .ae-btn[disabled] { opacity: .5; cursor: default; }
/* The shields switch (PLAN44 D12), per the Switch Anatomy reference: a
   pill track ~2× its white handle, a green check in the handle, and the
   state word outside on the right. The track is the alarm — quiet gray
   while the door is shut, red while it stands open. Colours sampled from
   the reference: #34c671 (green), #d4d4d7 (track gray). */
:is(#sb, #home) .shields-switch {
  appearance: none; border: 0; background: transparent; cursor: pointer;
  display: inline-flex; align-items: center; gap: 8px; padding: 0;
  /* The field column stretches its children; hug the control instead. */
  align-self: flex-start;
}
:is(#sb, #home) .shields-track {
  position: relative; width: 32px; height: 17px;
  border-radius: 999px; background: #00ad0d;
  transition: background 140ms ease;
}
:is(#sb, #home) .shields-knob {
  position: absolute; top: 50%; margin-top: -8px;
  /* Handle right = Up, handle left = Down. */
  left: 15px;
  width: 16px; height: 16px; border-radius: 50%; background: #fff;
  box-shadow: 0 1px 2px rgba(0,0,0,.3);
  display: inline-flex; align-items: center; justify-content: center;
  transition: left 160ms cubic-bezier(.2,.8,.2,1);
}
:is(#sb, #home) .shields-icon {
  /* Same hue as the track. Up wears the Fill face (a solid glyph); Down
     wears Bold (a warned glyph) — the weight changes with the state. */
  font-size: 10px; color: #00ad0d;
}
:is(#sb, #home) .shields-label {
  display: inline-flex; align-items: center; gap: 5px;
  font: 500 12px/1 var(--sans); color: var(--text);
}
:is(#sb, #home) .shields-switch.is-down .shields-track { background: #ef4444; }
:is(#sb, #home) .shields-switch.is-down .shields-knob { left: 1px; }
:is(#sb, #home) .shields-switch.is-down .shields-icon { color: #b91c1c; }
/* Field prefix icons (PLAN44 UI): white, bold phosphor glyphs. The globe
   is inset in the input (absolute, so the field keeps its height); the
   device glyph is each row's bullet; the shield stands by the switch. */
:is(#sb, #home) .ae-input-icon { position: relative; display: flex; align-items: center; }
:is(#sb, #home) .ae-input-glyph {
  position: absolute; left: 10px; z-index: 1; pointer-events: none;
  color: #fff; font-size: 14px; line-height: 1;
}
:is(#sb, #home) .ae-input-icon input { padding-left: 32px; }
:is(#sb, #home) .ae-device-icon {
  color: #fff; font-size: 12px; line-height: 1.6; flex: 0 0 auto;
}
:is(#sb, #home) .shields-glyph { color: #fff; font-size: 13px; line-height: 1; }
/* Recognized devices (PLAN44 A2): one row per issued session cookie. */
:is(#sb, #home) .ae-devices {
  display: flex; flex-direction: column; gap: 4px;
  padding: 8px 10px; border: 1px solid var(--line-soft, var(--line));
  border-radius: 8px; min-height: 34px;
}
:is(#sb, #home) .ae-device {
  display: flex; flex-wrap: wrap; align-items: center; gap: 6px 10px;
}
:is(#sb, #home) .ae-device-fp,
:is(#sb, #home) .ae-device-ip {
  font: 400 11px/1.6 var(--mono); color: var(--gray-dim);
  overflow-wrap: anywhere;
}
:is(#sb, #home) .ae-device-when {
  font: 400 11px/1.6 var(--mono); color: var(--gray-dim);
}
:is(#sb, #home) .ae-device-del {
  border: 0; background: transparent; color: var(--gray-dim);
  cursor: pointer; padding: 2px; border-radius: 6px; font-size: 12px;
  line-height: 1;
  /* The row's action, not its content: hidden until the row is hovered —
     same idiom as the transcript's fork button. Keyboard still reaches it. */
  opacity: 0; pointer-events: none; transition: opacity 100ms ease;
}
:is(#sb, #home) .ae-device:hover .ae-device-del,
:is(#sb, #home) .ae-bans:hover .ae-device-del,
:is(#sb, #home) .ae-device-del:focus-visible {
  opacity: 1; pointer-events: auto;
}
:is(#sb, #home) .ae-device-del:hover { color: var(--danger, #f87171); background: var(--panel-2); }
:is(#sb, #home) .ae-bans {
  display: flex; flex-wrap: wrap; gap: 6px 10px; align-items: center;
  margin-top: 6px;
}

"#;

const ST: &str = "$store.agents";

fn field(label: &str, hint: &str, model: &str, placeholder: &str, rows: usize) -> Node {
    let input = if rows == 0 {
        m("input")
            .attr("type", "text")
            .attr("placeholder", placeholder)
            .model(model)
    } else {
        m("textarea")
            .attr("rows", &rows.to_string())
            .attr("placeholder", placeholder)
            .model(model)
    };
    m("div").class("ae-field").kids(vec![
        m("label").class("ae-label").kids(vec![
            m("span").child(lit(label)),
            m("span").class("hint").child(lit(hint)),
        ]),
        input,
    ])
}

fn mode(value: &str, label: &str, icon: &str) -> Node {
    m("button")
        .class("ae-mode")
        .attr("type", "button")
        .bind(
            "class",
            &format!("({ST}.draft.engagement === '{value}' ? 'is-on' : '')"),
        )
        .on("click", &format!("{ST}.setEngagement('{value}')"))
        .kids(vec![
            m("i").class(&format!("ph {icon}")),
            m("span").child(lit(label)),
        ])
}

/// The editor, shown when an agent is selected or being created.
pub fn agent_editor() -> Node {
    m("div")
        .class("agent-editor")
        .show(&format!("{ST}.open"))
        .kids(vec![
            m("div").class("agent-editor-head").kids(vec![
                // The agent being edited, drawn as itself.
                Node::icon_dyn(&format!("({ST}.draft.icon || 'robot')")),
                m("span")
                    .class("agent-editor-title")
                    .text(&format!(
                        "({ST}.creating ? 'New agent' : ({ST}.draft.name || 'Agent'))"
                    )),
            ]),
            // The verdict banner: always at the top of the form, just above
            // the first field — one brick, every aside form.
            validation_info::live(
                &format!("({ST}.error ? 'error' : 'ok')"),
                &format!("({ST}.error ? {ST}.error : {ST}.saved)"),
            ),
            field("Name", "the @handle", &format!("{ST}.draft.name"), "scout", 0),
            field(
                "Description",
                "what it is for",
                &format!("{ST}.draft.description"),
                "an ops engineer who reads deploy logs",
                2,
            ),

            // --- engagement ------------------------------------------------
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Engagement")),
                    m("span").class("hint").child(lit("when it speaks")),
                ]),
                m("div").class("ae-modes").kids(vec![
                    mode("invoked", "Invoked", "ph-at"),
                    mode("relevant", "Relevant", "ph-funnel"),
                    mode("engaged", "Engaged", "ph-chats-circle"),
                ]),
                // Spelled out, because "relevant" does not say what happens.
                m("p")
                    .class("ae-mode-note")
                    .text(&format!("{ST}.engagementLabel({ST}.draft.engagement)")),
            ]),

            // --- the three prompt parts ------------------------------------
            field(
                "Soul",
                "who it is",
                &format!("{ST}.draft.soul"),
                "You are Scout, a terse ops engineer.",
                5,
            ),
            field(
                "Goals",
                "what it is working toward",
                &format!("{ST}.draft.goals"),
                "Keep deploys healthy.",
                3,
            ),
            field(
                "Memory",
                "what it has learned — it may rewrite this",
                &format!("{ST}.draft.memory"),
                "(empty)",
                3,
            ),

            // The glyph this agent is drawn with, above the colour it is
            // drawn in: the two together are what you recognise it by in
            // the rail, the roster and every byline.
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Icon")),
                    m("span")
                        .class("hint")
                        .child(lit("a phosphor name; blank = robot")),
                ]),
                m("div")
                    .class("ae-icon")
                    // The preview wears the draft's colour, which is also
                    // how the colour field gets a shape to show itself on.
                    .bind(
                        "style",
                        &format!(
                            "('--agent-accent: ' + ({ST}.draft.accentColor || '#a78bfa'))"
                        ),
                    )
                    .kids(vec![
                        // The preview *is* the validation: a name the set
                        // does not have draws nothing, which is the only
                        // way to tell a typo from a real icon without
                        // shipping the whole catalogue in a dropdown.
                        m("span")
                            .class("ae-icon-preview")
                            .child(Node::icon_dyn(&format!(
                                "({ST}.draft.icon || 'robot')"
                            ))),
                        m("input")
                            .attr("type", "text")
                            .attr("spellcheck", "false")
                            .attr("placeholder", "robot")
                            .attr("title", "phosphoricons.com — the name under the glyph")
                            .model(&format!("{ST}.draft.icon")),
                    ]),
            ]),

            // The colour this agent is drawn in, everywhere it appears.
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Accent colour")),
                    m("span").class("hint").child(lit("how it is drawn")),
                ]),
                m("div").class("ae-color").kids(vec![
                    m("input")
                        .attr("type", "color")
                        .model(&format!("{ST}.draft.accentColor")),
                    // The hex beside the swatch, because a swatch alone
                    // cannot be copied, pasted or read out.
                    m("input")
                        .attr("type", "text")
                        .class("ae-color-hex")
                        .attr("spellcheck", "false")
                        .model(&format!("{ST}.draft.accentColor")),
                ]),
            ]),

            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").child(lit("Model")),
                dropdown::agent_option("model", "cpu"),
            ]),
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").child(lit("Reasoning")),
                dropdown::agent_option("reasoningEffort", "brain"),
            ]),
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Max turns")),
                    m("span")
                        .class("hint")
                        .child(lit("model round-trips per reply; blank = 5")),
                ]),
                m("input")
                    .class("ae-num")
                    .attr("type", "number")
                    .attr("min", "1")
                    .attr("max", "1000")
                    .attr("step", "1")
                    // Each tool call spends one. An agent that reads a few
                    // files before answering needs more than a chat reply
                    // does, which is why this is per-agent.
                    .attr("placeholder", "5")
                    .model(&format!("{ST}.draft.maxTurns")),
            ]),
            // Where the agent's file and shell tools believe the disk begins:
            // an explicit root, or the project root if left blank. Blank is
            // the placeholder of the field, not an error. Relative values
            // resolve under the project root, as `${projectRoot}` expands.
            field(
                "Workspace root",
                "file-io and the shell start here; blank = project root \
                 — host-only (no effect on sandboxes)",
                &format!("{ST}.draft.cwd"),
                "/workspace/blz",
                0,
            ),
            // PLAN55 D1: the agent's Containerfile (absolute host path).
            // Set, and every turn of this agent runs in a container built
            // from `sbox-<this file's directory>:latest` — one per session,
            // launched at its first turn there. Empty = the host, unchanged.
            field(
                "Containerfile",
                "absolute host path; empty = runs on the host",
                &format!("{ST}.draft.containerFile"),
                "/workspace/models/sbox/video/Containerfile",
                0,
            ),
            field(
                "MCP servers",
                "one per line",
                &format!("{ST}.draft.mcp"),
                "file-io\nterm-local",
                2,
            ),
            field(
                "Allowlist",
                "auto-approved tools, one per line — empty approves nothing",
                &format!("{ST}.draft.allowlist"),
                "file_io__read_file",
                4,
            ),

            m("div").class("ae-actions").kids(vec![
                m("button")
                    .class("ae-btn primary")
                    .attr("type", "button")
                    .bind("disabled", &format!("{ST}.saving"))
                    .on("click", &format!("{ST}.save()"))
                    .kids(vec![
                        Node::icon("check").class("ph ph-check"),
                        m("span").text(&format!("({ST}.saving ? 'Saving…' : 'Save')")),
                    ]),
                // Who else may reach this agent (PLAN44 C4) — the same
                // share dialog sessions use; the dialog is kind-agnostic.
                m("button")
                    .class("ae-btn")
                    .attr("type", "button")
                    .show(&format!("!{ST}.creating"))
                    .on("click", "$store.share.open('agent', $store.agents.draft.name)")
                    .kids(vec![
                        Node::icon("users-three").class("ph ph-users-three"),
                        m("span").child(lit("Share…")),
                    ]),
                m("span").class("spacer"),
                m("button")
                    .class("ae-btn danger")
                    .attr("type", "button")
                    .show(&format!("!{ST}.creating"))
                    .bind("class", &format!("({ST}.confirmDelete ? 'is-confirm' : '')"))
                    .on("click", &format!("{ST}.remove()"))
                    .kids(vec![
                        Node::icon("trash").class("ph ph-trash"),
                        // The second press is the destructive one, and it
                        // says so rather than opening a dialog over a panel.
                        m("span").text(&format!(
                            "({ST}.confirmDelete ? 'Really delete?' : 'Delete')"
                        )),
                    ]),
            ]),
        ])
}

/// The app's own settings, in the same column.
///
/// Reuses the editor's field and button styles rather than growing a second
/// set: an inspector panel is an inspector panel, and two that look
/// different for no reason is the sort of thing nobody decides but everyone
/// notices.
pub fn settings_panel() -> Node {
    let st = "$store.settings";
    m("div")
        // Shares the editor's styles, keeps its own name: two panels that
        // are indistinguishable in the DOM are two panels nothing can
        // target — a test, a stylesheet or a later feature.
        .class("agent-editor settings-panel")
        .show(&format!("{st}.open"))
        .init(&format!("{st}.load()"))
        .kids(vec![
            m("div").class("agent-editor-head").kids(vec![
                Node::icon("gear").class("ph ph-gear"),
                m("span").class("agent-editor-title").child(lit("Settings")),
            ]),
            // The verdict banner: always at the top of the form, just above
            // the first field — one brick, every aside form. This form
            // auto-saves, so the banner is only ever the bad news.
            validation_info::live("'error'", "$store.ident.error"),
            // ---- admin: the perimeter (PLAN44 D12) --------------------
            m("div").class("ae-field").show("$store.ident.admin").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Shields")),
                    m("span").class("hint").child(lit("up: strangers are firewalled · down: invite by name")),
                ]),
                // The anatomy (Switch Anatomy reference): one pill track
                // ~2× the handle, white handle with a green check icon,
                // state word *outside* on the right — with its shield
                // prefixed: check when the door is shut, plain when open.
                m("button")
                    .attr("type", "button")
                    .attr("title", "Toggle shields")
                    .bind(
                        "class",
                        "($store.ident.shields === 'down' ? 'shields-switch is-down' : 'shields-switch')",
                    )
                    .on("click", "$store.ident.setShields($store.ident.shields !== 'down')")
                    .kids(vec![
                        m("span").class("shields-track").child(
                            m("span")
                                .class("shields-knob")
                                // Check-fat in the Fill face when the
                                // door is shut (solid); x in the Bold face
                                // when it is open. ONE class binding —
                                // stacked binds clobber each other.
                                .child(m("i").bind(
                                    "class",
                                    "($store.ident.shields === 'down' ? 'ph ph-bold ph-x shields-icon' : 'ph ph-fill ph-check-fat shields-icon')",
                                )),
                        ),
                        m("span").class("shields-label").kids(vec![
                            // The state shield prefixes the word.
                            m("i").bind(
                                "class",
                                "($store.ident.shields === 'down' ? 'ph ph-bold ph-shield shields-glyph' : 'ph ph-bold ph-shield-check shields-glyph')",
                            ),
                            m("span")
                                .text("($store.ident.shields === 'down' ? 'Down' : 'Up')"),
                        ]),
                    ]),
                m("div")
                    .class("ae-bans")
                    .for_each("ip in $store.ident.bans")
                    .kids(vec![
                        m("span").class("ae-device-ip").text("ip"),
                        m("button")
                            .class("ae-device-del")
                            .attr("type", "button")
                            .attr("title", "Unban")
                            .on("click", "$store.ident.unban(ip)")
                            .child(Node::icon("x").class("ph ph-x")),
                    ]),
            ]),
        ])
}

pub fn register() {
    css::inject("agent-editor", CSS);
}

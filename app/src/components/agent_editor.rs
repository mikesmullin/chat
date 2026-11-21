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
use crate::components::dropdown;

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
  display: flex; align-items: baseline; gap: 6px;
}
:is(#sb, #home) .ae-label .hint { color: var(--gray-dim); opacity: .7; font-size: 10px; }
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
:is(#sb, #home) .ae-error { color: var(--danger, #d9534f); font-size: 11px; line-height: 1.5; }
:is(#sb, #home) .ae-saved { color: var(--ok, #4f9d69); font-size: 11px; line-height: 1.5; }

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
                "file-io and the shell start here; blank = project root",
                &format!("{ST}.draft.cwd"),
                "/workspace/blz",
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

            m("p").class("ae-error").show(&format!("{ST}.error")).text(&format!("{ST}.error")),
            m("p").class("ae-saved").show(&format!("{ST}.saved")).text(&format!("{ST}.saved")),

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
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Full name")),
                    m("span").class("hint").child(lit("how you are named")),
                ]),
                m("input")
                    .attr("type", "text")
                    .attr("placeholder", "Bob")
                    .model(&format!("{st}.draft.user__full_name")),
            ]),
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Location")),
                    m("span").class("hint").child(lit("where you are")),
                ]),
                m("input")
                    .attr("type", "text")
                    .attr("placeholder", "City, State")
                    .model(&format!("{st}.draft.location")),
            ]),
            m("p")
                .class("ae-saved")
                .show(&format!("{st}.saved"))
                .text(&format!("{st}.saved")),
            m("div").class("ae-actions").child(
                m("button")
                    .class("ae-btn primary")
                    .attr("type", "button")
                    .bind("disabled", &format!("{st}.saving"))
                    .on("click", &format!("{st}.save()"))
                    .kids(vec![
                        Node::icon("check").class("ph ph-check"),
                        m("span").text(&format!("({st}.saving ? 'Saving…' : 'Save')")),
                    ]),
            ),
        ])
}

pub fn register() {
    css::inject("agent-editor", CSS);
}

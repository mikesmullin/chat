//! `markdown` — the look of rendered markdown.
//!
//! This component has no markup of its own, and that is the point:
//! `md-body` is never an element, it is a class put **on** one —
//! `bubble-assistant md-body`, `fold-body md-body`, `thread-text
//! md-body`. What it owns is the style contract for a block of
//! model-written prose: headings, lists, inline code, fenced blocks,
//! links. Anything that renders markdown wears the class and gets the
//! same typography.
//!
//! The class itself stays a literal in the templates that wear it —
//! a Rust constant for it would be one more name for `md-body`, used by
//! some call sites and not others.
//!
//! It stands on its own rather than living inside `assistant_card`
//! because three unrelated callers already use it, and the first of
//! them is not more entitled to the rules than the others.

use crate::css;

pub const CSS: &str = r#"
.mjs-chat .md-body { white-space: normal; word-wrap: break-word; overflow-wrap: anywhere; }
.mjs-chat .md-body > :first-child { margin-top: 0; }
.mjs-chat .md-body > :last-child { margin-bottom: 0; }
.mjs-chat .md-body p { margin: 0.55em 0; }
.mjs-chat .md-body h1, .mjs-chat .md-body h2, .mjs-chat .md-body h3 {
  color: var(--mjs-ink); font-weight: 600; line-height: 1.3; margin: 0.9em 0 0.4em;
}
.mjs-chat .md-body ul, .mjs-chat .md-body ol { margin: 0.45em 0; padding-left: 1.35em; }
.mjs-chat .md-body code {
  font-family: var(--mjs-mono);
  font-size: 0.88em;
  background: var(--mjs-panel-2);
  border: 1px solid var(--mjs-line);
  border-radius: 4px;
  padding: 0.1em 0.35em;
}
.mjs-chat .md-body pre {
  margin: 0.65em 0;
  padding: 10px 12px;
  background: var(--mjs-panel);
  border: 1px solid var(--mjs-line);
  border-radius: 10px;
  overflow-x: auto;
  font-size: 12.5px;
}
.mjs-chat .md-body pre code { background: transparent; border: 0; padding: 0; }
.mjs-chat .md-body a { color: var(--mjs-accent); }
.mjs-chat .md-body a:hover { text-decoration: underline; }
"#;

pub fn register() {
    css::inject("markdown", CSS);
}

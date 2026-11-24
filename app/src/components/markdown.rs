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
.mjs-chat .md-body .md-table-wrap { max-width: 100%; overflow-x: auto; margin: 0.65em 0; }
.mjs-chat .md-body table { border-collapse: collapse; width: 100%; }
.mjs-chat .md-body th, .mjs-chat .md-body td { border: 1px solid var(--mjs-line); padding: 0.45em 0.65em; text-align: left; vertical-align: top; overflow-wrap: normal; }
.mjs-chat .md-body th { background: var(--mjs-panel-2); font-weight: 600; }
.mjs-chat .md-body .md-align-center { text-align: center; }
.mjs-chat .md-body .md-align-right { text-align: right; }
.mjs-chat .md-body .md-align-left { text-align: left; }
/* An attached image in a message: a small thumbnail, **fit** — the whole
   picture in a 150px frame, never the natural size overflowing the room.
   Click opens the lightbox (the transcript delegates the click). */
.mjs-chat .md-body img {
  max-width: 150px; max-height: 150px;
  object-fit: contain;
  border-radius: 8px;
  background: var(--mjs-panel-2);
  display: block;
  margin: 6px 0;
  cursor: zoom-in;
}
"#;

pub fn register() {
    css::inject("markdown", CSS);
}

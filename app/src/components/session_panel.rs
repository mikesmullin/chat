use rustex1_kit::h::{m, Node};

use crate::components::{composer, history_tree, roster, session};
use crate::css;

pub const LIVE_STORE: &str = "$store.live";

pub const CSS: &str = r#"
.mjs-chat .main > div:has(> .messages) {
  display: flex; flex-direction: column; flex: 1 1 auto; min-height: 0;
}

.mjs-chat .msg-block {
  display: flex;
  flex-direction: column;
  gap: 0;
  max-width: min(780px, 92%);
  align-self: flex-start;
}
.mjs-chat.mode-sidebar .msg-block { max-width: 94%; }
.mjs-chat .msg-block.msg-user { align-self: flex-end; max-width: min(720px, 85%); }
.mjs-chat .msg-block.msg-hidden { opacity: 0.45; }
.mjs-chat .msg-block.msg-hidden .bubble-user,
.mjs-chat .msg-block.msg-hidden .bubble-assistant,
.mjs-chat .msg-block.msg-hidden .bubble-error,
.mjs-chat .msg-block.msg-hidden .bubble-reasoning,
.mjs-chat .msg-block.msg-hidden .tool-card {
  text-decoration: line-through;
  text-decoration-color: var(--mjs-dim);
}
.mjs-chat .bubble-user {
  width: 100%;
  background: var(--mjs-user);
  border: 1px solid var(--mjs-user-border);
  border-radius: 14px 14px 4px 14px;
  padding: 10px 14px;
  white-space: pre-wrap;
  line-height: 1.45;
  font-size: 14px;
}
.mjs-chat.mode-sidebar .bubble-user { padding: 8px 12px; font-size: 13px; }
.mjs-chat .bubble-assistant {
  width: 100%;
  padding: 4px 2px;
  line-height: 1.55;
  font-size: 14px;
}
.mjs-chat.mode-sidebar .bubble-assistant { font-size: 13px; }
.mjs-chat .bubble-assistant.is-streaming { white-space: pre-wrap; word-break: break-word; }
.mjs-chat .bubble-error {
  width: 100%;
  padding: 4px 2px;
  line-height: 1.55;
  font-size: 14px;
  color: var(--mjs-err);
  white-space: pre-wrap;
  word-break: break-word;
}
.mjs-chat.mode-sidebar .bubble-error { font-size: 13px; }

.session-panel-demo { width: min(640px, 100%); height: 640px; }
.session-panel-demo .mjs-chat.mode-sidebar { border-left: 0; }
.session-panel-demo .mjs-chat { height: 100%; }
"#;

pub fn panel(st: &str) -> Node {
    m("div")
        .class("mjs-chat mode-sidebar")
        .attr("aria-label", "Session panel")
        .child(
            m("section").class("main").kids(vec![
                roster::roster(st),
                m("div")
                    .show(&format!("!{st}.treeOpen && !{st}.compacted"))
                    .child(session::messages(st)),
                m("div")
                    .show(&format!("!{st}.treeOpen && {st}.compacted"))
                    .child(session::messages_threaded(st)),
                m("div").show(&format!("{st}.treeOpen")).class("history-tree-wrap").child(history_tree::tree(st)),
                composer::panel(st),
            ]),
        )
}

pub fn session_panel() -> Node {
    panel(LIVE_STORE)
}

pub fn demo() -> Node {
    m("div").class("session-panel-demo").child(panel(crate::components::samples::EXPR))
}

pub fn register() {
    css::inject("session-panel", CSS);
}

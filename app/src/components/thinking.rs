use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
.mjs-chat .thinking {
  align-self: flex-start;
  color: var(--mjs-dim);
  font-size: 13px;
  padding: 4px 0;
}
.mjs-chat .thinking span { animation: mjs-chat-blink 1.2s infinite; }
.mjs-chat .thinking span:nth-child(2) { animation-delay: 0.2s; }
.mjs-chat .thinking span:nth-child(3) { animation-delay: 0.4s; }
@keyframes mjs-chat-blink { 50% { opacity: 0.2; } }

.thinking-demo { display: flex; }
.thinking-demo .mjs-chat.mode-sidebar { border-left: 0; }
"#;

pub fn thinking() -> Node {
    m("div").class("thinking").kids(vec![
        lit("Thinking "),
        m("span").child(lit("•")),
        m("span").child(lit("•")),
        m("span").child(lit("•")),
    ])
}

pub fn demo() -> Node {
    m("div").class("thinking-demo").child(
        m("div")
            .class("mjs-chat mode-sidebar")
            .child(thinking()),
    )
}

pub fn register() {
    css::inject("thinking", CSS);
}

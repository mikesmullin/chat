use rustex1_kit::h::{lit, m, Node};

use crate::components::picker::{picker, Picker};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .browser-pane {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel);
}
:is(#sb, #home) .bp-head {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
:is(#sb, #home) .bp-head .ph { font-size: 16px; color: var(--muted); flex: 0 0 auto; }
:is(#sb, #home) .bp-title {
  font-family: var(--sans);
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.02em;
  color: var(--fg);
  flex: 0 0 auto;
}
:is(#sb, #home) .bp-id {
  font-family: var(--mono);
  font-size: 11px;
  color: var(--gray-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1 1 auto;
  min-width: 0;
}
:is(#sb, #home) .bp-head .picker { flex: 0 0 auto; }
:is(#sb, #home) .bp-head .picker .dropdown-btn {
  background: var(--panel-2);
  border: 1px solid transparent;
  border-radius: 8px;
  padding: 4px 8px;
  font-size: 12px;
}
:is(#sb, #home) .bp-head .picker .picker-menu { left: auto; right: 0; }

:is(#sb, #home) .bp-stage {
  position: relative;
  width: 100%;
  aspect-ratio: 16 / 10;
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
  background: #0b0b0d;
}
:is(#sb, #home) .bp-frame {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  border: 0;
  display: block;
}
:is(#sb, #home) .bp-empty {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  border: 1px dashed var(--border);
  border-radius: 8px;
  color: var(--gray-dim);
}
:is(#sb, #home) .bp-empty .ph { font-size: 26px; }
:is(#sb, #home) .bp-empty p {
  margin: 0;
  font-family: var(--sans);
  font-size: 12px;
  text-align: center;
  padding: 0 14px;
}

.browser-pane-demo { width: 340px; }
"#;

fn chooser(st: &str) -> Node {
    picker(&Picker {
        key: "browserSandbox",
        icon: "monitor",
        extra: "dd-browsersandbox",
        search: "Search sandboxes…",
        items: &format!("{st}.browserSandboxes"),
        value: &format!("{st}.browserSandboxId"),
        null: None,
        null_icon: "monitor",
        on_pick: &format!("{st}.pickBrowserSandbox({{}})"),
        on_edit: "",
        actions: &[],
    })
}

pub fn panel(st: &str) -> Node {
    m("div")
        .class("browser-pane")
        .iff(&format!("{st}.browserSandboxes.length > 0"))
        .kids(vec![
            m("div").class("bp-head").kids(vec![
                Node::icon("monitor"),
                m("span").class("bp-title").child(lit("Sandbox desktop")),
                m("span")
                    .class("bp-id")
                    .text(&format!("{st}.browserSandboxId")),
                chooser(st).iff(&format!("{st}.browserSandboxes.length > 1")),
            ]),
            m("div")
                .class("bp-stage")
                .init(&format!("{st}.loadViewerUrl()"))
                .kids(vec![
                    m("iframe")
                        .class("bp-frame")
                        .attr("title", "sandbox desktop")
                        .attr("allow", "autoplay")
                        .bind("src", &format!("{st}.browserViewerUrl"))
                        .iff(&format!("{st}.browserViewerUrl !== ''")),
                    m("div").class("bp-empty").kids(vec![
                        Node::icon("desktop"),
                        m("p").child(lit("connecting to the sandbox desktop…")),
                    ]).iff(&format!("{st}.browserViewerUrl === ''")),
                ]),
        ])
}

pub fn demo() -> Node {
    m("div")
        .class("browser-pane-demo")
        .child(panel(crate::components::samples::EXPR))
        .child(panel_empty())
}

pub fn panel_empty() -> Node {
    m("div")
        .class("browser-pane browser-pane-demo")
        .kids(vec![
            m("div").class("bp-head").kids(vec![
                Node::icon("monitor"),
                m("span").class("bp-title").child(lit("Sandbox desktop")),
            ]),
            m("div").class("bp-stage").kids(vec![m("div").class("bp-empty").kids(vec![
                Node::icon("desktop"),
                m("p").child(lit("the desktop appears here when the session's sandbox is ready")),
            ])]),
        ])
}

pub fn register() {
    css::inject("browser-pane", CSS);
}

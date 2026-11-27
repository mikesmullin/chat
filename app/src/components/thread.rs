use rustex1_kit::h::{lit, m, Node};

use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .thread-row {
  display: flex; gap: 8px; align-items: flex-start;
  padding-left: calc(var(--d, 0) * 20px);
  margin-bottom: 6px;
}
:is(#sb, #home) .thread-card {
  flex: 1; min-width: 0;
  display: flex; gap: 10px; align-items: flex-start;
  padding: 8px 12px;
  border-radius: 10px;
  background: var(--panel-2);
  border-left: 2px solid var(--gray-dim);
  cursor: pointer;
  text-align: left;
  color: inherit;
  font: inherit;
  width: 100%;
}
:is(#sb, #home) .thread-card:hover { border-left-color: var(--accent); }
:is(#sb, #home) .thread-card.is-open { border-left-color: var(--accent); }
:is(#sb, #home) .thread-caret {
  flex: 0 0 auto; line-height: 18px; color: var(--gray-dim);
  transition: transform 120ms ease;
}
:is(#sb, #home) .thread-card.is-open .thread-caret { transform: rotate(90deg); }
:is(#sb, #home) .thread-body { flex: 1; min-width: 0; }
:is(#sb, #home) .thread-meta {
  display: flex; gap: 8px; align-items: center;
  font-size: 11px; color: var(--gray-dim); margin-bottom: 2px;
}
:is(#sb, #home) .thread-count { font-variant-numeric: tabular-nums; }
:is(#sb, #home) .thread-ref { opacity: .6; font-family: var(--mono, monospace); }
:is(#sb, #home) .thread-clipped { color: var(--warn, #c9a227); }
:is(#sb, #home) .thread-text { font-size: 13px; line-height: 1.5; }
:is(#sb, #home) .thread-text.unbuilt { color: var(--gray-dim); font-style: italic; }
:is(#sb, #home) .thread-spinner { font-size: 11px; color: var(--gray-dim); }

"#;

pub fn row(st: &str) -> Node {
    m("button")
        .bind("class", "('thread-card ' + (t.expanded ? 'is-open' : ''))")
        .on("click", &format!("{st}.toggleThread(t.id)"))
        .kids(vec![
            Node::icon("caret-right").class("ph ph-caret-right thread-caret"),
            m("div").class("thread-body").kids(vec![
                m("div").class("thread-meta").kids(vec![
                    m("span").class("thread-count").text("t.label"),
                    m("span").class("thread-ref").text("t.ref"),
                    m("span")
                        .class("thread-clipped")
                        .iff("t.clipped")
                        .child(lit("truncated")),
                    m("span")
                        .class("thread-spinner")
                        .iff("t.loading")
                        .child(lit("opening…")),
                ]),
                m("div")
                    .bind("class", "('thread-text md-body ' + (t.built ? '' : 'unbuilt'))")
                    .inner_html("t.html || t.text"),
            ]),
        ])
}

pub fn register() {
    css::inject("thread", CSS);
}

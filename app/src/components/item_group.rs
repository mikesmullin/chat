use rustex1_kit::h::{lit, m, Node};

pub const CSS: &str = r#"
.item-group { color: var(--muted); margin-bottom: 18px; max-width: 480px; width: 100%; }
.item-group.idle { color: var(--gray-dim); }
.item-group.working { color: var(--pastel-amber); }
.item-group.done { color: var(--accent); }
.item-group:last-child { margin-bottom: 0; }

.item-head {
  display: flex; align-items: center; gap: 6px; margin-bottom: 6px;
  font-size: 12px;
}
.item-head-name { font-weight: 600; }
.item-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex; flex-direction: column; gap: 2px;
}

.item-row {
  display: flex; align-items: center; gap: 10px;
  padding: 6px 0; border-radius: 8px; color: var(--text);
  transition: background .13s;
}
.item-row:hover { background: var(--accent-glow); }
.item-icon { font-size: 16px; line-height: 1; color: var(--muted); flex: 0 0 auto; transition: color .13s; }
.item-row:hover .item-icon { color: var(--accent-soft); }
.item-name { font-weight: 600; flex: 0 0 auto; }
.item-summary {
  flex: 1 1 auto; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  color: var(--muted); font-size: 13px; padding-right: 4px;
}
.item-meta { display: flex; align-items: center; gap: 10px; flex: 0 0 auto; }
"#;

fn row(name: &str, summary: &str, reltime: &str) -> Node {
    m("li")
        .class("item-row")
        .kids(vec![
            Node::icon("robot"),
            m("span").class("item-name").child(lit(name)),
            m("span").class("item-summary").child(lit(summary)),
            m("span").class("item-meta").kids(vec![
                m("span").class("relative-timestamp").child(lit(reltime)),
            ]),
        ])
}

fn head(label: &str, count: &str) -> Node {
    m("div")
        .class("item-head")
        .kids(vec![
            m("span").class("item-head-name").child(lit(label)),
            Node::host("uiCounter", &[]).bind("value", count),
        ])
}

pub fn item_group(tone: &str, label: &str, count: &str, rows: &[(String, String, String)]) -> Node {
    m("div")
        .class(&format!("item-group {}", tone))
        .kids(vec![
            head(label, count),
            m("ul")
                .class("item-list")
                .kids(rows.iter().map(|r| row(&r.0, &r.1, &r.2)).collect()),
        ])
}

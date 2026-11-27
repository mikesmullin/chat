use rustex1_kit::h::{m, Node};

use crate::components::presence;
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .name-tag {
  display: inline-flex; align-items: center; gap: 6px;
  font-size: 11px; line-height: 18px;
  color: var(--agent-accent, #a78bfa);
}
:is(#sb, #home) .name-tag-who { display: inline-flex; align-items: center; gap: 5px; }
:is(#sb, #home) .name-tag i { font-size: 13px; line-height: 18px; }
:is(#sb, #home) .name-tag-when { color: var(--gray-dim); }
:is(#sb, #home) button.name-tag {
  background: transparent; border: 0; padding: 0;
  font-family: inherit; text-align: left; cursor: pointer;
}
:is(#sb, #home) button.name-tag:focus-visible {
  outline: 2px solid var(--accent); outline-offset: 2px; border-radius: 4px;
}
"#;

fn who(icon: &str, name: &str) -> Node {
    m("span").class("name-tag-who").kids(vec![
        Node::icon_dyn(icon),
        m("span").text(name),
    ])
}

pub fn who_link(icon: &str, accent: &str, name: &str, on: &str) -> Node {
    m("button")
        .class("name-tag")
        .attr("type", "button")
        .bind("style", &format!("('--agent-accent: ' + ({accent}))"))
        .on("click", on)
        .child(who(icon, name))
}

pub fn mention(st: &str, handle: &str) -> Node {
    let user = format!("({handle} === 'user' || ({st}.userName && {handle} === {st}.userName))");
    let icon = format!("({user} ? '{}' : {st}.iconOf({handle}))", presence::USER_ICON);
    m("span")
        .class("name-tag")
        .bind("style", &format!("('--agent-accent: ' + ({user} ? '{}' : {st}.accentOf({handle})))", presence::USER_ACCENT))
        .child(who(&icon, handle))
}

pub fn byline(st: &str, handle_expr: &str, ts_expr: &str, when: &str) -> Node {
    m("div")
        .class("name-tag")
        .iff(when)
        .kids(vec![
            who(&format!("{st}.iconOf({handle_expr} || '')"), handle_expr),
            m("span")
                .class("name-tag-when")
                .show(ts_expr)
                .text(&format!("'· ' + {st}.relAgo({ts_expr})")),
        ])
}

pub fn user_byline_as(
    st: &str,
    name_expr: &str,
    ts_expr: &str,
    when: &str,
    action: Node,
) -> Node {
    m("div")
        .class("name-tag name-tag-user")
        .attr("style", &format!("--agent-accent: {}", presence::USER_ACCENT))
        .iff(when)
        .kids(vec![
            who(&format!("'{}'", presence::USER_ICON), name_expr),
            m("span")
                .class("name-tag-when")
                .show(ts_expr)
                .text(&format!("'· ' + {st}.relAgo({ts_expr})")),
            action,
        ])
}

pub fn register() {
    css::inject("name-tag", CSS);
}

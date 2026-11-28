use rustex1_kit::h::{m, Node};

use crate::components::{notification_toggle, presence, session_state};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .roster {
  display: flex; gap: 6px; align-items: center; flex-wrap: wrap;
  padding: 6px 10px; font-size: 11px;
}
:is(#sb, #home) .roster .presence-seat:nth-child(n+7) { display: none; }
:is(#sb, #home) .roster .browser-toggle {
  width: 26px; height: 26px; padding: 0; border: 0; border-radius: 6px;
  display: inline-flex; align-items: center; justify-content: center;
  background: transparent; color: var(--muted); cursor: pointer;
  margin-left: auto;
}
:is(#sb, #home) .roster .browser-toggle:hover { color: var(--text); background: var(--panel-2); }
:is(#sb, #home) .roster .browser-toggle.is-on { color: var(--pastel-cyan); }
:is(#sb, #home) .roster .browser-toggle:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
:is(#sb, #home) .roster .browser-toggle .ph { font-size: 16px; }
"#;

pub fn roster(st: &str) -> Node {
    m("div")
        .class("roster")
        .show(&format!("{st}.activeId"))
        .child(
            m("button")
                .class("browser-toggle")
                .attr("type", "button")
                .attr("aria-label", "Toggle sandbox desktop")
                .bind("class", &format!("($store.home.browserOpen ? ' is-on' : '')"))
                .bind("aria-pressed", &format!("($store.home.browserOpen ? 'true' : 'false')"))
                .bind("title", &format!("($store.home.browserOpen ? 'Hide sandbox desktop' : 'Show sandbox desktop')"))
                .on("click", "$store.home.toggleBrowser()")
                .child(Node::icon("desktop"))
                .show("$store.live.browserSandboxes.length > 0")
        )
        .child(notification_toggle::button(st))
        .child(
            presence::row(
                presence::seats(vec![
                    presence::seat(
                        &presence::user_name(st),
                        &format!("'{}'", presence::USER_ICON),
                        &format!("'{}'", presence::USER_ACCENT),
                        "false",
                        &format!("({st}.activeOthers.length + 100)"),
                    ),
                    presence::seat(
                        "p.handle",
                        &format!("{st}.iconOf(p.handle)"),
                        &format!("{st}.accentOf(p.handle)"),
                        "!p.active",
                        "(100 - i)",
                    )
                    .for_each(&format!("(p, i) in {st}.activeOthers"))
                    .key("p.id"),
                    presence::more(&format!("{st}.activeOthers.length - 5")),
                ]),
            ),
        )
        .child(
            Node::host("uiPieRing", &[("color", "var(--accent)")])
                .bind(
                    "fraction",
                    &session_state::ctx_fraction(
                        &format!("{st}.totalTokens"),
                        &format!("{st}.session.contextWindow"),
                    ),
                )
                .bind(
                    "title",
                    &session_state::ctx_hint(
                        &format!("{st}.totalTokens"),
                        &format!("{st}.session.contextWindow"),
                    ),
                ),
        )
}

pub fn register() {
    css::inject("roster", CSS);
}

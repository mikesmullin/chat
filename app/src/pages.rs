//! The app's routes.
//!
//! `/` is the app shell (`home`); `/storybook` and its pages are the design
//! guide (`storybook`). There is nothing else: the routes are thin, because
//! every page is composed from `components/*`, and a page that drew its own
//! markup would be a second definition of whatever it drew.
//!
//! There used to be a third route here, `/chat-demo` — the original
//! `m-js-cmp-chat` demo grid, ported markup-for-markup. It was the last place
//! that drew a transcript, a composer and a history rail by hand, and it is
//! gone: those are components now, and the design guide shows them against
//! sample data without a second copy of the markup.

use crate::css;
use rustex1_kit::h::{lit, m};
use rustex1_kit::{mjs, Runtime};
use rustex1_kit::mjs::Def;

pub mod home;
pub mod storybook;

pub fn register(rt: &mut Runtime) {
    mjs::router_set_mode("path");
    // the home shell owns a CSS block and the `home` store (aside collapse)
    home::register();
    // `/` is the three-column app shell, composed from the storybook
    // components.
    rt.route("/", "Chat", |_params| {
        let mut d = Def::new().named("route /");
        d.tree(home::shell());
        d.finish()
    });

    storybook::register(rt);

    // /test — a single small Rust + m-js hello world for diagnosing the Fire
    // crashes: no components, no stores, no fetches. Just the framework, the
    // wasm instance and Chromium. If this crashes on the tablet, the stack
    // itself is the problem; if it holds, the heavy app's data/rendering is.
    css::inject(
        "test",
        r#"#test {
  display: grid;
  place-items: center;
  min-height: 100vh;
  min-height: 100dvh;
  text-align: center;
  padding: 2rem;
}
#test h1 {
  font-size: 2.4rem;
  line-height: 1.2;
}
#test p {
  color: var(--muted);
  max-width: 34ch;
  line-height: 1.55;
  margin: 0 auto;
}
"#,
    );
    rt.route("/test", "Test", |_| {
        let mut d = Def::new().named("route /test");
        d.tree(
            m("div")
                .attr("id", "test")
                .class("test-page")
                .kids(vec![
                    m("h1").child(lit("Hello, World")),
                    m("p").child(lit(
                        "One tiny component. No stores, no fetches, no widgets. If this crashes on the Fire, it is the Rust + m-js + Chromium stack, not the heavy app.",
                    )),
                ]),
        );
        d.finish()
    });
}

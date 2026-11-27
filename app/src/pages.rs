use crate::css;
use rustex1_kit::h::{lit, m};
use rustex1_kit::{mjs, Runtime};
use rustex1_kit::mjs::Def;

pub mod home;
pub mod storybook;

pub fn register(rt: &mut Runtime) {
    mjs::router_set_mode("path");
    home::register();
    rt.route("/", "Chat", |_params| {
        let mut d = Def::new().named("route /");
        d.tree(home::shell());
        d.finish()
    });

    storybook::register(rt);

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

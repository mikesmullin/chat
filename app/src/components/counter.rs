use rustex1_kit::h::m;
use rustex1_kit::mjs::{self, Def};
use crate::css;

const NAME: &str = "uiCounter";

const CSS: &str = r#"
.counter-demo { display: flex; gap: 6px; align-items: baseline; }
"#;

fn def() -> Def {
    let mut d = Def::new().named(NAME);
    d.prop_num("value", 0.0)
        .tree(m("span").class("counter").text("value"));
    d
}

pub fn register() {
    css::inject(NAME, CSS);
    mjs::component(NAME, &def().finish());
}

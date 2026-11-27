use rustex1_kit::mjs::Def;
use rustex1_kit::mjs;

pub const MODEL: &str = "$store.demo";

pub fn register() {
    let mut d = Def::new().named("store demo");
    d.str("text1", "")
        .str("text2", "")
        .str("toggle", "false")
        .str("range", "")
        .str("radio", "")
        .str("area", "")
        .str("tags", "")
        .str("field", "");
    mjs::store("demo", &d.finish());
}

use rustex1_kit::h::{m, Node};
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

pub const NAME: &str = "uiPrAvatar";

pub const CSS: &str = r#"
.pr-av {
  display: inline-flex; align-items: center; justify-content: center;
  width: var(--s, 20px); height: var(--s, 20px);
  border-radius: 50%; flex: 0 0 auto;
  font-size: calc(var(--s, 20px) * .42); font-weight: 700; color: #fff;
  background: linear-gradient(135deg, var(--c1, var(--accent)), var(--c2, var(--accent)));
  overflow: hidden; user-select: none;
}
.pr-av.av-bot { background: var(--panel-2); color: var(--gray); border: 1px solid var(--border); }
.pr-av .ph { font-size: calc(var(--s, 20px) * .55); }
.pr-av-demo { display: flex; gap: 10px; align-items: center; }
"#;

pub fn mount(name: &str, size: &str) -> Node {
    Node::host(NAME, &[("name", name), ("size", size)])
}

pub fn demo() -> Node {
    m("div").class("pr-av-demo").kids(vec![
        mount("Bob", "20"),
        mount("Alice", "28"),
        mount("ajcs-bot", "20"),
    ])
}

pub fn register() {
    css::inject(NAME, CSS);
    let mut d = Def::new().named(NAME);
    d.prop_str("name", "")
        .prop_str("size", "20")
        .tree(
            m("span")
                .bind("style", "('--s:' + (size || 20) + 'px;--c1:hsl(' + (((name || '').split('').reduce((a, c) => (a * 31 + c.charCodeAt(0)) >>> 0, 7) % 360)) + ' 62% 42%);--c2:hsl(' + (((name || '').split('').reduce((a, c) => (a * 31 + c.charCodeAt(0)) >>> 0, 7) % 360) + 40) + ' 62% 32%)')")
                .bind("class", "((name === 'ajcs-bot' || /^agent:/.test(name || '') || /-bot$/.test(name || '')) ? 'pr-av av-bot' : 'pr-av')")
                .bind("title", "name")
                .kids(vec![
                    m("i")
                        .class("ph ph-robot")
                        .show("(name === 'ajcs-bot' || /^agent:/.test(name || '') || /-bot$/.test(name || ''))"),
                    m("span")
                        .text("((name || '').replace(/^agent:/, '').split(/[:/_\\-.]+/).filter(Boolean)[0] || (name || '?')).slice(0, 2).toUpperCase()")
                        .show("!(name === 'ajcs-bot' || /^agent:/.test(name || '') || /-bot$/.test(name || ''))"),
                ]),
        );
    mjs::component(NAME, &d.finish());
}

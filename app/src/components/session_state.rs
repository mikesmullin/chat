use rustex1_kit::h::m;
use rustex1_kit::mjs;
use rustex1_kit::mjs::Def;

use crate::css;

pub const NAME: &str = "uiSessionState";

const CSS: &str = r#"
.state-demo { display: flex; align-items: center; gap: 36px; }
.ui-state-root {
  position: relative; display: inline-flex; align-items: flex-end; justify-content: center;
  width: 14px; height: 14px; flex: 0 0 auto;
}
.ui-state-root::after {
  content: ""; position: absolute; bottom: 1px; left: 2px; right: 2px; height: 1.5px;
  border-radius: 1px; background: currentColor; opacity: .32;
}
.ui-state-root.working::after { content: none; }
.ui-state-root.done { color: var(--accent); }
.ui-state-root.working { color: var(--pastel-amber); }
.ui-state-root.idle { color: var(--gray-dim); }
.state-ball {
  position: relative; z-index: 1;
  width: 9px; height: 9px; border-radius: 50%;
  background: currentColor;
  box-shadow: 0 1px 2px rgba(0, 0, 0, .45);
  margin-bottom: 2px;
}
.ui-state-root.working .state-ball {
  animation: state-bounce 0.755s ease-in-out infinite;
}
@keyframes state-bounce {
  0%, 100% { transform: translateY(0) scale(1.18, 0.82); }
  42%      { transform: translateY(-6px) scale(0.9, 1.1); }
}
@media (prefers-reduced-motion: reduce) {
  .ui-state-root.working .state-ball { animation: none; }
}
"#;

pub fn ctx_fraction(tokens: &str, window: &str) -> String {
    format!("Math.min(1, ({tokens} || 0) / Math.max(1, ({window} || 128000)))")
}

pub fn ctx_hint(tokens: &str, window: &str) -> String {
    format!(
        "'Context ' + {used} + ' of ' + {cap}",
        used = token_hint(&format!("({tokens} || 0)")),
        cap = token_hint(&format!("({window} || 128000)")),
    )
}

fn token_hint(value: &str) -> String {
    format!(
        "(({value}) >= 1e6 ? ((({value}) / 1e6).toFixed(1).replace('.0', '') + 'M') \
         : ({value}) >= 1e3 ? ((({value}) / 1e3).toFixed(1).replace('.0', '') + 'K') \
         : ('' + ({value})))"
    )
}

pub fn tone_of(row: &str) -> String {
    format!(
        "({row}.status === 'running' ? 'working' : {row}.turns === 0 ? 'idle' : 'done')"
    )
}

pub fn register() {
    css::inject(NAME, CSS);
    mjs::component(NAME, &def().finish());
}

fn def() -> Def {
    let mut d = Def::new().named(NAME);
    d.prop_str("tone", "done")
        .prop_str("title", "")
        .tree(
            m("span")
                .bind("class", "(tone === 'working' ? 'ui-state-root working' : tone === 'idle' ? 'ui-state-root idle' : 'ui-state-root done')")
                .bind("title", "title")
                .child(m("span").class("state-ball")),
        );
    d
}

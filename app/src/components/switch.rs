use rustex1_kit::h::{m, Node};

use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .sw {
  appearance: none; border: 0; background: transparent; cursor: pointer;
  display: inline-flex; align-items: center; gap: 8px; padding: 0;
  align-self: flex-start;
}
:is(#sb, #home) .sw-track {
  position: relative; width: 32px; height: 17px;
  border-radius: 999px; background: #00ad0d;
  transition: background 140ms ease;
}
:is(#sb, #home) .sw-knob {
  position: absolute; top: 50%; margin-top: -8px;
  left: 15px;
  width: 16px; height: 16px; border-radius: 50%; background: #fff;
  box-shadow: 0 1px 2px rgba(0,0,0,.3);
  display: inline-flex; align-items: center; justify-content: center;
  transition: left 160ms cubic-bezier(.2,.8,.2,1);
}
:is(#sb, #home) .sw-icon {
  font-size: 10px; color: #00ad0d;
}
:is(#sb, #home) .sw-label {
  display: inline-flex; align-items: center; gap: 5px;
  font: 500 12px/1 var(--sans); color: var(--text);
}
:is(#sb, #home) .sw-glyph { color: #fff; font-size: 13px; line-height: 1; }
:is(#sb, #home) .sw.is-off .sw-track { background: #ef4444; }
:is(#sb, #home) .sw.is-off .sw-knob { left: 1px; }
:is(#sb, #home) .sw.is-off .sw-icon { color: #b91c1c; }
:is(#sb, #home) .sw:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; border-radius: 999px; }
.sw-demo { display: flex; flex-direction: column; gap: 12px; }
"#;

pub fn switch(checked: &str, on: &str, label: &str, glyph: Option<Node>) -> Node {
    let mut kids = Vec::new();
    if let Some(g) = glyph { kids.push(g); }
    kids.push(m("span").text(label));
    m("button")
        .attr("type", "button")
        .bind(
            "class",
            &format!("({checked} ? 'sw' : 'sw is-off')"),
        )
        .on("click", on)
        .kids(vec![
            m("span").class("sw-track").child(
                m("span").class("sw-knob").child(m("i").bind(
                    "class",
                    &format!("({checked} ? 'ph ph-fill ph-check-fat sw-icon' : 'ph ph-bold ph-x sw-icon')"),
                )),
            ),
            m("span").class("sw-label").kids(kids),
        ])
}

pub fn demo() -> Node {
    let st = "$store.demo";
    m("div").class("sw-demo").kids(vec![
        switch(&format!("{st}.switchOn"), &format!("{st}.toggleBool('switchOn')"), "'Automation active'", None),
        switch(&format!("{st}.switchOff"), &format!("{st}.toggleBool('switchOff')"), "'Catch up missed runs at startup'", None),
    ])
}

pub fn register() {
    css::inject("switch", CSS);
}

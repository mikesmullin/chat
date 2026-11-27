use rustex1_kit::h::m;
use rustex1_kit::mjs::{self, Def};
use crate::css;

const NAME: &str = "uiPieRing";

const CSS: &str = r#"
.demo-ring { width: 64px; height: 64px; color: var(--accent); }
.demo-ring svg, .demo-ring .pie-ring svg { width: 100%; height: 100%; display: block; }
@keyframes ringfill {
  0%   { stroke-dasharray: 0 1; }
  50%  { stroke-dasharray: 1 0; }
  100% { stroke-dasharray: 0 1; }
}
.demo-ring circle:last-of-type {
  stroke-dasharray: 1 0;
  animation: ringfill 3s ease-in-out infinite;
}
"#;

fn def() -> Def {
    let mut d = Def::new().named(NAME);
    d.prop_num("fraction", 0.68)
        .prop_str("color", "")
        .tree(
            m("svg")
                .class("pie-ring")
                .attr("viewBox", "0 0 16 16")
                .attr("aria-hidden", "true")
                .bind("style", "color ? 'color:' + color : ''")
                .kids(vec![
                    m("circle")
                        .attr("cx", "8")
                        .attr("cy", "8")
                        .attr("r", "6")
                        .attr("fill", "none")
                        .attr("stroke", "currentColor")
                        .attr("stroke-opacity", ".22")
                        .attr("stroke-width", "2")
                        .attr("pathLength", "1"),
                    m("circle")
                        .attr("cx", "8")
                        .attr("cy", "8")
                        .attr("r", "6")
                        .attr("fill", "none")
                        .attr("stroke", "currentColor")
                        .attr("stroke-width", "2")
                        .attr("pathLength", "1")
                        .attr("stroke-linecap", "round")
                        .attr("transform", "rotate(-90 8 8)")
                        .bind("stroke-dasharray", "fraction + ' 1'"),
                ]),
        );
    d
}

pub fn register() {
    css::inject(NAME, CSS);
    mjs::component(NAME, &def().finish());
}

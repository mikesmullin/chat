//! `uiPieRing` — a generic cooldown/progress ring (the storybook's
//! `/pie-ring-graph` component).
//!
//! Two SVG circles on a `pathLength="1"` normalisation: a faint full-circle
//! track and a fill arc whose length is the `fraction` prop. Colour is
//! `currentColor`, so it tints to whatever state it sits in (or the host's
//! `color` prop).
//!
//! props (host-provided, with defaults):
//! - `fraction` — fill fraction 0..1 (default 0.68)
//! - `color`    — explicit CSS colour; empty => inherit (currentColor) (default "")
//!
//! The shared `.pie-ring` base (14px inline) lives in the global layer — the
//! same ring is used as the "working" symbol inside item rows. The showcase
//! chrome (`.demo-ring`, the looping pulse) is owned here.

use rustex1_kit::h::m;
use rustex1_kit::mjs::{self, Def};
use crate::css;

const NAME: &str = "uiPieRing";

const CSS: &str = r#"
/* standalone ring showcase: 64px, accent, loops 0% → 100% → 0%
   (fill circle only; the static track is the first circle) */
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
                    // track: faint, full circumference.
                    m("circle")
                        .attr("cx", "8")
                        .attr("cy", "8")
                        .attr("r", "6")
                        .attr("fill", "none")
                        .attr("stroke", "currentColor")
                        .attr("stroke-opacity", ".22")
                        .attr("stroke-width", "2")
                        .attr("pathLength", "1"),
                    // fill arc: dasharray = fraction (pathLength normalised to 1).
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

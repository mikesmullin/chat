use rustex1_kit::h::{m, Node};

fn at_expr(offset_ms: i64) -> String {
    format!(
        "(window.__rtEpoch===undefined)&&(window.__rtEpoch=Date.now()),new Date(window.__rtEpoch-{}).toISOString()",
        offset_ms
    )
}

const MIN: i64 = 60 * 1000;

pub fn demo() -> Node {
    m("div")
        .class("relative-timestamp demo-set")
        .kids(vec![
            Node::host("uiRelTime", &[]).bind("at", &at_expr(3 * MIN)),
            Node::host("uiRelTime", &[]).bind("at", &at_expr(20 * MIN)),
            Node::host("uiRelTime", &[]).bind("at", &at_expr(45 * MIN)),
            Node::host("uiRelTime", &[]).bind("at", &at_expr(60 * MIN)),
            Node::host("uiRelTime", &[]).bind("at", &at_expr(0)),
        ])
}

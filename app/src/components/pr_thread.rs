use rustex1_kit::h::{m, Node};

use crate::components::{pr_avatar, samples};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .pr-thread { display: flex; flex-direction: column; gap: 12px; }
:is(#sb, #home) .prt-item { display: flex; gap: 8px; align-items: flex-start; }
:is(#sb, #home) .prt-cc {
  flex: 1 1 auto; min-width: 0;
  border: 1px solid var(--border); border-radius: 10px; background: var(--panel-2);
  padding: 7px 10px;
}
:is(#sb, #home) .prt-cc.cc-ok { border-color: color-mix(in srgb, var(--pastel-green, #22c55e) 45%, transparent); }
:is(#sb, #home) .prt-cc.cc-warn { border-color: color-mix(in srgb, var(--mjs-yellow) 45%, transparent); }
:is(#sb, #home) .prt-cc.cc-danger { border-color: color-mix(in srgb, var(--danger, #d9534f) 45%, transparent); }
:is(#sb, #home) .prt-head {
  display: flex; align-items: center; gap: 6px; flex-wrap: wrap;
  font-size: 11px; color: var(--gray-dim);
}
:is(#sb, #home) .prt-head b { color: var(--fg); font-size: 12px; }
:is(#sb, #home) .prt-body { margin-top: 5px; font-size: 12px; color: var(--fg); }
:is(#sb, #home) .prt-thin {
  display: flex; align-items: center; gap: 7px; flex-wrap: wrap;
  font-size: 11.5px; color: var(--gray-dim);
}
:is(#sb, #home) .prt-thin b { color: var(--fg); }
:is(#sb, #home) .prt-event {
  display: flex; align-items: center; gap: 8px;
  font-size: 11.5px; color: var(--gray-dim);
}
:is(#sb, #home) .prt-event .prt-ico {
  display: inline-flex; align-items: center; justify-content: center;
  width: 22px; height: 22px; border-radius: 50%;
  background: color-mix(in srgb, var(--pastel-purple, #a78bfa) 20%, transparent);
  color: var(--pastel-purple, #a78bfa);
}
:is(#sb, #home) .prt-event code {
  background: var(--panel-2); border: 1px solid var(--border); border-radius: 5px; padding: 0 5px;
}
:is(#sb, #home) .prt-verb-ok { color: var(--pastel-green, #22c55e); }
:is(#sb, #home) .prt-verb-warn { color: var(--mjs-yellow); }
:is(#sb, #home) .prt-verb-danger { color: var(--danger, #d9534f); }
.pr-thread-demo { width: min(560px, 100%); }
"#;

const DEC_ICON: &str = "({comment:'ph-chat-circle-text',approve:'ph-seal-check',request_changes:'ph-pencil-simple',reject:'ph-prohibit'}[rv.decision] || 'ph-chat-circle-text')";
const DEC_VERB: &str = "({comment:'commented',approve:'approved these changes',request_changes:'requested changes',reject:'rejected these changes'}[rv.decision] || rv.decision)";
const DEC_TONE: &str = "({comment:'',approve:'ok',request_changes:'warn',reject:'danger'}[rv.decision] || '')";

pub fn parts(st: &str, v: &str) -> Vec<Node> {
    vec![m("div").class("pr-thread").kids(items(st, v))]
}

fn items(st: &str, v: &str) -> Vec<Node> {
    vec![
        m("div")
            .class("prt-item")
            .show(&format!("{v}.description"))
            .kids(vec![
                Node::host(pr_avatar::NAME, &[]).bind("name", &format!("{v}.author")),
                m("div").class("prt-cc").kids(vec![
                    m("div").class("prt-head").kids(vec![
                        m("b").text(&format!("{v}.author || 'someone'")),
                        m("span").child(lit_keep("opened this pull request")),
                        Node::host(crate::components::reltime::NAME, &[]).bind("at", &format!("{v}.createdAt")),
                    ]),
                    m("div").class("prt-body md-body").inner_html(&format!("{st}.descriptionHtml")),
                ]),
            ]),
        m("div")
            .class("prt-item")
            .for_each(&format!("rv in {v}.reviews"))
            .key("rv.id")
            .show(&format!("rv.comment"))
            .kids(vec![
                Node::host(pr_avatar::NAME, &[]).bind("name", "rv.reviewer"),
                m("div").bind("class", &format!("('prt-cc cc-' + {DEC_TONE})")).kids(vec![
                    m("div").class("prt-head").kids(vec![
                        m("i").class("ph").bind("class", &format!("'ph ' + {DEC_ICON}")),
                        m("b").text("rv.reviewer"),
                        m("span").bind("class", &format!("('prt-verb-' + {DEC_TONE})")).text(DEC_VERB),
                        Node::host(crate::components::reltime::NAME, &[]).bind("at", "rv.createdAt"),
                    ]),
                    m("div").class("prt-body md-body").text("rv.comment"),
                ]),
            ]),
        m("div")
            .class("prt-thin")
            .for_each(&format!("rv in {v}.reviews"))
            .key(&format!("'thin' + rv.id"))
            .show(&format!("!rv.comment"))
            .kids(vec![
                Node::host(pr_avatar::NAME, &[]).bind("name", "rv.reviewer"),
                m("b").text("rv.reviewer"),
                m("span").bind("class", &format!("('prt-verb-' + {DEC_TONE})")).text(DEC_VERB),
                Node::host(crate::components::reltime::NAME, &[]).bind("at", "rv.createdAt"),
            ]),
        m("div")
            .class("prt-event")
            .show(&format!("{v}.state === 'merged' && {v}.mergeCommit"))
            .kids(vec![
                m("span").class("prt-ico").child(Node::icon("git-merge")),
                m("div").kids(vec![
                    m("b").text(&format!("{v}.mergedBy")).show(&format!("{v}.mergedBy")),
                    m("span").child(lit_keep("merged commit")),
                    m("code").text(&format!("({v}.mergeCommit || '').slice(0, 10)")),
                    m("span").child(lit_keep("into")),
                    m("code").text(&format!("{v}.targetBranch")),
                ]),
                Node::host(crate::components::reltime::NAME, &[]).bind("at", &format!("{v}.updatedAt")),
            ]),    ]
}

fn lit_keep(text: &str) -> Node {
    rustex1_kit::h::lit(text)
}

pub fn demo() -> Node {
    let (st, v) = (samples::EXPR, samples::sample("pr"));
    m("div").class("pr-thread-demo").kids(parts(st, &v))
}

pub fn register() {
    css::inject("pr-thread", CSS);
}

use rustex1_kit::h::{lit, m, Node};

use crate::components::{job_run_chip, vcs};
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .merge-panel {
  border: 1px solid var(--border); border-radius: 12px;
  background: var(--panel-2); overflow: hidden;
}
:is(#sb, #home) .mp-band {
  display: flex; align-items: flex-start; gap: 8px;
  padding: 10px 12px; border-bottom: 1px solid var(--border);
}
:is(#sb, #home) .mp-band .mp-ico {
  display: inline-flex; align-items: center; justify-content: center;
  width: 24px; height: 24px; border-radius: 50%; flex: 0 0 auto;
}
:is(#sb, #home) .mp-band.mp-ready .mp-ico { background: color-mix(in srgb, var(--pastel-green, #22c55e) 20%, transparent); color: var(--pastel-green, #22c55e); }
:is(#sb, #home) .mp-band.mp-wait .mp-ico { background: color-mix(in srgb, var(--mjs-yellow) 18%, transparent); color: var(--mjs-yellow); }
:is(#sb, #home) .mp-band.mp-bad .mp-ico { background: color-mix(in srgb, var(--danger, #d9534f) 15%, transparent); color: var(--danger, #d9534f); }
:is(#sb, #home) .mp-band.mp-done .mp-ico { background: color-mix(in srgb, var(--pastel-purple, #a78bfa) 22%, transparent); color: var(--pastel-purple, #a78bfa); }
:is(#sb, #home) .mp-band-main { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
:is(#sb, #home) .mp-band-title { font-size: 12.5px; font-weight: 700; color: var(--fg); }
:is(#sb, #home) .mp-band-sub { font-size: 11px; color: var(--gray-dim); }
:is(#sb, #home) .mp-rows { display: flex; flex-direction: column; }
:is(#sb, #home) .mp-row {
  display: flex; align-items: center; gap: 7px;
  padding: 6px 12px; border-top: 1px solid var(--border);
  font-size: 11.5px; color: var(--gray);
}
:is(#sb, #home) .mp-row:first-child { border-top: 0; }
:is(#sb, #home) .mp-row .mp-row-ico { color: var(--gray-dim); flex: 0 0 auto; }
:is(#sb, #home) .mp-row.mp-row-ok .mp-row-ico { color: var(--pastel-green, #22c55e); }
:is(#sb, #home) .mp-row.mp-row-warn .mp-row-ico { color: var(--mjs-yellow); }
:is(#sb, #home) .mp-row.mp-row-bad .mp-row-ico { color: var(--danger, #d9534f); }
:is(#sb, #home) .mp-row code {
  background: var(--panel); border: 1px solid var(--border); border-radius: 5px;
  padding: 0 5px; font-size: 10.5px;
}
:is(#sb, #home) .mp-foot {
  display: flex; align-items: center; gap: 8px;
  padding: 10px 12px; border-top: 1px solid var(--border);
}
:is(#sb, #home) .mp-reason { font-size: 11px; color: var(--gray-dim); }
:is(#sb, #home) .mp-merge {
  margin-left: auto;
  display: inline-flex; align-items: center; gap: 6px;
  border: 0; border-radius: 8px; padding: 7px 15px;
  background: var(--pastel-green, #22c55e); color: #fff;
  font: inherit; font-size: 12.5px; font-weight: 700; cursor: pointer;
}
:is(#sb, #home) .mp-merge:disabled { opacity: .45; cursor: default; }
:is(#sb, #home) .mp-ghost {
  display: inline-flex; align-items: center; gap: 5px;
  border: 1px solid var(--border); border-radius: 8px;
  padding: 5px 11px; background: var(--panel); color: var(--gray);
  font: inherit; font-size: 11.5px; cursor: pointer;
}
:is(#sb, #home) .mp-ghost:hover { color: var(--fg); border-color: var(--accent); }
:is(#sb, #home) .mp-ghost.danger:hover { color: var(--danger, #d9534f); border-color: var(--danger, #d9534f); }
.merge-panel-demo { width: min(440px, 100%); }
"#;

pub fn parts(st: &str) -> Vec<Node> {
    let s = format!("{st}.status");
    vec![m("div").class("merge-panel").kids(vec![
        m("div")
            .class("mp-band")
            .bind("class", &format!(
                "('mp-band ' + ({s}.state === 'merged' ? 'mp-done' : {s}.state === 'rejected' ? 'mp-bad' : {st}.readyToMerge() ? 'mp-ready' : 'mp-wait'))"
            ))
            .kids(vec![
                m("span").class("mp-ico").child(
                    Node::icon_dyn(&format!(
                        "({s}.state === 'merged' ? 'ph-git-merge' : {s}.state === 'rejected' ? 'ph-prohibit' : {st}.readyToMerge() ? 'ph-seal-check' : 'ph-hourglass')"
                    )),
                ),
                m("div").class("mp-band-main").kids(vec![
                    m("span").class("mp-band-title").text(&format!(
                        "({s}.state === 'merged' ? 'Merged' : {s}.state === 'rejected' ? 'Rejected' : {st}.readyToMerge() ? 'Ready to merge' : 'Waiting before merge')"
                    )),
                    m("span").class("mp-band-sub").text(&format!(
                        "({s}.approved ? (s.approvals + ' of ' + s.minApprovals + ' approvals') : (s.approvals + ' of ' + s.minApprovals + ' approvals · ' + ({st}.mergeReason() || 'reviews open')))"
                    )),
                ]),
            ]),
        m("div").class("mp-rows").kids(vec![
            m("div")
                .class("mp-row")
                .for_each(&format!("c in {st}.checks"))
                .key("c.runId")
                .kids(vec![
                    m("span").class("mp-row-ico").child(Node::icon("path")),
                    m("span").text("c.label"),
                    Node::host(job_run_chip::NAME, &[]).bind("state", "c.state"),
                    m("button")
                        .class("mp-ghost")
                        .attr("type", "button")
                        .attr("title", "open the run")
                        .on("click", &format!("$store.jobs.openRun('' + c.runId)"))
                        .child(lit("open")),
                ]),
            m("div")
                .class("mp-row mp-row-warn")
                .show(&format!("{s}.outOfDate"))
                .kids(vec![
                    m("span").class("mp-row-ico").child(Node::icon("arrow-counter-clockwise")),
                    m("span").child(lit("the target branch has new commits — merge main into this branch before merging")),
                ]),
            m("div")
                .class("mp-row mp-row-bad")
                .for_each(&format!("f in {s}.conflictedFiles"))
                .key("f")
                .kids(vec![
                    m("span").class("mp-row-ico").child(Node::icon("warning")),
                    m("span").child(lit("conflict in")),
                    m("code").text("f"),
                ]),
            m("div")
                .class("mp-row")
                .show(&format!("{s}.mergeCommit"))
                .kids(vec![
                    m("span").class("mp-row-ico mp-row-ok").child(Node::icon("git-commit")),
                    m("span").child(lit("merged as")),
                    m("code").text(&format!("({s}.mergeCommit || '').slice(0, 10)")),
                ]),
        ]),
        m("div").class("mp-foot").kids(vec![
            m("span").class("mp-reason").text(&format!("({st}.mergeReason() || 'good to go')")),
            m("button")
                .class("mp-merge")
                .attr("type", "button")
                .show(&format!("{s}.state !== 'merged' && {s}.state !== 'rejected'"))
                .bind("disabled", &format!("(!{st}.readyToMerge())"))
                .on("click", &format!("{st}.mergePr()"))
                .kids(vec![Node::icon("git-merge"), m("span").child(lit("Merge pull request"))]),
            m("button")
                .class("mp-ghost")
                .attr("type", "button")
                .show(&format!("{s}.state === 'merged'"))
                .on("click", &format!("{st}.cleanupPr('branch')"))
                .kids(vec![Node::icon("trash"), m("span").child(lit("delete branch + reap worktree"))]),
            m("button")
                .class("mp-ghost danger")
                .attr("type", "button")
                .show(&format!("{s}.state === 'merged' || {s}.state === 'rejected'"))
                .on("click", &format!("{st}.deletePr()"))
                .kids(vec![Node::icon("x"), m("span").child(lit("delete pull request"))]),
        ]),
    ])]
}

pub fn demo() -> Node {
    m("div").class("merge-panel-demo").kids(parts(vcs::EXPR))
}

pub fn register() {
    css::inject("merge-panel", CSS);
}

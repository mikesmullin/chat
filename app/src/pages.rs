//! The demo page: fullscreen chat + sidebar, one shared store.
//!
//! Rust port of `m-js-cmp-chat/demo/app.js` + `demo/index.html` composed with
//! `src/templates.js` (`chatTpl` + friends) via `m()` hyperscript. The tree
//! compiles to the same m-js template string the JS widgets use, so
//! directives (`x-for`, `x-text`, `:class`, `@click`) work identically — the
//! gain is that mis-nested markup is unwriteable and the kit linter checks
//! every directive before anything paints.
//!
//! Layout contract (from the JS demo):
//!   * `.demo` grid: `.demo-full` (mode-fullscreen: history + tabs + agent +
//!     transcript + composer) and `.demo-side` (mode-sidebar sheet: same
//!     transcript + composer, no tabs/agent/goto/visibility/starters).
//!   * Both panes read the same `chat` store, exactly like the demo mounts
//!     two `mjs-chat` widgets against one `createChatStore`.

use rustex1_kit::h::{lit, m, Node};
use rustex1_kit::mjs::Def;
use rustex1_kit::{mjs, Runtime};

use crate::store::STORE;

/// One pane of the demo. The fullscreen pane is the primary chat; the sidebar
/// is the compact rail the demo mounts beside it — same store, fewer chrome.
struct Pane {
    /// id prefix: `chat` (fullscreen) or `chat-side` (sidebar).
    p: &'static str,
    sidebar: bool,
}

const FULL: Pane = Pane { p: "chat", sidebar: false };
const SIDE: Pane = Pane { p: "chat-side", sidebar: true };

pub fn register(rt: &mut Runtime) {
    mjs::router_set_mode("path");
    rt.route("/", "Chat", |_params| {
        let mut d = Def::new().named("route /");
        d.tree(shell());
        d.finish()
    });
}

fn shell() -> Node {
    // No outer brand bar: the demo grid IS the app, exactly like
    // m-js-cmp-chat/demo/index.html. Status/connected surface inside each
    // pane's topbar (statusLabel) and the transcript (bubble-error).
    m("div").class("shell").child(stage())
}

fn stage() -> Node {
    m("main").class("stage").child(
        // side-collapsed drops the right rail's grid track with it (x-show
        // alone would leave a 380px hole).
        m("div")
            .class("demo")
            // NOTE: plain string (not format!) — double braces here would emit
            // literally and break m-js expression compile (lint rule `syntax`).
            .bind("class", "{ 'side-collapsed': !$store.chat.sideOpen }")
            .kids(vec![
                m("div").class("demo-full").child(chat_root(&FULL)),
                m("div").class("demo-side").child(chat_root(&SIDE)),
            ]),
    )
}

fn chat_root(pane: &Pane) -> Node {
    m("div")
        .class(if pane.sidebar {
            "mjs-chat mode-sidebar sheet-chat"
        } else {
            "mjs-chat mode-fullscreen"
        })
        .attr("aria-label", "Session")
        .kids(vec![
            // Sidebar has no history rail (show-history is fullscreen-only).
            if pane.sidebar {
                m("span").attr("hidden", "")
            } else {
                history()
            },
            main_col(pane),
        ])
}

fn history() -> Node {
    m("aside")
        .class("history")
        .show(&format!("$store.{STORE}.historyOpen"))
        .kids(vec![
            m("div").class("history-header").kids(vec![
                lit("History"),
                m("button")
                    .class("icon-btn trash-all")
                    .attr("type", "button")
                    .attr("title", "Delete all sessions")
                    .on("click", &format!("$store.{STORE}.cleanAllSessions()"))
                    .show(&format!("$store.{STORE}.historyList.length > 0"))
                    .child(Node::icon("trash")),
            ]),
            m("div").class("history-list").kids(vec![
                m("div")
                    .class("text-xs px-2 py-3")
                    .show(&format!("$store.{STORE}.historyList.length === 0"))
                    .child(lit("No sessions yet")),
                m("div")
                    .class("history-item-row")
                    .for_each(&format!("item in $store.{STORE}.historyList"))
                    .key("item.id")
                    .bind(
                        "class",
                        &format!("{{ active: $store.{STORE}.activeSessionId === item.id }}"),
                    )
                    .kids(vec![
                        m("button")
                            .class("history-item")
                            .attr("type", "button")
                            .on("click", &format!("$store.{STORE}.openSession(item.id)"))
                            .kids(vec![
                                m("div")
                                    .class("history-title")
                                    .text("item.title || item.agent || item.id"),
                                m("div").class("history-meta").kids(vec![
                                    m("span").text("item.agent || ''"),
                                    m("span")
                                        .show("item.model")
                                        .text("' · ' + $store.chat.shortModel(item.model)"),
                                ]),
                            ]),
                        m("button")
                            .class("trash-one")
                            .attr("type", "button")
                            .attr("title", "Delete this session")
                            .on("click.stop", &format!("$store.{STORE}.deleteSession(item.id)"))
                            .child(Node::icon("trash")),
                    ]),
            ]),
        ])
}

fn main_col(pane: &Pane) -> Node {
    // NOTE: Node::class REPLACES (kit h.rs: later call wins), so multi-class
    // nodes must be set in ONE call — chaining drops the earlier class.
    let cls = if pane.sidebar { "main sheet-chat" } else { "main" };
    m("section").class(cls).kids(vec![topbar(pane), transcript(pane), composer(pane)])
}

fn topbar(pane: &Pane) -> Node {
    if pane.sidebar {
        // Demo sidebar: no tabs toggle, no tab strip, no fullscreen button —
        // just the status readout (show-status). Collapsing is owned by the
        // one sidebar toggle in the fullscreen topbar, which stays visible
        // while the rail is hidden.
        return m("header")
            .class("topbar sheet-chat-top")
            .child(
                m("div").class("sheet-chat-tabs").child(
                    m("div").class("chat-header-meta").kids(vec![
                        m("span")
                            .class("chat-status")
                            .bind("class", &format!("$store.{STORE}.statusClass"))
                            .text(&format!("$store.{STORE}.statusLabel")),
                        m("span")
                            .class("chat-elapsed")
                            .show(&format!("$store.{STORE}.elapsed"))
                            .text(&format!("$store.{STORE}.elapsed")),
                    ]),
                )
            );
    }
    m("header").class("topbar").kids(vec![
        m("button")
            .class("icon-btn")
            .attr("type", "button")
            .attr("title", "Toggle history")
            .on("click", &format!("$store.{STORE}.toggleHistory()"))
            .child(Node::icon("list")),
        m("div")
            .kids(vec![m("div").class("tabs").kids(vec![
                m("div")
                    .class("tab")
                    .for_each(&format!("tab in $store.{STORE}.tabList"))
                    .key("tab.id")
                    .bind(
                        "class",
                        &format!("{{ active: tab.id === $store.{STORE}.activeTabId }}"),
                    )
                    .on("click", &format!("$store.{STORE}.selectTab(tab.id)"))
                    .kids(vec![
                        m("span").text("tab.name"),
                        m("button")
                            .class("x")
                            .attr("type", "button")
                            .attr("title", "Close")
                            .on("click.stop", &format!("$store.{STORE}.closeTab(tab.id)"))
                            .child(Node::icon("x")),
                    ]),
                m("button")
                    .class("icon-btn")
                    .attr("type", "button")
                    .attr("title", "New chat")
                    .on("click", &format!("$store.{STORE}.newTab()"))
                    .child(Node::icon("plus")),
            ])])
            .attr("style", "flex:1;min-width:0;display:flex;align-items:center;gap:8px"),
        m("div").class("chat-header-meta").kids(vec![
            m("span")
                .class("chat-status")
                .bind("class", &format!("$store.{STORE}.statusClass"))
                .text(&format!("$store.{STORE}.statusLabel")),
            m("span")
                .class("chat-elapsed")
                .show(&format!("$store.{STORE}.elapsed"))
                .text(&format!("$store.{STORE}.elapsed")),
        ]),
        m("button")
            .class("icon-btn")
            .attr("type", "button")
            .on("click", &format!("$store.{STORE}.toggleFullscreen()"))
            .bind(
                "title",
                &format!(
                    "$store.{STORE}.isFullscreen ? 'Exit fullscreen' : 'Fullscreen chat'"
                ),
            )
            .kids(vec![
                m("i")
                    .class("ph ph-corners-out")
                    .show(&format!("!$store.{STORE}.isFullscreen")),
                m("i")
                    .class("ph ph-corners-in")
                    .show(&format!("$store.{STORE}.isFullscreen")),
            ]),
        // Right rail toggle (the left rail's is the history button).
        m("button")
            .class("icon-btn")
            .attr("type", "button")
            .attr("title", "Toggle sidebar")
            .bind("class", &format!("{{ active: $store.{STORE}.sideOpen }}"))
            .on("click", &format!("$store.{STORE}.toggleSide()"))
            .child(Node::icon("sidebar")),
    ])
}

fn transcript(pane: &Pane) -> Node {
    let cls = if pane.sidebar { "messages sheet-messages" } else { "messages" };
    m("div").class(cls)
        .attr("id", &format!("{p}-messages", p = pane.p))
        .kids(vec![welcome(pane), message_block(pane), thinking()])
}

fn welcome(pane: &Pane) -> Node {
    // Host slot content, per demo pane: fullscreen greets as Cowork, the
    // sidebar as the compact Session rail reading the same transcript.
    let (title, lead) = if pane.sidebar {
        ("Session", "Same messages as the fullscreen pane.")
    } else {
        ("Cowork", "Browser UI → angela → MCP. Rust port of m-js-cmp-chat.")
    };
    let cls = if pane.sidebar { "welcome sheet-welcome" } else { "welcome" };
    let w = m("div").class(cls);
    let mut kids: Vec<Node> = vec![m("h1").child(lit(title)), m("p").child(lit(lead))];
    if !pane.sidebar {
        // show-starters is fullscreen-only (and the list is empty until an
        // agent seeds it — hidden until then, like the original).
        kids.push(
            m("div")
                .class("starter")
                .show(&format!("$store.{STORE}.welcomeStarters.length"))
                .child(
                    m("button")
                        .attr("type", "button")
                        .for_each(&format!("s in $store.{STORE}.welcomeStarters"))
                        .key("s.prompt")
                        .on("click", &format!("$store.{STORE}.useStarter(s.prompt)"))
                        .text("s.label"),
                ),
        );
    }
    w.show(&format!("$store.{STORE}.showWelcome")).kids(kids)
}

fn message_block(pane: &Pane) -> Node {
    m("div")
        .class("msg-block")
        .for_each(&format!("msg in $store.{STORE}.activeMessages"))
        .key("msg.id")
        .bind(
            "class",
            &format!("{{ 'msg-hidden': msg.visible === false, 'msg-user': msg.kind === 'user' }}"),
        )
        .bind("data-msg-id", "msg.id")
        .kids(vec![
            // user
            m("div")
                .class("bubble-user")
                .show("msg.kind === 'user'")
                .text("msg.text"),
            // assistant streaming
            m("div")
                .class("bubble-assistant md-body is-streaming")
                .show("msg.kind === 'assistant' && msg.streaming")
                .kids(vec![
                    m("span").text("msg.text || ''"),
                    m("span").class("stream-caret"),
                ]),
            // assistant final (markdown)
            m("div")
                .class("bubble-assistant md-body")
                .show("msg.kind === 'assistant' && !msg.streaming")
                .inner_html(&format!("$store.{STORE}.renderMarkdown(msg.text)")),
            // error
            m("div")
                .class("bubble-error")
                .show("msg.kind === 'error'")
                .text("msg.text"),
            // reasoning
            m("div")
                .class("bubble-reasoning")
                .show("msg.kind === 'reasoning'")
                .bind("class", "{ 'is-streaming': msg.streaming, 'is-collapsed': msg.collapsed }")
                .kids(vec![
                    m("div")
                        .class("reasoning-header bubble-fold-header")
                        .attr("title", "Click to collapse or expand")
                        .kids(vec![
                            Node::icon("brain"),
                            m("span").class("reasoning-label").child(lit("Reasoning")),
                        ]),
                    m("div")
                        .class("reasoning-body bubble-scroll")
                        .show("!msg.collapsed")
                        .text("msg.text || ''"),
                ]),
            // tool_call
            m("div")
                .class("tool-card tool-call-card")
                .show("msg.kind === 'tool_call'")
                .kids(vec![
                    m("div").class("tool-call-header").kids(vec![
                        Node::icon("wrench"),
                        m("span").class("tool-call-name").text("msg.name"),
                    ]),
                    m("div")
                        .class("tool-call-explanation")
                        .show(&format!("$store.{STORE}.toolCallExplanation(msg)"))
                        .text(&format!("$store.{STORE}.toolCallExplanation(msg)")),
                    m("pre")
                        .class("tool-call-code bubble-scroll")
                        .show(&format!("$store.{STORE}.toolCallCode(msg)"))
                        .text(&format!("$store.{STORE}.toolCallCode(msg)")),
                    m("pre")
                        .class("tool-call-progress bubble-scroll")
                        .show("msg.progress")
                        .text("msg.progress"),
                ]),
            // tool_result
            m("div")
                .class("tool-card tool-card-row")
                .show("msg.kind === 'tool_result'")
                .bind("class", "(msg.ok === false ? 'err' : 'ok')")
                .kids(vec![
                    m("div").class("tool-result-header").kids(vec![
                        Node::icon("check-circle"),
                        m("span").text("msg.name"),
                        m("span")
                            .class("tool-image-note")
                            .show(&format!("$store.{STORE}.toolResultImageNote(msg)"))
                            .text(&format!("$store.{STORE}.toolResultImageNote(msg)")),
                    ]),
                    m("pre").class("tool-result-body").text("msg.text || ''"),
                ]),
            // approval
            m("div")
                .class("approval-card")
                .show("msg.kind === 'approval_needed'")
                .bind("class", "msg.status === 'pending' ? 'pending' : 'resolved'")
                .kids(vec![
                    m("div").class("font-semibold text-sm approval-title").kids(vec![
                        Node::icon("warning"),
                        lit(" Tool approval required"),
                    ]),
                    m("div").class("text-xs mt-1 font-mono").text("msg.name"),
                    m("div")
                        .class("flex gap-2 mt-2")
                        .show("msg.status === 'pending'")
                        .kids(vec![
                            m("button")
                                .class("btn-allow")
                                .attr("type", "button")
                                .on("click", &format!("$store.{STORE}.decideApproval(msg, 'allow')"))
                                .child(lit("Allow")),
                            m("button")
                                .class("btn-always")
                                .attr("type", "button")
                                .on("click", &format!("$store.{STORE}.decideApproval(msg, 'always')"))
                                .child(lit("Always")),
                            m("button")
                                .class("btn-deny")
                                .attr("type", "button")
                                .on("click", &format!("$store.{STORE}.decideApproval(msg, 'deny')"))
                                .child(lit("Deny")),
                        ]),
                    m("div")
                        .class("text-xs mt-2")
                        .show("msg.status !== 'pending'")
                        .text("'Decision: ' + msg.status"),
                ]),
            // gutter: visibility + goto are fullscreen-only (demo flags).
            gutter(pane),
        ])
}

fn gutter(pane: &Pane) -> Node {
    let mut actions = vec![m("button")
        .class("gutter-btn")
        .attr("type", "button")
        .attr("title", "Include/exclude in context window")
        .on("click", &format!("$store.{STORE}.toggleMsgVisible(msg)"))
        .child(Node::icon_dyn("msg.visible === false ? 'eye-slash' : 'eye'"))];
    if !pane.sidebar {
        actions.push(
            m("button")
                .class("gutter-btn gutter-btn-goto")
                .attr("type", "button")
                .attr("title", "Goto bookmark")
                .on("click", &format!("$store.{STORE}.toggleMsgGoto(msg)"))
                .child(Node::icon("bookmark-simple")),
        );
    }
    m("div").class("msg-gutter").kids(vec![
        m("div").class("msg-gutter-actions").kids(actions),
        m("div")
            .class("msg-gutter-metrics")
            .inner_html(&format!("$store.{STORE}.formatTelemetryHtml(msg.telemetry, msg)")),
    ])
}

fn thinking() -> Node {
    m("div")
        .class("thinking")
        .show(&format!("$store.{STORE}.waiting"))
        .child(lit("Thinking •••"))
}

fn composer(pane: &Pane) -> Node {
    let cls = if pane.sidebar { "composer sheet-composer" } else { "composer" };
    m("footer").class(cls).child(
        m("div").class("composer-inner").kids(vec![
            // todos
            m("div")
                .class("todo-panel")
                .show(&format!("$store.{STORE}.todoItems.length"))
                .kids(vec![
                    m("button")
                        .class("todo-header")
                        .attr("type", "button")
                        .on("click", &format!("$store.{STORE}.toggleTodos()"))
                        .kids(vec![
                            m("span")
                                .class("todo-progress")
                                .text(&format!("$store.{STORE}.todoProgress")),
                            m("span")
                                .class("todo-current")
                                .show(&format!("!$store.{STORE}.todosOpen"))
                                .text(&format!("$store.{STORE}.todoCurrentLabel")),
                        ]),
                    m("ul").class("todo-list").show(&format!("$store.{STORE}.todosOpen")).kids(vec![
                        m("li")
                            .for_each(&format!("t in $store.{STORE}.todoItems"))
                            .key("t.id")
                            .bind("class", "'is-' + (t.status || 'pending')")
                            .kids(vec![
                                m("i").bind("class", &format!("'ph ' + $store.{STORE}.todoIcon(t)")),
                                m("span").class("todo-text").text("t.content"),
                            ]),
                    ]),
                ]),
            // allowlist
            m("div")
                .class("allowlist-panel")
                .show(&format!("$store.{STORE}.allowlistOpen"))
                .kids(vec![
                    m("label").class("allowlist-label").kids(vec![
                        lit("allowed commands (one rule per line) "),
                        m("span")
                            .class("allowlist-source")
                            .text(&format!(
                                "$store.{STORE}.allowlistOverridden ? '· ui override' : '· agent default'"
                            )),
                    ]),
                    m("textarea")
                        .class("allowlist-ta")
                        .attr("id", &format!("{p}-allowlist", p = pane.p))
                        .model(&format!("$store.{STORE}.allowlist"))
                        .on("input", &format!("$store.{STORE}.onAllowlistInput($event)"))
                        .on("blur", &format!("$store.{STORE}.onAllowlistBlur()"))
                        .attr("spellcheck", "false"),
                ]),
            // draft
            m("textarea")
                .class("draft-ta")
                .attr("id", &format!("{p}-draft", p = pane.p))
                .model(&format!("$store.{STORE}.draft"))
                .on("keydown", &format!("$store.{STORE}.onKeydown($event)"))
                .attr("placeholder", "Message… (Enter send, Shift+Enter newline)"),
            // row
            composer_row(pane),
        ]),
    )
}

fn composer_row(pane: &Pane) -> Node {
    let mut kids: Vec<Node> = Vec::new();
    if !pane.sidebar {
        // show-agent is fullscreen-only.
        kids.push(
            m("select")
                .class("dd agent-dd")
                .attr("id", &format!("{p}-agent", p = pane.p))
                .attr("title", "Agent")
                .effect(&format!("$store.{STORE}.fillAgents($el)"))
                .on("change", &format!("$store.{STORE}.onAgentChange($event)"))
                .bind("disabled", &format!("$store.{STORE}.waiting")),
        );
    }
    kids.push(
        m("select")
            .class("dd model-dd")
            .attr("id", &format!("{p}-model", p = pane.p))
            .attr("title", "Model")
            .effect(&format!("$store.{STORE}.fillModels($el)"))
            .on("change", &format!("$store.{STORE}.onModelChange($event)"))
            .bind("disabled", &format!("$store.{STORE}.waiting")),
    );
    kids.push(
        m("div")
            .class("ctx-pie")
            .bind("title", &format!("$store.{STORE}.contextTooltip"))
            .kids(vec![
                m("svg").at(&[("viewBox", "0 0 36 36"), ("width", "28"), ("height", "28")]).kids(vec![
                    m("circle").at(&[
                        ("cx", "18"), ("cy", "18"), ("r", "15.5"),
                        ("fill", "none"), ("stroke", "currentColor"),
                        ("class", "ctx-pie-track"), ("stroke-width", "4"),
                    ]),
                    m("circle")
                        .at(&[
                            ("cx", "18"), ("cy", "18"), ("r", "15.5"),
                            ("fill", "none"), ("stroke", "currentColor"),
                            ("class", "ctx-pie-fill"), ("stroke-width", "4"),
                            ("stroke-linecap", "round"),
                            ("transform", "rotate(-90 18 18)"),
                        ])
                        .bind("stroke-dasharray", &format!("$store.{STORE}.pieDash")),
                ]),
                m("span")
                    .class("ctx-pie-label")
                    .text(&format!("$store.{STORE}.pieUsedK")),
            ]),
    );
    kids.push(
        m("button")
            .class("btn-stop")
            .attr("type", "button")
            .show(&format!("$store.{STORE}.waiting"))
            .on("click", &format!("$store.{STORE}.stop()"))
            .attr("title", "Stop agent")
            .kids(vec![Node::icon("stop"), m("span").child(lit("Stop"))]),
    );
    kids.push(
        m("button")
            .class("icon-btn allowlist-toggle")
            .attr("type", "button")
            .bind("class", &format!("{{ active: $store.{STORE}.allowlistOpen }}"))
            .on("click", &format!("$store.{STORE}.toggleAllowlist()"))
            .attr("title", "Allowlist")
            .child(Node::icon("shield-check")),
    );
    kids.push(
        m("button")
            .class("btn-primary")
            .attr("type", "button")
            .show(&format!("!$store.{STORE}.waiting"))
            .bind("disabled", &format!("!$store.{STORE}.draft.trim()"))
            .on("click", &format!("$store.{STORE}.send()"))
            .kids(vec![Node::icon("paper-plane-right"), m("span").child(lit("Send"))]),
    );
    m("div").class("composer-row").kids(kids)
}

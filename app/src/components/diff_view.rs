use wasm_bindgen::JsValue;

use rustex1_kit::h::{m, Node};

use crate::components::samples;
use crate::css;

pub const CSS: &str = r#"
:is(#sb, #home) .diff-view { display: flex; flex-direction: column; gap: 10px; }
:is(#sb, #home) .diff-empty {
  margin: 0; padding: 10px 12px; border-radius: 8px;
  border: 1px dashed var(--border); color: var(--gray-dim); font-size: 11.5px;
  display: flex; align-items: center; gap: 6px;
}
:is(#sb, #home) .diff-file {
  border: 1px solid var(--border); border-radius: 10px; overflow: hidden;
  background: var(--panel);
}
:is(#sb, #home) .diff-file-head {
  display: flex; align-items: center; gap: 7px;
  padding: 6px 10px; background: var(--panel-2);
  border-bottom: 1px solid var(--border);
  font-size: 11.5px; color: var(--gray-dim); min-width: 0;
}
:is(#sb, #home) .diff-path {
  flex: 1 1 auto; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  color: var(--fg); font-size: 11.5px;
}
:is(#sb, #home) .diff-stats { display: inline-flex; gap: 7px; flex: 0 0 auto; }
:is(#sb, #home) .d-add-count { color: var(--pastel-green, #22c55e); }
:is(#sb, #home) .d-del-count { color: var(--danger, #d9534f); }
:is(#sb, #home) .diff-body {
  overflow-x: auto;
  font-family: var(--mjs-mono, ui-monospace, monospace); font-size: 11px; line-height: 1.55;
}
:is(#sb, #home) .d-line { display: flex; gap: 8px; padding: 0 10px; white-space: pre; }
:is(#sb, #home) .d-line.d-add { background: color-mix(in srgb, var(--pastel-green, #22c55e) 12%, transparent); }
:is(#sb, #home) .d-line.d-del { background: color-mix(in srgb, var(--danger, #d9534f) 11%, transparent); }
:is(#sb, #home) .d-line.d-hunk { background: var(--panel-2); color: var(--accent-soft); }
:is(#sb, #home) .d-line.d-meta { color: var(--gray-dim); font-style: italic; }
:is(#sb, #home) .d-sign { width: 10px; flex: 0 0 auto; color: var(--gray-dim); user-select: none; }
:is(#sb, #home) .d-line.d-add .d-sign { color: var(--pastel-green, #22c55e); }
:is(#sb, #home) .d-line.d-del .d-sign { color: var(--danger, #d9534f); }
.diff-view-demo { width: min(680px, 100%); max-height: 420px; overflow-y: auto; }
"#;

pub fn is_md(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".md") || lower.ends_with(".markdown") || lower.ends_with(".mdown") || lower.ends_with(".mkd")
}

fn meta_line(line: &str) -> bool {
    line.starts_with("+++")
        || line.starts_with("---")
        || line.starts_with("index ")
        || line.starts_with("new file mode")
        || line.starts_with("deleted file mode")
        || line.starts_with("old mode")
        || line.starts_with("new mode")
        || line.starts_with("similarity index")
        || line.starts_with("rename from")
        || line.starts_with("rename to")
        || line.starts_with("copy from")
        || line.starts_with("copy to")
        || line.starts_with("Binary files")
}

fn push_line(files: &mut Vec<File>, sign: &str, cls: &str, text: &str, id: usize) {
    if files.is_empty() {
        files.push(File::new("(unknown)"));
    }
    let f = files.last_mut().unwrap();
    if cls == "d-add" {
        f.added += 1;
    } else if cls == "d-del" {
        f.removed += 1;
    }
    f.lines.push(Line {
        id,
        sign: sign.to_string(),
        cls: cls.to_string(),
        text: text.to_string(),
    });
}

struct Line {
    id: usize,
    sign: String,
    cls: String,
    text: String,
}

struct File {
    path: String,
    added: usize,
    removed: usize,
    lines: Vec<Line>,
}

impl File {
    fn new(path: &str) -> File {
        File {
            path: path.to_string(),
            added: 0,
            removed: 0,
            lines: Vec::new(),
        }
    }
}

fn parse(text: &str) -> Vec<File> {
    let mut out: Vec<File> = Vec::new();
    let mut id = 0usize;
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        if line.starts_with("diff --git ") {
            let path = line
                .strip_prefix("diff --git a/")
                .and_then(|rest| rest.split(" b/").nth(1))
                .unwrap_or(&line[11..])
                .to_string();
            out.push(File::new(&path));
            continue;
        }
        if out.is_empty() {
            if line.trim().is_empty() {
                continue;
            }
            out.push(File::new("(unknown)"));
        }
        if line.starts_with("@@") {
            push_line(&mut out, "", "d-hunk", line, id);
        } else if meta_line(line) {
            push_line(&mut out, "", "d-meta", line, id);
        } else if line.starts_with('+') {
            push_line(&mut out, "+", "d-add", &line[1..], id);
        } else if line.starts_with('-') {
            push_line(&mut out, "−", "d-del", &line[1..], id);
        } else {
            let text = line.strip_prefix(' ').unwrap_or(line);
            push_line(&mut out, "", "d-ctx", text, id);
        }
        id += 1;
    }
    out
}

pub fn parse_json(text: &str) -> JsValue {
    let out = js_sys::Array::new();
    for file in parse(text) {
        let f = rustex1_kit::js::obj();
        rustex1_kit::js::set_str(&f, "path", &file.path);
        rustex1_kit::js::set_bool(&f, "isMd", is_md(&file.path));
        rustex1_kit::js::set_num(&f, "added", file.added as f64);
        rustex1_kit::js::set_num(&f, "removed", file.removed as f64);
        let lines = js_sys::Array::new();
        for line in &file.lines {
            let l = rustex1_kit::js::obj();
            rustex1_kit::js::set_num(&l, "id", line.id as f64);
            rustex1_kit::js::set_str(&l, "sign", &line.sign);
            rustex1_kit::js::set_str(&l, "cls", &line.cls);
            rustex1_kit::js::set_str(&l, "text", &line.text);
            lines.push(&l);
        }
        rustex1_kit::js::set(&f, "lines", &lines);
        let runs = js_sys::Array::new();
        if is_md(&file.path) {
            let mut rid = 0usize;
            let mut current: Option<(String, Vec<String>)> = None;
            let flush = |cls: &str, texts: Vec<String>, rid: usize, runs: &js_sys::Array| {
                let run = rustex1_kit::js::obj();
                rustex1_kit::js::set_num(&run, "id", rid as f64);
                rustex1_kit::js::set_str(&run, "cls", cls);
                let html = crate::helpers::markdown_to_html(&texts.join("\n"));
                rustex1_kit::js::set_str(&run, "html", &html);
                runs.push(&run);
            };
            for line in &file.lines {
                let cls: &str = match line.cls.as_str() {
                    "d-add" => "add",
                    "d-del" => "del",
                    "d-ctx" => "ctx",
                    _ => continue,
                };
                match current.take() {
                    Some((c, mut texts)) if c == cls => {
                        texts.push(line.text.clone());
                        current = Some((c, texts));
                    }
                    Some((c, texts)) => {
                        flush(&c, texts, rid, &runs);
                        rid += 1;
                        current = Some((cls.to_string(), vec![line.text.clone()]));
                    }
                    None => current = Some((cls.to_string(), vec![line.text.clone()])),
                }
            }
            if let Some((c, texts)) = current {
                flush(&c, texts, rid, &runs);
            }
        }
        rustex1_kit::js::set(&f, "runs", &runs);
        out.push(&f);
    }
    out.into()
}

pub fn parts(files_expr: &str) -> Vec<Node> {
    vec![
        m("p")
            .class("diff-empty")
            .show(&format!("{files_expr}.length === 0"))
            .kids(vec![
                Node::icon("file-magnifying-glass"),
                m("span").text("''"),
                rustex1_kit::h::lit(" Empty diff — source has no commits beyond target."),
            ]),
        m("div")
            .class("diff-view")
            .kids(vec![m("section")
                .class("diff-file")
                .for_each(&format!("f in {files_expr}"))
                .key("f.path")
                .kids(vec![
                    m("header").class("diff-file-head").kids(vec![
                        Node::icon("file-text"),
                        m("code").class("diff-path").text("f.path"),
                        m("span").class("diff-stats").kids(vec![
                            m("span").class("d-add-count").text("'+' + f.added"),
                            m("span").class("d-del-count").text("'−' + f.removed"),
                        ]),
                    ]),
                    m("div").class("diff-body").kids(vec![m("div")
                        .class("d-line")
                        .for_each("line in f.lines")
                        .key("line.id")
                        .bind("class", "('d-line ' + line.cls)")
                        .kids(vec![
                            m("span").class("d-sign").text("line.sign"),
                            m("code").class("d-text").text("line.text"),
                        ])]),
                ])]),
    ]
}

pub fn demo() -> Node {
    m("div")
        .class("diff-view-demo")
        .kids(parts(&format!("{}.diffFiles", samples::EXPR)))
}

pub fn register() {
    css::inject("diff-view", CSS);
}

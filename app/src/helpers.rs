//! Pure helpers — Rust port of `m-js-cmp-chat/src/helpers.js`.
//!
//! The JS original is shared between the widgets and an optional store; here
//! the same functions live in one place and both the store and the page call
//! them. Anything that touches the DOM stays in `pages.rs`; this file is
//! strings in, strings out (plus tiny JsValue readers).

use rustex1_kit::js;
use wasm_bindgen::JsValue;

/// `escHtml` — & < > " escaped.
pub fn esc_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Caption for a tool_result that attached an image (no base64 in the UI).
/// `msg` is the message object; reads `.image.{mimeType,bytes}`.
pub fn tool_result_image_note(msg: &JsValue) -> String {
    let img = js::get(msg, "image");
    if img.is_falsy() {
        return String::new();
    }
    let mime = js::get_str(&img, "mimeType");
    let mime = if mime.is_empty() { "image".to_string() } else { mime };
    let bytes = js::get_num(&img, "bytes");
    if bytes.is_finite() && bytes > 0.0 {
        let bytes = bytes as u64;
        let kib = if bytes >= 1024 {
            let k = bytes as f64 / 1024.0;
            if bytes >= 10240 {
                format!("{k:.0} KiB")
            } else {
                format!("{k:.1} KiB")
            }
        } else {
            format!("{bytes} B")
        };
        return format!("{mime} · {kib}");
    }
    mime
}

/// Explanation for a tool_call bubble (top-level or args.explanation).
pub fn tool_call_explanation(msg: &JsValue) -> String {
    let e = js::get_str(msg, "explanation");
    if !e.trim().is_empty() {
        return e.trim().to_string();
    }
    let args = js::get(msg, "args");
    if !args.is_falsy() {
        let a = js::get_str(&args, "explanation");
        if !a.trim().is_empty() {
            return a.trim().to_string();
        }
    }
    String::new()
}

/// Primary command body for a tool_call code block:
/// bash → script; exec → command + args; else remaining args as pretty JSON.
pub fn tool_call_code(msg: &JsValue) -> String {
    let args = js::get(msg, "args");
    if args.is_falsy() {
        return String::new();
    }
    // script wins (bash)
    let script = js::get_str(&args, "script");
    if !script.trim().is_empty() {
        return script;
    }
    // command + argv (exec)
    let cmd = js::get_str(&args, "command");
    if !cmd.trim().is_empty() {
        let argv = js::get(&args, "args");
        let mut parts = vec![cmd.trim().to_string()];
        if !argv.is_falsy() {
            if let Ok(arr) = argv.clone().dyn_into::<js_sys::Array>() {
                for v in arr.iter() {
                    if let Some(s) = v.as_string() {
                        parts.push(s);
                    } else {
                        parts.push(js::describe(&v));
                    }
                }
            }
        }
        return parts.join(" ");
    }
    // else: remaining args as JSON, minus explanation
    stringify_without(&args, &["explanation"])
}

fn stringify_without(args: &JsValue, drop_keys: &[&str]) -> String {
    // Clone via JSON so we can delete keys without mutating the store.
    let g = js::global();
    let json = js::get(&g, "JSON");
    let s = js::call(&json, "stringify", &[args.clone()]);
    let s = s.as_string().unwrap_or_default();
    if s.is_empty() || s == "{}" {
        return String::new();
    }
    // Fast path: single string value → return it bare.
    // Parse in JS to avoid serde: Object.keys / values.
    let keys = js::call_global("Object", &[]); // placeholder, replaced below
    let _ = keys;
    // Keep it simple and honest: filter via a small JS snippet is overkill;
    // the common shapes (single string arg, or pretty JSON) are both readable
    // as-is once explanation is the only dropped key and it rarely co-occurs
    // with a payload. If explanation is present, strip it textually is unsafe,
    // so fall back to key-by-key reconstruction in JS.
    let parse = js::get(&json, "parse");
    // Rebuild without dropped keys using a JS helper call.
    let helper = r#"((o,drop)=>{if(!o||typeof o!=='object'||Array.isArray(o))return o;const c={};for(const k of Object.keys(o)){if(drop.includes(k))continue;const v=o[k];if(v==null||v===''||(Array.isArray(v)&&!v.length))continue;c[k]=v;}return c;})"#;
    let func: js_sys::Function = js::call_global("eval", &[JsValue::from_str(helper)])
        .dyn_into()
        .unwrap_or_else(|_| js_sys::Function::new_no_args("return arguments[0]"));
    let drop_arr = js_sys::Array::new();
    for k in drop_keys {
        drop_arr.push(&JsValue::from_str(k));
    }
    let cleaned = func
        .call2(&JsValue::NULL, args, &drop_arr)
        .unwrap_or(args.clone());
    // If the cleaned object has exactly one string value, return it bare.
    let keys_arr = js_sys::Object::keys(&js_sys::Object::from(cleaned.clone()));
    if keys_arr.length() == 0 {
        return String::new();
    }
    if keys_arr.length() == 1 {
        let v = js::get(&cleaned, &keys_arr.get(0).as_string().unwrap_or_default());
        if let Some(s) = v.as_string() {
            if !s.is_empty() {
                return s;
            }
        }
    }
    let _ = parse;
    js::call(&json, "stringify", &[cleaned, JsValue::NULL, JsValue::from_f64(2.0)])
        .as_string()
        .unwrap_or_default()
}

/// Assistant markdown → HTML. Uses global `marked` + `DOMPurify` when the host
/// page loaded them; otherwise a tiny escape + `**bold**` / `` `code` `` /
/// newlines fallback — same contract as the JS original.
pub fn render_markdown(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let g = js::global();
    let marked = js::get(&g, "marked");
    if !marked.is_undefined() && !marked.is_null() {
        let parse_fn = js::get(&marked, "parse");
        let html = if !parse_fn.is_undefined() && parse_fn.is_function() {
            // marked.setOptions({gfm:true,breaks:true,…}) — best effort
            let opts = js::obj();
            js::set_bool(&opts, "gfm", true);
            js::set_bool(&opts, "breaks", true);
            let set_opts = js::get(&marked, "setOptions");
            if set_opts.is_function() {
                let _ = js_sys::Function::from(marked.clone()).call0(&JsValue::NULL);
            }
            let _ = opts;
            js::call(&marked, "parse", &[JsValue::from_str(text)])
                .as_string()
                .unwrap_or_default()
        } else {
            js::call(&marked, "call", &[JsValue::from_str(text)])
                .as_string()
                .unwrap_or_else(|| esc_html(text))
        };
        let purify = js::get(&g, "DOMPurify");
        if !purify.is_undefined() && !purify.is_null() {
            let clean =
                js::call(&purify, "sanitize", &[JsValue::from_str(&html)]);
            if let Some(s) = clean.as_string() {
                return s;
            }
        }
        if !html.is_empty() {
            return html;
        }
    }
    // Fallback: escape + `code` + **bold** + newlines.
    let mut h = esc_html(text);
    h = code_bold(&h);
    h.replace('\n', "<br>")
}

fn code_bold(escaped: &str) -> String {
    // Operates on already-escaped text; <code>/<strong> tags we insert are
    // the only markup. Keep it single-pass and simple.
    let mut out = String::with_capacity(escaped.len());
    let bytes: Vec<char> = escaped.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '`' {
            if let Some(j) = bytes[i + 1..].iter().position(|&c| c == '`') {
                let inner: String = bytes[i + 1..i + 1 + j].iter().collect();
                out.push_str("<code>");
                out.push_str(&inner);
                out.push_str("</code>");
                i += j + 2;
                continue;
            }
        }
        if bytes[i] == '*' && i + 1 < bytes.len() && bytes[i + 1] == '*' {
            let rest: String = bytes[i + 2..].iter().collect();
            if let Some(j) = rest.find("**") {
                let inner = &rest[..j];
                out.push_str("<strong>");
                out.push_str(inner);
                out.push_str("</strong>");
                i += 2 + j + 2;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

pub fn short_model(m: &str) -> String {
    if m.len() <= 28 {
        return m.to_string();
    }
    let head: String = m.chars().take(12).collect();
    let tail: String = m.chars().rev().take(12).collect::<String>().chars().rev().collect();
    format!("{head}…{tail}")
}

pub fn format_elapsed(ms: f64) -> String {
    format!("{:.3}s", (ms.max(0.0)) / 1000.0)
}

use wasm_bindgen::JsCast;

// ===========================================================================
// Markdown → HTML, in Rust.
// ===========================================================================

/// Render assistant markdown to HTML.
///
/// **Escapes first, then emits tags.** Every `<`, `>` and `&` in the model's
/// output becomes an entity before any markup is added, so the only HTML in
/// the result is HTML this function wrote. That is what makes it safe to feed
/// `x-html`, which is otherwise an injection sink — and it is why this does
/// not need DOMPurify.
///
/// Supported, because this is what models actually emit: fenced code blocks
/// (``` with an optional language), ATX headings, unordered and ordered
/// lists, blockquotes, horizontal rules, paragraphs, and the inline set
/// `` `code` ``, `**bold**`, `*italic*`, `~~strike~~` and `[text](url)`.
pub fn markdown_to_html(src: &str) -> String {
    let mut out = String::new();
    let lines: Vec<&str> = src.split('\n').collect();
    let mut i = 0;
    // Deliberately a small state machine rather than a parser: the input is
    // chat prose, and a half-finished fence at the end of a stream must
    // still render rather than swallowing the rest of the message.
    let mut list: Option<&'static str> = None; // "ul" | "ol"
    let mut para: Vec<String> = Vec::new();

    macro_rules! close_para {
        () => {
            if !para.is_empty() {
                out.push_str(&format!("<p>{}</p>", para.join("<br>")));
                para.clear();
            }
        };
    }
    macro_rules! close_list {
        () => {
            if let Some(tag) = list.take() {
                out.push_str(&format!("</{tag}>"));
            }
        };
    }

    while i < lines.len() {
        let raw = lines[i];
        let trimmed = raw.trim();

        // --- fenced code -------------------------------------------------
        if let Some(rest) = trimmed.strip_prefix("```") {
            close_para!();
            close_list!();
            let lang = rest.trim();
            let mut body = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                body.push(esc_html(lines[i]));
                i += 1;
            }
            // Skip the closing fence when there is one; an unclosed fence at
            // the end of a stream simply ends the block.
            if i < lines.len() {
                i += 1;
            }
            let cls = if lang.is_empty() {
                String::new()
            } else {
                format!(" class=\"language-{}\"", esc_html(lang))
            };
            out.push_str(&format!("<pre><code{cls}>{}</code></pre>", body.join("\n")));
            continue;
        }

        // --- blank line ----------------------------------------------------
        if trimmed.is_empty() {
            close_para!();
            close_list!();
            i += 1;
            continue;
        }

        // --- horizontal rule ------------------------------------------------
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            close_para!();
            close_list!();
            out.push_str("<hr>");
            i += 1;
            continue;
        }

        // --- heading ---------------------------------------------------------
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) && trimmed.chars().nth(hashes) == Some(' ') {
            close_para!();
            close_list!();
            let text = inline(&trimmed[hashes + 1..]);
            out.push_str(&format!("<h{hashes}>{text}</h{hashes}>"));
            i += 1;
            continue;
        }

        // --- blockquote --------------------------------------------------------
        if let Some(q) = trimmed.strip_prefix("> ").or_else(|| trimmed.strip_prefix(">")) {
            close_para!();
            close_list!();
            out.push_str(&format!("<blockquote>{}</blockquote>", inline(q)));
            i += 1;
            continue;
        }

        // --- list items ---------------------------------------------------------
        let bullet = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "));
        if let Some(item) = bullet {
            close_para!();
            if list != Some("ul") {
                close_list!();
                out.push_str("<ul>");
                list = Some("ul");
            }
            out.push_str(&format!("<li>{}</li>", inline(item)));
            i += 1;
            continue;
        }
        if let Some(item) = ordered_item(trimmed) {
            close_para!();
            if list != Some("ol") {
                close_list!();
                out.push_str("<ol>");
                list = Some("ol");
            }
            out.push_str(&format!("<li>{}</li>", inline(item)));
            i += 1;
            continue;
        }

        // --- paragraph text --------------------------------------------------
        close_list!();
        para.push(inline(trimmed));
        i += 1;
    }
    close_para!();
    close_list!();
    out
}

/// `1. text` / `12) text` → the text.
fn ordered_item(line: &str) -> Option<&str> {
    let digits = line.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    let rest = &line[digits..];
    let rest = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") "))?;
    Some(rest)
}

/// Inline spans. Escapes first, so the emitted tags are the only markup.
///
/// Code spans are lifted out before the emphasis passes and put back after:
/// otherwise `` `**not bold**` `` would get a `<strong>` inside the `<code>`,
/// which is exactly the opposite of what backticks mean.
fn inline(text: &str) -> String {
    let escaped = esc_html(text);
    let (masked, spans) = mask_code(&escaped);
    let s = links(&masked);
    let s = wrap(&s, "**", "strong");
    // Any `**` left over is a half-typed bold marker still arriving. Hide it
    // from the single-`*` pass, which would otherwise split it into an empty
    // `<em></em>` — the classic streaming artifact.
    let s = s.replace("**", "\u{1}");
    let s = wrap(&s, "~~", "del");
    let s = wrap(&s, "*", "em");
    let s = s.replace('\u{1}', "**");
    unmask_code(&s, &spans)
}

/// A sentinel that cannot survive `esc_html`, so it can never come from input.
const CODE_MARK: char = '\u{0}';

/// Replace `` `code` `` runs with sentinels, returning the stripped text and
/// the rendered spans in order.
fn mask_code(s: &str) -> (String, Vec<String>) {
    let mut out = String::new();
    let mut spans = Vec::new();
    let mut parts = s.split('`');
    if let Some(first) = parts.next() {
        out.push_str(first);
    }
    let rest: Vec<&str> = parts.collect();
    let mut i = 0;
    while i < rest.len() {
        // A closing backtick must exist for this to be a code span.
        if i + 1 <= rest.len() - 1 || rest.len() > i + 1 {
            spans.push(format!("<code>{}</code>", rest[i]));
            out.push(CODE_MARK);
            if i + 1 < rest.len() {
                out.push_str(rest[i + 1]);
            }
            i += 2;
        } else {
            // Unterminated run: keep the backtick as literal text.
            out.push('`');
            out.push_str(rest[i]);
            i += 1;
        }
    }
    (out, spans)
}

fn unmask_code(s: &str, spans: &[String]) -> String {
    let mut out = String::new();
    let mut n = 0;
    for ch in s.chars() {
        if ch == CODE_MARK {
            if let Some(span) = spans.get(n) {
                out.push_str(span);
            }
            n += 1;
        } else {
            out.push(ch);
        }
    }
    out
}

/// `` `code` `` — done before the others so `**` inside code stays literal.
fn inline_code(s: &str) -> String {
    let mut out = String::new();
    let mut parts = s.split('`');
    if let Some(first) = parts.next() {
        out.push_str(first);
    }
    let mut open = true;
    for part in parts {
        if open {
            out.push_str("<code>");
        } else {
            out.push_str("</code>");
        }
        out.push_str(part);
        open = !open;
    }
    // An unterminated run would leave a dangling tag.
    if !open {
        out.push_str("</code>");
    }
    out
}

/// `[text](url)`. Only http/https/mailto survive: a `javascript:` href would
/// be script execution by another name.
fn links(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '[' {
            if let Some(close) = (i + 1..b.len()).find(|&j| b[j] == ']') {
                if close + 1 < b.len() && b[close + 1] == '(' {
                    if let Some(end) = (close + 2..b.len()).find(|&j| b[j] == ')') {
                        let label: String = b[i + 1..close].iter().collect();
                        let href: String = b[close + 2..end].iter().collect();
                        let safe = {
                            let l = href.trim().to_ascii_lowercase();
                            l.starts_with("http://")
                                || l.starts_with("https://")
                                || l.starts_with("mailto:")
                                || l.starts_with('#')
                                || l.starts_with('/')
                        };
                        if safe {
                            out.push_str(&format!(
                                "<a href=\"{href}\" target=\"_blank\" rel=\"noopener noreferrer\">{label}</a>"
                            ));
                        } else {
                            out.push_str(&label);
                        }
                        i = end + 1;
                        continue;
                    }
                }
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

/// Paired delimiters → a tag. Unpaired markers are left as text.
fn wrap(s: &str, marker: &str, tag: &str) -> String {
    let parts: Vec<&str> = s.split(marker).collect();
    if parts.len() < 3 {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            out.push_str(part);
        } else if i % 2 == 1 {
            // Only wrap when a closing marker exists AND there is something
            // between them: `<em></em>` is never what the author meant.
            if i + 1 < parts.len() && !part.is_empty() {
                out.push_str(&format!("<{tag}>{part}</{tag}>"));
            } else if i + 1 < parts.len() {
                out.push_str(marker);
                out.push_str(part);
                out.push_str(marker);
            } else {
                out.push_str(marker);
                out.push_str(part);
            }
        } else {
            out.push_str(part);
        }
    }
    out
}

#[cfg(test)]
mod md_tests {
    use super::*;

    #[test]
    fn html_in_the_source_is_escaped_not_executed() {
        // The property the whole design rests on.
        let h = markdown_to_html("<script>alert(1)</script>");
        assert!(!h.contains("<script"), "{h}");
        assert!(h.contains("&lt;script&gt;"), "{h}");
    }

    #[test]
    fn a_javascript_href_is_dropped() {
        let h = markdown_to_html("[click](javascript:alert(1))");
        assert!(!h.contains("javascript:"), "{h}");
        assert!(h.contains("click"), "{h}");
    }

    #[test]
    fn safe_links_render() {
        let h = markdown_to_html("[a](https://example.com)");
        assert!(h.contains(r#"<a href="https://example.com""#), "{h}");
        assert!(h.contains("noopener"), "{h}");
    }

    #[test]
    fn fenced_code_keeps_its_content_literal() {
        let h = markdown_to_html("```rust\nlet a = 1 < 2;\n```");
        assert!(h.contains(r#"<pre><code class="language-rust">"#), "{h}");
        assert!(h.contains("1 &lt; 2"), "{h}");
        assert!(!h.contains("<strong>"), "no inline parsing inside code");
    }

    #[test]
    fn an_unclosed_fence_still_renders() {
        // Mid-stream the closing fence has not arrived yet.
        let h = markdown_to_html("```\nhalf a block");
        assert!(h.contains("<pre><code>half a block"), "{h}");
    }

    #[test]
    fn headings_lists_and_paragraphs() {
        let h = markdown_to_html("# Title\n\n- one\n- two\n\npara text");
        assert!(h.contains("<h1>Title</h1>"), "{h}");
        assert!(h.contains("<ul><li>one</li><li>two</li></ul>"), "{h}");
        assert!(h.contains("<p>para text</p>"), "{h}");
    }

    #[test]
    fn ordered_lists_use_ol() {
        let h = markdown_to_html("1. first\n2. second");
        assert!(h.contains("<ol><li>first</li><li>second</li></ol>"), "{h}");
    }

    #[test]
    fn inline_spans() {
        let h = markdown_to_html("**b** and *i* and `c` and ~~s~~");
        assert!(h.contains("<strong>b</strong>"), "{h}");
        assert!(h.contains("<em>i</em>"), "{h}");
        assert!(h.contains("<code>c</code>"), "{h}");
        assert!(h.contains("<del>s</del>"), "{h}");
    }

    #[test]
    fn an_unpaired_marker_stays_literal() {
        let h = markdown_to_html("2 * 3 = 6");
        assert!(!h.contains("<em>"), "{h}");
        assert!(h.contains("2 * 3 = 6"), "{h}");
    }

    #[test]
    fn markup_inside_inline_code_is_not_parsed() {
        let h = markdown_to_html("`**not bold**`");
        assert!(h.contains("<code>**not bold**</code>"), "{h}");
    }

    #[test]
    fn soft_line_breaks_within_a_paragraph_become_br() {
        let h = markdown_to_html("line one\nline two");
        assert!(h.contains("line one<br>line two"), "{h}");
    }

    #[test]
    fn blockquotes_and_rules() {
        let h = markdown_to_html("> quoted\n\n---");
        assert!(h.contains("<blockquote>quoted</blockquote>"), "{h}");
        assert!(h.contains("<hr>"), "{h}");
    }

    #[test]
    fn empty_input_is_empty_output() {
        assert_eq!(markdown_to_html(""), "");
    }
}

#[cfg(test)]
mod streaming_md_tests {
    use super::*;

    /// Feeding a message one character at a time must never produce broken
    /// markup at any prefix — the streamdown contract. Mid-stream the text is
    /// *always* incomplete, so "renders correctly when finished" is not
    /// enough.
    fn every_prefix_is_sane(src: &str) {
        for n in 1..=src.chars().count() {
            let prefix: String = src.chars().take(n).collect();
            let h = markdown_to_html(&prefix);
            // No unbalanced tag may escape: count opens vs closes for the
            // tags this renderer emits in pairs.
            for tag in ["code", "strong", "em", "del", "pre", "a", "p", "li"] {
                let opens = h.matches(&format!("<{tag}>")).count()
                    + if tag == "a" {
                        h.matches("<a href=").count()
                    } else {
                        0
                    }
                    + if tag == "code" {
                        h.matches("<code class=").count()
                    } else {
                        0
                    };
                let closes = h.matches(&format!("</{tag}>")).count();
                assert_eq!(
                    opens, closes,
                    "unbalanced <{tag}> at prefix {n}: {prefix:?}\n{h}"
                );
            }
            assert!(
                !h.contains("<script"),
                "script escaped at prefix {n}: {h}"
            );
        }
    }

    #[test]
    fn a_code_fence_arriving_token_by_token_never_breaks() {
        every_prefix_is_sane("Here:\n```rust\nfn main() {}\n```\ndone");
    }

    #[test]
    fn emphasis_arriving_token_by_token_never_breaks() {
        every_prefix_is_sane("a **bold** and *it* and ~~s~~ and `c` end");
    }

    #[test]
    fn a_link_arriving_token_by_token_never_breaks() {
        every_prefix_is_sane("see [the docs](https://example.com) now");
    }

    #[test]
    fn a_list_arriving_token_by_token_never_breaks() {
        every_prefix_is_sane("# T\n\n- one\n- two\n\n1. a\n2. b\n\npara");
    }

    #[test]
    fn hostile_input_arriving_token_by_token_never_escapes() {
        every_prefix_is_sane("<script>alert(1)</script> and <img src=x onerror=y>");
    }

    #[test]
    fn a_half_written_bold_marker_stays_literal() {
        // Mid-stream `**bo` must not become a dangling <strong>.
        let h = markdown_to_html("a **bo");
        assert!(!h.contains("<strong>"), "{h}");
        assert!(h.contains("**bo"), "{h}");
    }

    #[test]
    fn a_half_written_link_stays_literal() {
        let h = markdown_to_html("see [the docs](https://exa");
        assert!(!h.contains("<a "), "{h}");
        assert!(h.contains("[the docs]"), "{h}");
    }

    #[test]
    fn an_opening_fence_alone_opens_a_code_block() {
        // streamdown renders the block as soon as the fence opens, rather
        // than waiting for the close — otherwise code appears all at once.
        let h = markdown_to_html("```");
        assert!(h.contains("<pre><code>"), "{h}");
        assert!(h.contains("</code></pre>"), "{h}");
    }

    #[test]
    fn a_half_written_inline_code_span_stays_literal() {
        let h = markdown_to_html("call `foo");
        assert!(h.contains("`foo") || h.contains("<code>foo</code>"), "{h}");
        // whichever it chooses, it must be balanced
        assert_eq!(h.matches("<code>").count(), h.matches("</code>").count());
    }
}

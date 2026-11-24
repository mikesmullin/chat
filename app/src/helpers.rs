//! Pure helpers — Rust port of `m-js-cmp-chat/src/helpers.js`.
//!
//! The JS original is shared between the widgets and an optional store; here
//! the same functions live in one place and both the store and the page call
//! them. Anything that touches the DOM stays in `pages.rs`; this file is
//! strings in, strings out (plus tiny JsValue readers).


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

pub fn format_elapsed(ms: f64) -> String {
    format!("{:.3}s", (ms.max(0.0)) / 1000.0)
}


// ===========================================================================
// Markdown → HTML, in Rust.
// ===========================================================================

/// Render message markdown to HTML.
///
/// **Escapes first, then emits tags.** Every `<`, `>` and `&` in the model's
/// output becomes an entity before any markup is added, so the only HTML in
/// the result is HTML this function wrote. That is what makes it safe to feed
/// `x-html`, which is otherwise an injection sink — and it is why this does
/// not need DOMPurify.
///
/// Supported, because this is what models actually emit: fenced code blocks
/// (``` with an optional language), ATX headings, unordered and ordered
/// lists, pipe tables, blockquotes, horizontal rules, paragraphs, and the inline set
/// `` `code` ``, `**bold**`, `*italic*`, `~~strike~~`, `[text](url)` and
/// bare HTTP(S) URLs.
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

        // A delimiter row distinguishes a table from ordinary pipe prose.
        if i + 1 < lines.len() {
            if let (Some(header), Some(align)) = (table_cells(raw), table_delimiter(lines[i + 1])) {
                if header.len() == align.len() {
                    close_para!();
                    close_list!();
                    out.push_str("<div class=\"md-table-wrap\"><table><thead><tr>");
                    for (cell, alignment) in header.iter().zip(&align) {
                        table_cell(&mut out, "th", cell, alignment);
                    }
                    out.push_str("</tr></thead><tbody>");
                    i += 2;
                    while i < lines.len() {
                        let Some(cells) = table_cells(lines[i]) else { break };
                        out.push_str("<tr>");
                        for (col, alignment) in align.iter().enumerate() {
                            table_cell(&mut out, "td", cells.get(col).map_or("", String::as_str), alignment);
                        }
                        out.push_str("</tr>");
                        i += 1;
                    }
                    out.push_str("</tbody></table></div>");
                    continue;
                }
            }
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

/// Split a pipe row, preserving escaped pipes and pipes inside code spans.
fn table_cells(line: &str) -> Option<Vec<String>> {
    let line = line.trim();
    if line.is_empty() { return None; }
    let mut cells = vec![String::new()];
    let mut chars = line.chars().peekable();
    let mut in_code = false;
    let mut separators = 0;
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'|') {
            chars.next();
            cells.last_mut().unwrap().push('|');
        } else if ch == '`' {
            in_code = !in_code;
            cells.last_mut().unwrap().push(ch);
        } else if ch == '|' && !in_code {
            separators += 1;
            cells.push(String::new());
        } else {
            cells.last_mut().unwrap().push(ch);
        }
    }
    if separators == 0 { return None; }
    if line.starts_with('|') { cells.remove(0); }
    if line.ends_with('|') && cells.last().is_some_and(|s| s.trim().is_empty()) { cells.pop(); }
    Some(cells.into_iter().map(|s| s.trim().to_string()).collect())
}

fn table_delimiter(line: &str) -> Option<Vec<&'static str>> {
    table_cells(line)?.iter().map(|cell| {
        let dashes = cell.trim_matches(':');
        if dashes.is_empty() || !dashes.chars().all(|c| c == '-') { return None; }
        Some(match (cell.starts_with(':'), cell.ends_with(':')) {
            (true, true) => "center",
            (false, true) => "right",
            (true, false) => "left",
            _ => "",
        })
    }).collect()
}

fn table_cell(out: &mut String, tag: &str, cell: &str, alignment: &str) {
    let class = if alignment.is_empty() { String::new() } else { format!(" class=\"md-align-{alignment}\"") };
    out.push_str(&format!("<{tag}{class}>{}</{tag}>", inline(cell)));
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
    let (s, anchors) = links(&masked);
    let s = wrap(&s, "**", "strong");
    // Any `**` left over is a half-typed bold marker still arriving. Hide it
    // from the single-`*` pass, which would otherwise split it into an empty
    // `<em></em>` — the classic streaming artifact.
    let s = s.replace("**", "\u{1}");
    let s = wrap(&s, "~~", "del");
    let s = wrap(&s, "*", "em");
    let s = s.replace('\u{1}', "**");
    let s = unmask(&s, LINK_MARK, &anchors);
    unmask(&s, CODE_MARK, &spans)
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

fn unmask(s: &str, mark: char, spans: &[String]) -> String {
    let mut out = String::new();
    let mut n = 0;
    for ch in s.chars() {
        if ch == mark {
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

/// Render explicit Markdown links and bare HTTP(S) URLs. Anchor markup is
/// masked until after emphasis parsing, so punctuation in an href stays data.
const LINK_MARK: char = '\u{2}';

fn links(s: &str) -> (String, Vec<String>) {
    let b: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut anchors = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '[' {
            if let Some(close) = (i + 1..b.len()).find(|&j| b[j] == ']') {
                if close + 1 < b.len() && b[close + 1] == '(' {
                    if let Some(end) = (close + 2..b.len()).find(|&j| b[j] == ')') {
                        let label: String = b[i + 1..close].iter().collect();
                        let href: String = b[close + 2..end].iter().collect();
                        let lower = href.trim().to_ascii_lowercase();
                        // `![alt](src)` — an image is a link whose bracket
                        // carries a `!`. Same URL rules; the `!` is eaten.
                        let is_image = i > 0 && b[i - 1] == '!';
                        let allowed = lower.starts_with("http://") || lower.starts_with("https://")
                            || lower.starts_with("mailto:") || lower.starts_with('#') || lower.starts_with('/');
                        if is_image {
                            if out.ends_with('!') {
                                out.pop();
                            }
                            if allowed && lower.starts_with("http") || lower.starts_with('/') {
                                anchors.push(format!(
                                    "<img src=\"{href}\" alt=\"{label}\" loading=\"lazy\">"
                                ));
                                out.push(LINK_MARK);
                            } else {
                                out.push_str(&label);
                            }
                            i = end + 1;
                            continue;
                        }
                        if allowed {
                            anchors.push(format!("<a href=\"{href}\" target=\"_blank\" rel=\"noopener noreferrer\">{label}</a>"));
                            out.push(LINK_MARK);
                        } else {
                            out.push_str(&label);
                        }
                        i = end + 1;
                        continue;
                    }
                }
            }
        }
        let starts = |pat: &str| {
            let mut end = i;
            for expected in pat.chars() {
                if end >= b.len() || !b[end].eq_ignore_ascii_case(&expected) { return false; }
                end += 1;
            }
            true
        };
        let scheme = if starts("https://") { 8 } else if starts("http://") { 7 } else { 0 };
        let boundary = i == 0 || !(b[i - 1].is_alphanumeric() || matches!(b[i - 1], '_' | '@'));
        // An incomplete [label](URL is kept literal until its closing ')'.
        let unfinished_link = i >= 2 && b[i - 1] == '(' && b[i - 2] == ']';
        if scheme > 0 && boundary && !unfinished_link && i + scheme < b.len()
            && (b[i + scheme].is_ascii_alphanumeric() || b[i + scheme] == '[')
        {
            let mut end = i + scheme;
            while end < b.len() {
                if b[end].is_whitespace() || matches!(b[end], '<' | '>' | '"' | '\'' | '`' | '*') { break; }
                // Escaped HTML delimiters are text boundaries, not part of a URL.
                if b[end] == '&' && ["&quot;", "&lt;", "&gt;"].iter().any(|entity| {
                    let tail: String = b[end..].iter().take(entity.len()).collect();
                    tail == *entity
                }) { break; }
                end += 1;
            }
            while end > i + scheme {
                let last = b[end - 1];
                let unbalanced = match last {
                    ')' => b[i..end].iter().filter(|&&c| c == ')').count() > b[i..end].iter().filter(|&&c| c == '(').count(),
                    ']' => b[i..end].iter().filter(|&&c| c == ']').count() > b[i..end].iter().filter(|&&c| c == '[').count(),
                    '}' => b[i..end].iter().filter(|&&c| c == '}').count() > b[i..end].iter().filter(|&&c| c == '{').count(),
                    _ => false,
                };
                if matches!(last, '.' | ',' | '!' | '?' | ';' | ':') || unbalanced { end -= 1; }
                else { break; }
            }
            let url: String = b[i..end].iter().collect();
            anchors.push(format!("<a href=\"{url}\" target=\"_blank\" rel=\"noopener noreferrer\">{url}</a>"));
            out.push(LINK_MARK);
            i = end;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    (out, anchors)
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
    fn pipe_tables_render_cells_alignment_and_inline_safely() {
        let html = markdown_to_html("Before\n\n| Who | Meaning |\n|:---|---:|\n| **Alice** | <script> \\| `a|b` |\n| Bob |\n\nAfter");
        assert!(html.contains("<th class=\"md-align-left\">Who</th>"));
        assert!(html.contains("<td class=\"md-align-left\"><strong>Alice</strong></td>"));
        assert!(html.contains("<td class=\"md-align-right\">&lt;script&gt; | <code>a|b</code></td>"));
        assert!(html.contains("<td class=\"md-align-right\"></td>"));
        assert!(html.ends_with("</tbody></table></div><p>After</p>"));
    }

    #[test]
    fn malformed_tables_remain_paragraphs_and_fences_remain_code() {
        assert!(!markdown_to_html("| a | b |\n|---|\n| x | y |").contains("<table>"));
        assert!(!markdown_to_html("```\n| a | b |\n|---|---|\n```").contains("<table>"));
    }

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
    fn bare_urls_are_links_and_keep_surrounding_punctuation() {
        let h = markdown_to_html("See (https://example.com/a_(b)?x=1&y=2), or http://localhost:3020/test.");
        assert!(h.contains(r#"href="https://example.com/a_(b)?x=1&amp;y=2""#), "{h}");
        assert!(h.contains("</a>), or "), "{h}");
        assert!(h.contains(r#"href="http://localhost:3020/test""#), "{h}");
        assert!(h.contains("</a>."), "{h}");
    }

    #[test]
    fn urls_in_code_are_literal_and_existing_links_do_not_nest() {
        let h = markdown_to_html("`https://example.com` [docs](https://example.com)\n```\nhttps://example.org\n```");
        assert!(h.contains("<code>https://example.com</code>"), "{h}");
        assert!(h.contains("<pre><code>https://example.org</code></pre>"), "{h}");
        assert_eq!(h.matches("<a href=").count(), 1, "{h}");
        assert!(!h.contains("<a href=\"<a"), "{h}");
    }

    #[test]
    fn autolinks_are_safe_and_work_inside_emphasis() {
        let h = markdown_to_html("**https://example.com/?a=1&b=2** https://example.com/\" onclick=\"alert(1)");
        assert!(h.contains(r#"<strong><a href="https://example.com/?a=1&amp;b=2""#), "{h}");
        assert!(!h.contains("\" onclick=\""), "{h}");
        assert_eq!(h.matches("<a href=").count(), 2, "{h}");
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
        every_prefix_is_sane("see https://example.com/a?x=1&y=2 now");
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

/// What the composer's metric row shows.
///
/// Two different things, deliberately: rates describe the **last** turn
/// (an average over a whole session would flatten to meaninglessness),
/// while totals describe the **session** (what this conversation has cost
/// so far). The icons say which is which — a gauge and a clock read as
/// "right now", arrows and a disk read as "so far".
#[derive(Debug, Default, Clone, Copy)]
pub struct SessionMetrics {
    /// Last turn.
    pub tok_per_sec: f64,
    /// Total wall time of the last turn, tool execution included.
    /// (The backend also reports `ttft_ms`; the row shows duration, which
    /// answers "how long did that take" rather than "when did it start".)
    pub duration_ms: f64,
    /// Session totals.
    pub input_tokens: f64,
    pub output_tokens: f64,
    pub bytes: f64,
}

/// Render the metric row.
pub fn metrics_html(m: &SessionMetrics) -> String {
    let mut chips = String::new();
    let chip = |icon: &str, text: String| {
        format!("<span class=\"metric-chip\"><i class=\"ph ph-{icon}\"></i><span>{text}</span></span>")
    };
    if m.tok_per_sec.is_finite() && m.tok_per_sec > 0.0 {
        // Whole tokens per second: the decimals were noise at this scale,
        // and they changed on every turn without meaning anything.
        chips.push_str(&chip("gauge", format!("{:.0} tok/s", m.tok_per_sec)));
    }
    // `↑ 1,761  ↓ 31 tok` — the arrows already say in and out, so the unit
    // is written once, on the last of the pair. When only one of them has a
    // value it carries the unit itself, so the row never reads `↑ 1,761`
    // with no indication of what was counted.
    let has_in = m.input_tokens.is_finite() && m.input_tokens > 0.0;
    let has_out = m.output_tokens.is_finite() && m.output_tokens > 0.0;
    if has_in {
        let unit = if has_out { "" } else { " tok" };
        chips.push_str(&chip(
            "arrow-up",
            format!("{}{unit}", fmt_count(m.input_tokens)),
        ));
    }
    if has_out {
        chips.push_str(&chip(
            "arrow-down",
            format!("{} tok", fmt_count(m.output_tokens)),
        ));
    }
    if m.bytes.is_finite() && m.bytes > 0.0 {
        chips.push_str(&chip("database", format!("{} bytes", fmt_count(m.bytes))));
    }
    if m.duration_ms.is_finite() && m.duration_ms > 0.0 {
        // `0.330s elapsed` — the same `{:.3}s` the per-card gutter uses,
        // with the word that says what the number is.
        chips.push_str(&chip(
            "timer",
            format!("{} elapsed", format_elapsed(m.duration_ms)),
        ));
    }
    chips
}

/// Byte counts, kept short so the chip row stays one line.
/// Byte counts, kept short so the chip row stays one line.
/// Thousands separators below 10k, then k/M — a 4-digit count is still
/// readable, a 7-digit one is not.
/// Group thousands, then abbreviate: `1,771` / `12.4k` / `3.1M`.
pub fn fmt_count(n: f64) -> String {
    let n = n.max(0.0);
    if n < 10_000.0 {
        let i = n as u64;
        let s = i.to_string();
        // group thousands: 1771 -> 1,771
        let mut out = String::new();
        for (idx, ch) in s.chars().enumerate() {
            if idx > 0 && (s.len() - idx) % 3 == 0 {
                out.push(',');
            }
            out.push(ch);
        }
        out
    } else if n < 1_000_000.0 {
        format!("{:.1}k", n / 1000.0)
    } else {
        format!("{:.1}M", n / 1_000_000.0)
    }
}

#[cfg(test)]
mod metric_tests {
    use super::*;

    #[test]
    fn counts_stay_short() {
        assert_eq!(fmt_count(0.0), "0");
        assert_eq!(fmt_count(871.0), "871");
        assert_eq!(fmt_count(1771.0), "1,771");
        assert_eq!(fmt_count(9999.0), "9,999");
        assert_eq!(fmt_count(10_000.0), "10.0k");
        assert_eq!(fmt_count(1_500_000.0), "1.5M");
    }

    #[test]
    fn a_lone_token_count_still_carries_the_unit() {
        let only_in = metrics_html(&SessionMetrics { input_tokens: 42.0, ..Default::default() });
        assert!(only_in.contains("42 tok"), "{only_in}");
        let only_out = metrics_html(&SessionMetrics { output_tokens: 7.0, ..Default::default() });
        assert!(only_out.contains("7 tok"), "{only_out}");
    }

    #[test]
    fn the_timer_reads_as_a_sentence() {
        let m = SessionMetrics { duration_ms: 330.0, ..Default::default() };
        assert!(metrics_html(&m).contains("0.330s elapsed"));
        let m = SessionMetrics { duration_ms: 1304.0, ..Default::default() };
        assert!(metrics_html(&m).contains("1.304s elapsed"));
    }

    #[test]
    fn rates_are_last_turn_and_totals_are_the_session() {
        let m = SessionMetrics { tok_per_sec: 267.07, duration_ms: 412.0,
                                 input_tokens: 5312.0, output_tokens: 193.0, bytes: 1771.0 };
        let h = metrics_html(&m);
        assert!(h.contains("267 tok/s"), "{h}");
        assert!(!h.contains("267.07"), "no decimals: {h}");
        assert!(h.contains("ph-arrow-up") && h.contains(">5,312<"), "{h}");
        assert!(h.contains("ph-arrow-down") && h.contains("193 tok"), "{h}");
        assert_eq!(h.matches(" tok<").count(), 1, "the unit is written once: {h}");
        assert!(!h.contains(" in<") && !h.contains(" out<"), "{h}");
        assert!(h.contains("1,771 bytes"), "{h}");
        assert!(h.contains("0.412s elapsed"), "{h}");
        assert!(!h.contains(" tps"), "renamed to tok/s: {h}");
    }

    #[test]
    fn a_zero_metric_is_omitted_rather_than_shown_as_zero() {
        let h = metrics_html(&SessionMetrics { tok_per_sec: 12.0, ..Default::default() });
        assert!(h.contains("tok/s"));
        assert!(!h.contains("in") || !h.contains("arrow-up"), "{h}");
        assert!(!h.contains("bytes"), "{h}");
    }
}

/// Truncate to `n` characters with an ellipsis, counting characters rather
/// than bytes so a multi-byte glyph is never cut in half.
#[cfg(test)]
pub fn clip(text: &str, n: usize) -> String {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars: Vec<char> = flat.chars().collect();
    if chars.len() <= n {
        flat
    } else {
        format!("{}…", chars[..n].iter().collect::<String>())
    }
}

#[cfg(test)]
mod clip_tests {
    use super::clip;

    #[test]
    fn clipping_counts_characters_not_bytes() {
        assert_eq!(clip("hello", 10), "hello");
        assert_eq!(clip("hello world", 5), "hello…");
        // Newlines collapse: a summary line is one line.
        assert_eq!(clip("a\n\nb", 10), "a b");
        // Four 3-byte characters: a byte-based cut would split one.
        assert_eq!(clip("哦哦哦哦", 2), "哦哦…");
    }
}

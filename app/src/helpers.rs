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



pub fn markdown_to_html(src: &str) -> String {
    let mut out = String::new();
    let lines: Vec<&str> = src.split('\n').collect();
    let mut i = 0;
    let mut list: Option<&'static str> = None;
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

        if trimmed.is_empty() {
            close_para!();
            close_list!();
            i += 1;
            continue;
        }

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

        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            close_para!();
            close_list!();
            out.push_str("<hr>");
            i += 1;
            continue;
        }

        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) && trimmed.chars().nth(hashes) == Some(' ') {
            close_para!();
            close_list!();
            let text = inline(&trimmed[hashes + 1..]);
            out.push_str(&format!("<h{hashes}>{text}</h{hashes}>"));
            i += 1;
            continue;
        }

        if let Some(q) = trimmed.strip_prefix("> ").or_else(|| trimmed.strip_prefix(">")) {
            close_para!();
            close_list!();
            out.push_str(&format!("<blockquote>{}</blockquote>", inline(q)));
            i += 1;
            continue;
        }

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

        close_list!();
        para.push(inline(trimmed));
        i += 1;
    }
    close_para!();
    close_list!();
    out
}

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

fn ordered_item(line: &str) -> Option<&str> {
    let digits = line.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    let rest = &line[digits..];
    let rest = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") "))?;
    Some(rest)
}

fn inline(text: &str) -> String {
    let escaped = esc_html(text);
    let (masked, spans) = mask_code(&escaped);
    let (s, anchors) = links(&masked);
    let s = wrap(&s, "**", "strong");
    let s = s.replace("**", "\u{1}");
    let s = wrap(&s, "~~", "del");
    let s = wrap(&s, "*", "em");
    let s = s.replace('\u{1}', "**");
    let s = unmask(&s, LINK_MARK, &anchors);
    unmask(&s, CODE_MARK, &spans)
}

const CODE_MARK: char = '\u{0}';

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
        if i + 1 <= rest.len() - 1 || rest.len() > i + 1 {
            spans.push(format!("<code>{}</code>", rest[i]));
            out.push(CODE_MARK);
            if i + 1 < rest.len() {
                out.push_str(rest[i + 1]);
            }
            i += 2;
        } else {
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
        let unfinished_link = i >= 2 && b[i - 1] == '(' && b[i - 2] == ']';
        if scheme > 0 && boundary && !unfinished_link && i + scheme < b.len()
            && (b[i + scheme].is_ascii_alphanumeric() || b[i + scheme] == '[')
        {
            let mut end = i + scheme;
            while end < b.len() {
                if b[end].is_whitespace() || matches!(b[end], '<' | '>' | '"' | '\'' | '`' | '*') { break; }
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

#[derive(Debug, Default, Clone, Copy)]
pub struct SessionMetrics {
    pub tok_per_sec: f64,
    pub duration_ms: f64,
    pub input_tokens: f64,
    pub output_tokens: f64,
    pub bytes: f64,
}

pub fn metrics_html(m: &SessionMetrics) -> String {
    let mut chips = String::new();
    let chip = |icon: &str, text: String| {
        format!("<span class=\"metric-chip\"><i class=\"ph ph-{icon}\"></i><span>{text}</span></span>")
    };
    if m.tok_per_sec.is_finite() && m.tok_per_sec > 0.0 {
        chips.push_str(&chip("gauge", format!("{:.0} tok/s", m.tok_per_sec)));
    }
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
        chips.push_str(&chip(
            "timer",
            format!("{} elapsed", format_elapsed(m.duration_ms)),
        ));
    }
    chips
}

pub fn fmt_count(n: f64) -> String {
    let n = n.max(0.0);
    if n < 10_000.0 {
        let i = n as u64;
        let s = i.to_string();
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

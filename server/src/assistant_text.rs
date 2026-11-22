//! Remove transcript-style speaker prefixes at the chat boundary.
use regex::Regex;
use serde_json::{Value, json};
use std::sync::OnceLock;

fn marker() -> &'static Regex {
    static MARKER: OnceLock<Regex> = OnceLock::new();
    MARKER.get_or_init(|| Regex::new(r"^(?:[01]?[0-9]|2[0-3]):[0-5][0-9]|^\[[^\[\]\r\n]{1,64}\]").unwrap())
}

// A marker can be split across arbitrarily many deltas. Delay only while the
// current text could still become one, and cap the hold to avoid stalling.
fn could_be_marker(s: &str) -> bool {
    if let Some(rest) = s.strip_prefix('[') {
        return !rest.contains(['[', ']', '\n', '\r']) && rest.chars().count() <= 64;
    }
    let bytes = s.as_bytes();
    match bytes {
        [] => true,
        [a] => a.is_ascii_digit() && *a <= b'2',
        [a, b] => (a.is_ascii_digit() && b.is_ascii_digit() && (*a < b'2' || (*a == b'2' && *b <= b'3'))) || (a.is_ascii_digit() && *b == b':'),
        [a, b, c] => (a.is_ascii_digit() && *b == b':' && (b'0'..=b'5').contains(c)) || (a.is_ascii_digit() && b.is_ascii_digit() && *c == b':' && (*a < b'2' || (*a == b'2' && *b <= b'3'))),
        [a, b, c, d] => a.is_ascii_digit() && b.is_ascii_digit() && *c == b':' && (b'0'..=b'5').contains(d) && (*a < b'2' || (*a == b'2' && *b <= b'3')),
        _ => false,
    }
}

/// None means more input is needed to decide whether there is a prefix.
fn clean(text: &str, finished: bool) -> Option<String> {
    let mut rest = text;
    let mut removed = false;
    loop {
        let trimmed = rest.trim_start_matches(char::is_whitespace);
        if trimmed.is_empty() {
            return if finished { Some(if removed { String::new() } else { text.to_string() }) } else if text.len() > 256 { Some(text.to_string()) } else { None };
        }
        if let Some(found) = marker().find(trimmed) {
            let after = &trimmed[found.end()..];
            if after.is_empty() && !finished { return None; }
            // Preserve Markdown links, reference definitions, and ordinary
            // words that merely start with a time-like sequence.
            if !after.is_empty() && !after.starts_with(char::is_whitespace) {
                return Some(if removed { rest.trim_start_matches(char::is_whitespace).to_string() } else { text.to_string() });
            }
            removed = true;
            rest = after;
        } else if !finished && could_be_marker(trimmed) && text.len() <= 256 {
            return None;
        } else {
            return Some(if removed { trimmed.to_string() } else { text.to_string() });
        }
    }
}

pub fn normalize_assistant_messages(value: &mut Value) {
    match value {
        Value::Object(obj) => {
            if obj.get("kind").and_then(Value::as_str) == Some("assistant") {
                if let Some(Value::String(text)) = obj.get_mut("text") {
                    *text = clean(text, true).unwrap_or_default();
                }
            }
            for value in obj.values_mut() { normalize_assistant_messages(value); }
        }
        Value::Array(values) => for value in values { normalize_assistant_messages(value); },
        _ => {}
    }
}

#[derive(Default)]
pub struct StreamFilter {
    pending: String,
    template: Option<Value>,
    decided: bool,
}
impl StreamFilter {
    pub fn finish(&mut self) -> Option<Value> {
        let mut delta = self.template.take()?;
        let text = clean(&self.pending, true).unwrap_or_default();
        self.pending.clear();
        self.decided = false;
        if text.is_empty() { return None; }
        delta["text"] = Value::String(text);
        Some(delta)
    }

    pub fn process(&mut self, row: &Value) -> Vec<Value> {
        let kind = row.get("type").and_then(Value::as_str).unwrap_or("");
        if kind == "assistant_delta" {
            if self.decided { return vec![row.clone()]; }
            self.pending.push_str(row.get("text").and_then(Value::as_str).unwrap_or(""));
            self.template = Some(row.clone());
            if let Some(text) = clean(&self.pending, false) {
                self.pending.clear();
                self.decided = true;
                let mut output = self.template.take().unwrap_or_else(|| json!({"type":"assistant_delta"}));
                output["text"] = Value::String(text);
                return vec![output];
            }
            return vec![];
        }
        let assistant_card = kind == "card" && row.pointer("/card/kind").and_then(Value::as_str) == Some("assistant");
        let boundary = assistant_card || kind == "done" || (kind == "card" && row.pointer("/card/kind").and_then(Value::as_str).is_some_and(|k| k == "tool_call" || k == "tool_result"));
        let mut out = Vec::new();
        if boundary {
            if let Some(delta) = self.finish() { out.push(delta); }
            self.pending.clear();
            self.decided = false;
        }
        let mut row = row.clone();
        if assistant_card { normalize_assistant_messages(&mut row); }
        out.push(row);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strips_only_leading_speaker_and_time_markers() {
        for (input, expected) in [
            ("10:36 [Bob] [videoguy] He says hello", "He says hello"),
            ("[user] Hello", "Hello"), ("09:10 Hello", "Hello"),
            ("Hello 10:36 [Bob]", "Hello 10:36 [Bob]"),
            ("[link](https://example.com)", "[link](https://example.com)"),
            ("[ref]: https://example.com", "[ref]: https://example.com"),
            ("12:30pm starts", "12:30pm starts"), ("2026 was a year", "2026 was a year"),
        ] { assert_eq!(clean(input, true).as_deref(), Some(expected), "{input}"); }
    }
    #[test]
    fn streaming_handles_every_split_and_resets_at_card() {
        let full = "10:36 [Bob] [videoguy] He says hello";
        for split in 0..=full.len() {
            let mut filter = StreamFilter::default();
            let mut text = String::new();
            for part in [&full[..split], &full[split..]] {
                for row in filter.process(&json!({"type":"assistant_delta","text":part})) { text.push_str(row["text"].as_str().unwrap()); }
            }
            let rows = filter.process(&json!({"type":"card","card":{"kind":"assistant","text":full}}));
            for row in &rows { if row["type"] == "assistant_delta" { text.push_str(row["text"].as_str().unwrap()); } }
            assert_eq!(text, "He says hello", "split {split}");
            assert_eq!(rows.last().unwrap()["card"]["text"], "He says hello");
            assert_eq!(filter.process(&json!({"type":"assistant_delta","text":"Normal"}))[0]["text"], "Normal");
        }
    }
    #[test]
    fn one_character_chunks_and_partial_plain_text_are_preserved() {
        for (input, expected) in [("10:36 [Bob] [videoguy] Hi", "Hi"), ("[link](url)", "[link](url)"), ("12:30pm", "12:30pm"), ("123 apples", "123 apples")] {
            let mut filter = StreamFilter::default();
            let mut result = String::new();
            for ch in input.chars() {
                for row in filter.process(&json!({"type":"assistant_delta", "text":ch.to_string()})) {
                    result.push_str(row["text"].as_str().unwrap());
                }
            }
            if let Some(row) = filter.finish() { result.push_str(row["text"].as_str().unwrap()); }
            assert_eq!(result, expected, "{input}");
        }
    }
    #[test]
    fn non_assistant_text_is_untouched() {
        let mut data = json!({"messages":[{"kind":"user","text":"10:36 [Bob] hi"},{"kind":"assistant","text":"10:36 [Bob] hello"}]});
        normalize_assistant_messages(&mut data);
        assert_eq!(data["messages"][0]["text"], "10:36 [Bob] hi");
        assert_eq!(data["messages"][1]["text"], "hello");
    }
}

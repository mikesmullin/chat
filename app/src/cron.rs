use wasm_bindgen::JsValue;

#[derive(Default, Clone, Debug)]
pub struct Prenormalize {
    pub original_parts: Vec<String>,
    pub parts: Vec<String>,
    pub days_anded: Option<bool>,
}

#[derive(Default, Clone, Debug)]
pub struct Normalized {
    pub days_anded: Option<bool>,
    pub minutes: Vec<i64>,
    pub hours: Vec<i64>,
    pub dates: Vec<i64>,
    pub months: Vec<i64>,
    pub weekdays: Vec<i64>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub empty: bool,
}

#[derive(Default, Clone, Debug)]
pub struct Described {
    pub full: String,
    pub special: Option<String>,
}

const ALIASES: &[(&str, &[&str])] = &[
    ("@yearly", &["0", "0", "1", "1", "*"]),
    ("@annually", &["0", "0", "1", "1", "*"]),
    ("@monthly", &["0", "0", "1", "*", "*"]),
    ("@weekly", &["0", "0", "*", "*", "0"]),
    ("@daily", &["0", "0", "*", "*", "*"]),
    ("@midnight", &["0", "0", "*", "*", "*"]),
    ("@hourly", &["0", "*", "*", "*", "*"]),
];

const MONTHS: &[(&str, &str)] = &[
    ("jan", "1"), ("feb", "2"), ("mar", "3"), ("apr", "4"),
    ("may", "5"), ("jun", "6"), ("jul", "7"), ("aug", "8"),
    ("sep", "9"), ("oct", "10"), ("nov", "11"), ("dec", "12"),
];

const WEEKDAYS: &[(&str, &str)] = &[
    ("sun", "0"), ("mon", "1"), ("tue", "2"), ("wed", "3"),
    ("thu", "4"), ("fri", "5"), ("sat", "6"),
];

const MONTH_NAMES: &[Option<&str>] = &[
    None,
    Some("January"), Some("February"), Some("March"), Some("April"),
    Some("May"), Some("June"), Some("July"), Some("August"),
    Some("September"), Some("October"), Some("November"), Some("December"),
];

const WEEKDAY_NAMES: &[Option<&str>] = &[
    Some("Sunday"), Some("Monday"), Some("Tuesday"), Some("Wednesday"),
    Some("Thursday"), Some("Friday"), Some("Saturday"), Some("Sunday"),
];

const EMPTY_NAMES: &[Option<&str>] = &[];

fn is_boundary_char(b: u8) -> bool {
    matches!(b, b' ' | b',' | b'-' | b'/')
}

fn is_star_boundary_char(b: u8) -> bool {
    matches!(b, b',' | b'-' | b'/')
}

fn apply_name_once(text: &str, key: &str, value: &str) -> String {
    let bytes = text.as_bytes();
    let key_len = key.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        let boundary_before = i == 0 || is_boundary_char(bytes[i - 1]);
        if boundary_before
            && i + key_len <= bytes.len()
            && text.get(i..i + key_len).map(|s| s.eq_ignore_ascii_case(key)).unwrap_or(false)
        {
            let boundary_after = i + key_len >= bytes.len() || is_boundary_char(bytes[i + key_len]);
            if boundary_after {
                out.push_str(value);
                i += key_len;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn expand_names(text: &str, map: &[(&str, &str)]) -> String {
    let mut result = text.to_string();
    for (key, value) in map {
        result = apply_name_once(&result, key, value);
        result = apply_name_once(&result, key, value);
    }
    result
}

pub fn prenormalize(input: &str) -> Prenormalize {
    let tokens: Vec<String> = input.split_whitespace().map(|t| t.to_string()).collect();
    if tokens.len() == 1 && tokens[0] == "@reboot" {
        return Prenormalize { original_parts: tokens, parts: Vec::new(), days_anded: None };
    }
    let expanded: Vec<String> = if tokens.len() == 1 {
        match ALIASES.iter().find(|(k, _)| *k == tokens[0]) {
            Some((_, parts)) => parts.iter().map(|s| s.to_string()).collect(),
            None => vec![tokens[0].clone()],
        }
    } else {
        tokens.clone()
    };
    let parts = expanded
        .iter()
        .enumerate()
        .map(|(i, part)| match i {
            3 => expand_names(part, MONTHS),
            4 => expand_names(part, WEEKDAYS),
            _ => part.clone(),
        })
        .collect::<Vec<_>>();
    let d2 = parts.get(2).map(|p| !p.is_empty() && p.starts_with('*')).unwrap_or(false);
    let d4 = parts.get(4).map(|p| !p.is_empty() && p.starts_with('*')).unwrap_or(false);
    Prenormalize { original_parts: tokens, parts, days_anded: Some(d2 || d4) }
}

fn fix_star_slash_one(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'*'
            && i + 2 < bytes.len()
            && bytes[i + 1] == b'/'
            && bytes[i + 2] == b'1'
            && (i + 3 >= bytes.len() || !bytes[i + 3].is_ascii_digit())
        {
            out.push('*');
            i += 3;
        } else {
            let ch = text[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn expand_star_once(text: &str, replacement: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'*' {
            let boundary_before = i == 0 || is_star_boundary_char(bytes[i - 1]);
            let boundary_after = i + 1 >= bytes.len() || is_star_boundary_char(bytes[i + 1]);
            if boundary_before && boundary_after {
                out.push_str(replacement);
                i += 1;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn expand_star(text: &str, replacement: &str) -> String {
    let mut current = text.to_string();
    for _ in 0..2 {
        current = expand_star_once(&current, replacement);
    }
    current
}

#[derive(Clone, PartialEq, Debug)]
enum Token {
    N(i64),
    S(String),
}

fn match_tokens(e: &str) -> Vec<Token> {
    let bytes = e.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let num = e[start..i].parse::<i64>().unwrap_or(0);
            out.push(Token::N(num));
        } else {
            let ch = e[i..].chars().next().unwrap();
            out.push(Token::S(ch.to_string()));
            i += ch.len_utf8();
        }
    }
    out
}

fn range(from: i64, to: i64, step: i64) -> Vec<i64> {
    let mut out = Vec::new();
    let mut v = from;
    while v <= to {
        out.push(v);
        v += step;
    }
    out
}

#[derive(Default)]
struct Piece {
    list: Vec<i64>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

fn parse_piece(piece: &str, max: i64) -> Piece {
    let tokens = match_tokens(piece);
    let first = match tokens.first() {
        Some(Token::N(n)) => *n,
        _ => return Piece { errors: vec!["invalid part".into()], ..Default::default() },
    };
    if tokens.len() == 1 {
        return Piece { list: vec![first], ..Default::default() };
    }
    if tokens.len() == 3
        && tokens[1] == Token::S("/".into())
        && matches!(tokens[2], Token::N(step) if step >= 1)
    {
        let Token::N(step) = tokens[2] else { return Piece::default() };
        return Piece { list: range(first, max, step), warnings: vec!["nonstandard".into()], ..Default::default() };
    }
    if tokens.len() == 3
        && tokens[1] == Token::S("-".into())
        && matches!(tokens[2], Token::N(end) if end >= first)
    {
        let Token::N(end) = tokens[2] else { return Piece::default() };
        return Piece { list: range(first, end, 1), ..Default::default() };
    }
    if tokens.len() == 5
        && tokens[1] == Token::S("-".into())
        && matches!(tokens[2], Token::N(end) if end >= first)
        && tokens[3] == Token::S("/".into())
        && matches!(tokens[4], Token::N(step) if step >= 1)
    {
        let Token::N(end) = tokens[2] else { return Piece::default() };
        let Token::N(step) = tokens[4] else { return Piece::default() };
        return Piece { list: range(first, end, step), ..Default::default() };
    }
    Piece { errors: vec!["invalid part".into()], ..Default::default() }
}

fn dedup_i64(items: Vec<i64>) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::new();
    for item in items {
        if !out.contains(&item) {
            out.push(item);
        }
    }
    out
}

fn dedup_str(items: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in items {
        if !out.contains(&item) {
            out.push(item);
        }
    }
    out
}

struct GResult {
    list: Vec<i64>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn g(text: &str, max: i64) -> GResult {
    let mut list = Vec::new();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for piece in text.split(',') {
        let r = parse_piece(piece, max);
        list.extend(r.list);
        warnings.extend(r.warnings);
        errors.extend(r.errors);
    }
    let mut list = dedup_i64(list);
    list.sort();
    let errors = dedup_str(errors);
    let warnings = dedup_str(warnings);
    GResult { list, errors, warnings }
}

fn out_of_range(list: &[i64], min: i64, max: i64) -> bool {
    if list.is_empty() {
        return false;
    }
    list[0] < min || list[list.len() - 1] > max
}

fn valid_chars(text: &str) -> bool {
    text.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '/' || c == ',')
}

fn has_non_digit(text: &str) -> bool {
    text.chars().any(|c| !c.is_ascii_digit())
}

pub fn normalize(pre: &Prenormalize) -> Normalized {
    let parts: Vec<String> = pre.parts.iter().map(|p| fix_star_slash_one(p)).collect();
    if parts.is_empty() && !pre.original_parts.is_empty() {
        return Normalized { empty: true, ..Default::default() };
    }
    let mut n = Normalized { days_anded: pre.days_anded, ..Default::default() };
    if parts.len() != 5 {
        n.errors.push("fields".into());
    }

    match parts.get(0) {
        Some(p) if !p.is_empty() => {
            let norm = expand_star(p, "0-59");
            let r = g(&norm, 59);
            n.minutes = r.list;
            if !r.errors.is_empty() || out_of_range(&n.minutes, 0, 59) || !valid_chars(&norm) {
                n.minutes = Vec::new();
                n.errors.push("minutes".into());
            }
            if !r.warnings.is_empty() {
                n.warnings.push("minutes".into());
            }
        }
        Some(_) => {}
        None => n.errors.push("minutes".into()),
    }

    match parts.get(1) {
        Some(p) if !p.is_empty() => {
            let norm = expand_star(p, "0-23");
            let r = g(&norm, 23);
            n.hours = r.list;
            if !r.errors.is_empty() || out_of_range(&n.hours, 0, 23) || !valid_chars(&norm) {
                n.hours = Vec::new();
                n.errors.push("hours".into());
            }
            if !r.warnings.is_empty() {
                n.warnings.push("hours".into());
            }
        }
        Some(_) => {}
        None => n.errors.push("hours".into()),
    }

    match parts.get(2) {
        Some(p) if !p.is_empty() => {
            let norm = expand_star(p, "1-31");
            let r = g(&norm, 31);
            n.dates = r.list;
            if !r.errors.is_empty() || out_of_range(&n.dates, 1, 31) || !valid_chars(&norm) {
                n.dates = Vec::new();
                n.errors.push("dates".into());
            }
            if !r.warnings.is_empty() {
                n.warnings.push("dates".into());
            }
        }
        Some(_) => {}
        None => n.errors.push("dates".into()),
    }

    match parts.get(3) {
        Some(p) if !p.is_empty() => {
            let norm = expand_star(p, "1-12");
            let original = pre.original_parts.get(3).map(|s| s.as_str());
            let r = g(&norm, 12);
            n.months = r.list;
            if !r.errors.is_empty() || out_of_range(&n.months, 1, 12) || !valid_chars(&norm) {
                n.months = Vec::new();
                n.errors.push("months".into());
            }
            let long_name = original
                .map(|l| p != l && l.len() > 3 && has_non_digit(l))
                .unwrap_or(false);
            if !r.warnings.is_empty() || long_name {
                n.warnings.push("months".into());
            }
        }
        Some(_) => {}
        None => n.errors.push("months".into()),
    }

    match parts.get(4) {
        Some(p) if !p.is_empty() => {
            let norm = expand_star(p, "0-6");
            let original = pre.original_parts.get(4).map(|s| s.as_str());
            let r = g(&norm, 7);
            let mut weekdays: Vec<i64> = r.list.iter().map(|v| if *v == 7 { 0 } else { *v }).collect();
            weekdays = dedup_i64(weekdays);
            weekdays.sort();
            n.weekdays = weekdays;
            if !r.errors.is_empty() || out_of_range(&n.weekdays, 0, 6) || !valid_chars(&norm) {
                n.weekdays = Vec::new();
                n.errors.push("weekdays".into());
            }
            let eleven_seven = r.list.contains(&7);
            let long_name = original
                .map(|l| p != l && l.len() > 3 && has_non_digit(l))
                .unwrap_or(false);
            if !r.warnings.is_empty() || eleven_seven || long_name {
                n.warnings.push("weekdays".into());
            }
        }
        Some(_) => {}
        None => n.errors.push("weekdays".into()),
    }

    n
}

fn ordinal(e: i64) -> String {
    let q = if e > 20 { e % 10 } else { e };
    match q {
        1 => format!("{e}st"),
        2 => format!("{e}nd"),
        3 => format!("{e}rd"),
        _ => format!("{e}th"),
    }
}

fn name_for(names: &[Option<&str>], index: i64) -> String {
    let i = index as usize;
    if i < names.len() {
        if let Some(n) = names[i] {
            return n.to_string();
        }
    }
    index.to_string()
}

fn describe_segment(e: &str, unit: &str, names: &[Option<&str>], max: i64) -> String {
    if e == "*" {
        return format!("every {unit}");
    }
    let tokens = match_tokens(e);
    let first = match tokens.first() {
        Some(Token::N(n)) => *n,
        _ => {
            if tokens.len() == 3
                && tokens[1] == Token::S("/".into())
                && matches!(tokens[2], Token::N(step) if step >= 1)
                && tokens.first() == Some(&Token::S("*".into()))
            {
                let Token::N(step) = tokens[2] else { return String::new() };
                return format!("every {} {unit}", ordinal(step));
            }
            return String::new();
        }
    };
    if tokens.len() == 1 {
        return name_for(names, first);
    }
    if tokens.len() == 3
        && tokens[1] == Token::S("/".into())
        && matches!(tokens[2], Token::N(step) if step >= 1)
    {
        let Token::N(step) = tokens[2] else { return String::new() };
        return format!("every {} {unit} from {} through {}", ordinal(step), name_for(names, first), name_for(names, max));
    }
    if tokens.len() == 3
        && tokens[1] == Token::S("-".into())
        && matches!(tokens[2], Token::N(end) if end >= first)
    {
        let Token::N(end) = tokens[2] else { return String::new() };
        return format!("every {unit} from {} through {}", name_for(names, first), name_for(names, end));
    }
    if tokens.len() == 5
        && tokens[1] == Token::S("-".into())
        && matches!(tokens[2], Token::N(end) if end >= first)
        && tokens[3] == Token::S("/".into())
        && matches!(tokens[4], Token::N(step) if step >= 1)
    {
        let Token::N(end) = tokens[2] else { return String::new() };
        let Token::N(step) = tokens[4] else { return String::new() };
        return format!("every {} {unit} from {} through {}", ordinal(step), name_for(names, first), name_for(names, end));
    }
    String::new()
}

fn describe_join(e: &str, unit: &str, names: &[Option<&str>], max: i64, no_prefix: bool) -> String {
    let pieces: Vec<&str> = e.split(',').collect();
    let parts: Vec<String> = pieces.iter().map(|p| describe_segment(p, unit, names, max)).collect();
    let joined = match parts.len() {
        0 => String::new(),
        1 => parts[0].clone(),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let last = parts.last().unwrap().clone();
            format!("{}, and {}", parts[..parts.len() - 1].join(", "), last)
        }
    };
    let mut out = if no_prefix { joined } else { format!("{unit} {joined}") };
    out = replace_once(out, "every 1st", "every");
    out = replace_once(out, &format!("{unit} every"), "every");
    out = replace_once(out, &format!(", {unit}"), ", ");
    out = replace_once(out, &format!(", and {unit}"), ", and ");
    out
}

fn replace_once(text: String, needle: &str, replacement: &str) -> String {
    if let Some(pos) = text.find(needle) {
        let mut out = text;
        out.replace_range(pos..pos + needle.len(), replacement);
        out
    } else {
        text
    }
}

fn matches_time_pattern(c: &str) -> bool {
    let mut n = 0;
    for ch in c.chars() {
        if !ch.is_ascii_digit() {
            return false;
        }
        n += 1;
    }
    n >= 1 && n <= 2
}

pub fn describe(pre: &Prenormalize) -> Described {
    if pre.original_parts.first().map(|p| p == "@reboot").unwrap_or(false) {
        return Described { full: "After rebooting.".into(), special: Some("After rebooting.".into()) };
    }
    let parts = &pre.parts;

    let o2 = parts.get(2).map(|p| p.as_str());
    let o3 = parts.get(3).map(|p| p.as_str());
    let o4 = parts.get(4).map(|p| p.as_str());

    let a = match o2 {
        Some(p) if p != "*" => format!("on {}", describe_join(p, "day-of-month", EMPTY_NAMES, 31, false)),
        _ => String::new(),
    };
    let u = match o3 {
        Some(p) if p != "*" => format!("in {}", describe_join(p, "month", MONTH_NAMES, 12, true)),
        _ => String::new(),
    };
    let i = match o4 {
        Some(p) if p != "*" => format!("on {}", describe_join(p, "day-of-week", WEEKDAY_NAMES, 7, true)),
        _ => String::new(),
    };

    let s = if !a.is_empty() && !i.is_empty() {
        if pre.days_anded.unwrap_or(false) { "if it's".to_string() } else { "and".to_string() }
    } else {
        String::new()
    };

    let pad = |s: &str| -> String {
        let doubled = format!("0{s}");
        doubled[doubled.len() - 2..].to_string()
    };

    let time_d = match (parts.get(0), parts.get(1)) {
        (Some(c), Some(l)) if matches_time_pattern(c) && matches_time_pattern(l) => {
            Some([pad(c), pad(l)])
        }
        _ => None,
    };

    if let Some([minutes, hours]) = time_d {
        let full = format!("At {hours}:{minutes}");
        let full = format!("{full} {a} {s} {i} {u}");
        let full = collapse_spaces(&full);
        let full = format!("{}.", full.trim());
        return Described { full, special: None };
    }

    let minute_piece = parts.get(0).map(|s| s.as_str()).unwrap_or("*");
    let f = describe_join(minute_piece, "minute", EMPTY_NAMES, 59, false);
    let hour_piece = parts.get(1).map(|s| s.as_str()).unwrap_or("*");
    let h = if hour_piece == "*" {
        String::new()
    } else {
        format!("past {}", describe_join(hour_piece, "hour", EMPTY_NAMES, 23, false))
    };
    let full = format!("At {f} {h} {a} {s} {i} {u}");
    let full = collapse_spaces(&full);
    let full = format!("{}.", full.trim());
    Described { full, special: None }
}

fn collapse_spaces(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn js_date_utc(year: i64, month: i64, day: i64, hour: i64, minute: i64) -> js_sys::Date {
    let date_fn = js::get(&js::global(), "Date");
    let millis = js::call(
        &date_fn,
        "UTC",
        &[
            JsValue::from_f64(year as f64),
            JsValue::from_f64(month as f64),
            JsValue::from_f64(day as f64),
            JsValue::from_f64(hour as f64),
            JsValue::from_f64(minute as f64),
        ],
    );
    js_sys::Date::new(&JsValue::from_f64(millis.as_f64().unwrap_or(0.0)))
}

use rustex1_kit::js;

pub fn next_at(schedule: &Normalized, base_ms: f64) -> Option<js_sys::Date> {
    if schedule.empty {
        return None;
    }
    if schedule.minutes.is_empty()
        || schedule.hours.is_empty()
        || schedule.dates.is_empty()
        || schedule.months.is_empty()
        || schedule.weekdays.is_empty()
    {
        return None;
    }

    let mut base = js_sys::Date::new(&JsValue::from_f64(base_ms));
    let ms = base.get_utc_milliseconds();
    if ms != 0 {
        base = js_sys::Date::new(&JsValue::from_f64(base.get_time() + (1000.0 - ms as f64)));
    }
    let sec = base.get_utc_seconds();
    if sec != 0 {
        base = js_sys::Date::new(&JsValue::from_f64(base.get_time() + 1000.0 * (60.0 - sec as f64)));
    }

    let days_anded = schedule.days_anded.unwrap_or(false);
    let mut counter = 1;
    let mut n = base;
    loop {
        if counter > 127 {
            return None;
        }
        let year = n.get_utc_full_year();
        let month = n.get_utc_month() + 1;
        if !schedule.months.contains(&(month as i64)) {
            n = js_date_utc(year as i64, month as i64, 1, 0, 0);
            counter += 1;
            continue;
        }
        let date = n.get_utc_date();
        let day_of_week = n.get_utc_day();
        let s = schedule.dates.contains(&(date as i64));
        let c = schedule.weekdays.contains(&(day_of_week as i64));
        if (days_anded && (!s || !c)) || (!days_anded && !s && !c) {
            n = js_date_utc(year as i64, month as i64 - 1, date as i64 + 1, 0, 0);
            counter += 1;
            continue;
        }
        let hour = n.get_utc_hours();
        if !schedule.hours.contains(&(hour as i64)) {
            n = js_date_utc(year as i64, month as i64 - 1, date as i64, hour as i64 + 1, 0);
            counter += 1;
            continue;
        }
        let minute = n.get_utc_minutes();
        if schedule.minutes.contains(&(minute as i64)) {
            return Some(n);
        }
        n = js_date_utc(year as i64, month as i64 - 1, date as i64, hour as i64, minute as i64 + 1);
        counter += 1;
    }
}

pub fn format_utc(date: &js_sys::Date) -> String {
    let year = date.get_utc_full_year();
    let month = date.get_utc_month() + 1;
    let day = date.get_utc_date();
    let hour = date.get_utc_hours();
    let minute = date.get_utc_minutes();
    let sec = date.get_utc_seconds();
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, minute, sec)
}

pub fn epoch_seconds(date: &js_sys::Date) -> i64 {
    (date.get_time() / 1000.0).floor() as i64
}

pub fn next_line(schedule: &Normalized, base_ms: f64) -> Option<String> {
    let date = next_at(schedule, base_ms)?;
    Some(format!("next at {} (epoch {})", format_utc(&date), epoch_seconds(&date)))
}

pub fn description_of(cron: &str) -> String {
    let pre = prenormalize(cron);
    let schedule = normalize(&pre);
    if !schedule.errors.is_empty() {
        return "\u{2014}".into();
    }
    let desc = describe(&pre);
    desc.special.unwrap_or(desc.full)
}

pub fn next_line_of(cron: &str) -> String {
    let pre = prenormalize(cron);
    let schedule = normalize(&pre);
    next_line(&schedule, js_sys::Date::now()).unwrap_or_else(|| "no next occurrence".into())
}

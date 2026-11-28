use serde_json::{json, Map, Value};

use super::agentfile::{validate_agentfile_doc, validate_sandbox_doc, Collector};
use super::yaml;

pub const JOB_NAME_MAX: usize = 99;
pub const DESCRIPTION_MAX: usize = 500;
pub const STAGE_NAME_MAX: usize = 200;
pub const STEP_TEXT_MAX: usize = 10_000;
pub const QUESTION_MAX: usize = 2000;
pub const PR_REPO_MAX: usize = 200;
pub const PR_BRANCH_MAX: usize = 200;
pub const PR_TITLE_MAX: usize = 200;
pub const PR_PATH_MAX: usize = 500;
pub const ENV_KEYS_MAX: usize = 128;
pub const ENV_VALUE_MAX: usize = 10_000;
pub const PARAM_VALUE_MAX: usize = 10_000;

pub fn step_kinds() -> &'static [&'static str] {
    &["agent", "shell", "input", "pr", "parallel"]
}

fn is_map(v: &Value) -> bool {
    v.is_object()
}

fn job_name_ok(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    name.chars().count() <= JOB_NAME_MAX
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
}

fn param_name_ok(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn cron_field_ok(raw: &str, min: i64, max: i64, names: &[(&str, i64)], field: &str) -> Result<(), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{field}: empty field"));
    }
    for token_raw in trimmed.split(',') {
        let token = token_raw.trim();
        if token.is_empty() {
            return Err(format!("{field}: empty token"));
        }
        let mut body = token;
        let mut step: i64 = 1;
        if token.contains('/') {
            let parts: Vec<&str> = token.split('/').collect();
            if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
                return Err(format!("{field}: invalid step token '{token}'"));
            }
            body = parts[0];
            step = parts[1]
                .parse()
                .map_err(|_| format!("{field}: invalid step '{token}'"))?;
            if step <= 0 {
                return Err(format!("{field}: step must be > 0"));
            }
        }
        if body == "*" {
            continue;
        }
        if body.contains('-') {
            let parts: Vec<&str> = body.split('-').collect();
            if parts.len() != 2 {
                return Err(format!("{field}: invalid range '{body}'"));
            }
            let start = cron_value(parts[0], names, min, max, field)?;
            let end = cron_value(parts[1], names, min, max, field)?;
            if start > end {
                return Err(format!("{field}: invalid range '{body}'"));
            }
            continue;
        }
        if step != 1 {
            return Err(format!("{field}: step requires '*' or a range"));
        }
        cron_value(body, names, min, max, field)?;
    }
    Ok(())
}

fn cron_value(raw: &str, names: &[(&str, i64)], min: i64, max: i64, field: &str) -> Result<i64, String> {
    let lower = raw.to_ascii_lowercase();
    if let Some((_, v)) = names.iter().find(|(n, _)| *n == lower) {
        return Ok(*v);
    }
    if !raw.chars().all(|c| c.is_ascii_digit()) || raw.is_empty() {
        return Err(format!("{field}: invalid value '{raw}'"));
    }
    let n: i64 = raw.parse().map_err(|_| format!("{field}: invalid value '{raw}'"))?;
    if n < min || n > max {
        return Err(format!(
            "{field}: value out of range '{raw}' (allowed {min}-{max})"
        ));
    }
    Ok(n)
}

pub fn validate_cron_shape(cron: &str) -> Result<(), String> {
    if cron.trim().is_empty() {
        return Err("cron must be a non-empty 5-field crontab string".to_string());
    }
    let parts: Vec<&str> = cron.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(format!(
            "cron expression must have exactly 5 fields (minute hour day-of-month month day-of-week); got {}",
            parts.len()
        ));
    }
    const MONTHS: &[(&str, i64)] = &[
        ("jan", 1), ("feb", 2), ("mar", 3), ("apr", 4), ("may", 5), ("jun", 6),
        ("jul", 7), ("aug", 8), ("sep", 9), ("oct", 10), ("nov", 11), ("dec", 12),
    ];
    const DOWS: &[(&str, i64)] = &[
        ("sun", 0), ("mon", 1), ("tue", 2), ("wed", 3), ("thu", 4), ("fri", 5), ("sat", 6),
    ];
    cron_field_ok(parts[0], 0, 59, &[], "minute")?;
    cron_field_ok(parts[1], 0, 23, &[], "hour")?;
    cron_field_ok(parts[2], 1, 31, &[], "day-of-month")?;
    cron_field_ok(parts[3], 1, 12, MONTHS, "month")?;
    cron_field_ok(parts[4], 0, 7, DOWS, "day-of-week")?;
    Ok(())
}

fn validate_env_map(
    collect: &mut Collector,
    env: &Value,
    base_path: &str,
) -> Map<String, Value> {
    let mut out = Map::new();
    if env.is_null() {
        return out;
    }
    if !is_map(env) {
        collect.add(base_path, "must be a map of string → string");
        return out;
    }
    let map = env.as_object().unwrap();
    if map.len() > ENV_KEYS_MAX {
        collect.add(base_path, format!("must have at most {ENV_KEYS_MAX} entries"));
    }
    for (key, value) in map {
        let key_path = format!("{base_path}.{key}");
        match value {
            Value::String(s) => {
                if s.chars().count() > ENV_VALUE_MAX {
                    collect.add(&key_path, format!("must be at most {ENV_VALUE_MAX} characters"));
                } else {
                    out.insert(key.clone(), Value::String(s.clone()));
                }
            }
            _ => collect.add(
                &key_path,
                "must be a string (quote YAML values, e.g. WINDOW_HOURS: \"6\")",
            ),
        }
    }
    out
}

fn validate_params_doc(collect: &mut Collector, params: &Value, base_path: &str) -> Value {
    let mut out = Map::new();
    if params.is_null() {
        return Value::Object(out);
    }
    if !is_map(params) {
        collect.add(base_path, "must be a map of param name → {default?: string}");
        return Value::Object(out);
    }
    let map = params.as_object().unwrap();
    if map.len() > ENV_KEYS_MAX {
        collect.add(base_path, format!("must have at most {ENV_KEYS_MAX} entries"));
    }
    for (key, value) in map {
        let key_path = format!("{base_path}.{key}");
        if !param_name_ok(key) {
            collect.add(&key_path, "must match ^[A-Za-z_][A-Za-z0-9_-]*$ — the param name IS the env var the value lands in");
            continue;
        }
        if !is_map(value) {
            collect.add(&key_path, "must be a map with at most one key: default");
            continue;
        }
        for field in value.as_object().unwrap().keys() {
            if field != "default" {
                collect.add(&format!("{key_path}.{field}"), "unknown field (allowed: default)");
            }
        }
        match value.get("default") {
            None => {
                out.insert(key.clone(), json!({}));
            }
            Some(Value::String(s)) => {
                if s.chars().count() > PARAM_VALUE_MAX {
                    collect.add(&format!("{key_path}.default"), format!("must be at most {PARAM_VALUE_MAX} characters"));
                } else {
                    out.insert(key.clone(), json!({ "default": s }));
                }
            }
            Some(_) => collect.add(&format!("{key_path}.default"), "must be a string (quote YAML values)"),
        }
    }
    Value::Object(out)
}

struct StageCtx {
    stage_names: std::collections::HashSet<String>,
    base_dir: String,
    inline: bool,
    depth: usize,
}

fn validate_steps(
    collect: &mut Collector,
    steps: &Value,
    base_path: &str,
    ctx: &mut StageCtx,
    allow_parallel: bool,
) -> Vec<Value> {
    let mut normalized = Vec::new();
    if steps.is_null() {
        return normalized;
    }
    if !steps.is_array() {
        collect.add(base_path, "must be a list of steps (each step is a single-key map)");
        return normalized;
    }
    for (index, step) in steps.as_array().unwrap().iter().enumerate() {
        let step_path = format!("{base_path}[{index}]");
        if !is_map(step) {
            collect.add(&step_path, "must be a single-key map naming one step kind (agent|shell|input|pr|parallel)");
            continue;
        }
        let map = step.as_object().unwrap();
        if map.len() != 1 {
            collect.add(
                &step_path,
                format!(
                    "must be a single-key map naming exactly one step kind (agent|shell|input|pr|parallel); got {} keys: {}",
                    map.len(),
                    map.keys().cloned().collect::<Vec<_>>().join(", ")
                ),
            );
            continue;
        }
        let (kind, value) = map.iter().next().unwrap();
        let value_path = format!("{step_path}.{kind}");
        if !step_kinds().contains(&kind.as_str()) {
            collect.add(&step_path, format!("unknown step kind '{kind}' (allowed: agent, shell, input, pr, parallel)"));
            continue;
        }
        match kind.as_str() {
            "agent" | "shell" => match value {
                Value::String(s) if !s.trim().is_empty() && s.chars().count() <= STEP_TEXT_MAX => {
                    normalized.push(json!({ kind: s }));
                }
                Value::String(s) if s.chars().count() > STEP_TEXT_MAX => {
                    collect.add(&value_path, format!("must be at most {STEP_TEXT_MAX} characters"));
                }
                _ => collect.add(
                    &value_path,
                    format!("must be a non-empty string (the {})", if kind == "agent" { "instruction" } else { "command" }),
                ),
            },
            "input" => {
                if !is_map(value) {
                    collect.add(&value_path, "must be a map: {question: \"…\"}");
                    continue;
                }
                for key in value.as_object().unwrap().keys() {
                    if key != "question" {
                        collect.add(&format!("{value_path}.{key}"), "unknown field (allowed: question)");
                    }
                }
                match value.get("question") {
                    Some(Value::String(q)) if !q.trim().is_empty() && q.chars().count() <= QUESTION_MAX => {
                        normalized.push(json!({ "input": { "question": q } }));
                    }
                    Some(Value::String(q)) if q.chars().count() > QUESTION_MAX => {
                        collect.add(&format!("{value_path}.question"), format!("must be at most {QUESTION_MAX} characters"));
                    }
                    _ => collect.add(
                        &format!("{value_path}.question"),
                        "is required and must be a non-empty string",
                    ),
                }
            }
            "pr" => {
                if !is_map(value) {
                    collect.add(&value_path, "must be a map: {repo, branch, title, path?}");
                    continue;
                }
                for key in value.as_object().unwrap().keys() {
                    if !matches!(key.as_str(), "repo" | "branch" | "title" | "path") {
                        collect.add(&format!("{value_path}.{key}"), "unknown field (allowed: repo, branch, title, path)");
                    }
                }
                let mut pr = Map::new();
                let mut ok = true;
                for (field, max) in [
                    ("repo", PR_REPO_MAX),
                    ("branch", PR_BRANCH_MAX),
                    ("title", PR_TITLE_MAX),
                ] {
                    let v = value.get(field);
                    match v {
                        Some(Value::String(s)) if !s.trim().is_empty() && s.chars().count() <= max => {
                            pr.insert(field.to_string(), Value::String(s.clone()));
                        }
                        Some(Value::String(s)) if s.chars().count() > max => {
                            collect.add(&format!("{value_path}.{field}"), format!("must be at most {max} characters"));
                            ok = false;
                        }
                        _ => {
                            collect.add(&format!("{value_path}.{field}"), "is required and must be a non-empty string");
                            ok = false;
                        }
                    }
                }
                if let Some(v) = value.get("path") {
                    match v {
                        Value::String(s) if !s.trim().is_empty() && s.chars().count() <= PR_PATH_MAX => {
                            pr.insert("path".into(), Value::String(s.clone()));
                        }
                        _ => {
                            collect.add(&format!("{value_path}.path"), format!("must be a non-empty string of at most {PR_PATH_MAX} characters"));
                            ok = false;
                        }
                    }
                }
                if ok {
                    normalized.push(json!({ "pr": pr }));
                }
            }
            "parallel" => {
                if !allow_parallel {
                    collect.add(&step_path, "post steps may not fork — 'parallel' is a pipeline concept (stages), not a post-condition");
                    continue;
                }
                if !value.is_array() {
                    collect.add(&value_path, "must be a list of stages (fork/join branches)");
                    continue;
                }
                if ctx.depth >= 1 {
                    collect.add(&step_path, "nested parallel stages are not supported — fork groups are one level deep");
                    continue;
                }
                let branches: Vec<Value> = value
                    .as_array()
                    .unwrap()
                    .iter()
                    .enumerate()
                    .map(|(bi, b)| validate_stage(collect, b, &format!("{value_path}[{bi}]"), ctx, 1))
                    .collect();
                normalized.push(json!({ "parallel": branches }));
            }
            _ => {}
        }
    }
    normalized
}

fn budget_value(collect: &mut Collector, budget: &Value, field: &str, path: &str) -> Option<i64> {
    match budget.get(field) {
        None | Some(Value::Null) => None,
        Some(Value::Number(n)) if n.as_i64().map(|v| v > 0).unwrap_or(false) => n.as_i64(),
        Some(other) => {
            collect.add(
                path,
                format!("must be a positive integer (got {other}) — zero, negative, and fractional values are refused; absent = unlimited"),
            );
            None
        }
    }
}

fn validate_budget(collect: &mut Collector, budget: &Value, base_path: &str) -> Value {
    let budget_path = format!("{base_path}.budget");
    if !is_map(budget) {
        collect.add(&budget_path, "must be a map: {max_turns?: <positive int>, wall_clock_ms?: <positive int>}");
        return Value::Null;
    }
    for key in budget.as_object().unwrap().keys() {
        if key != "max_turns" && key != "wall_clock_ms" {
            collect.add(&format!("{budget_path}.{key}"), "unknown budget field (allowed: max_turns, wall_clock_ms)");
        }
    }
    let mut out = Map::new();
    if let Some(v) = budget_value(collect, budget, "max_turns", &format!("{budget_path}.max_turns")) {
        out.insert("maxTurns".into(), json!(v));
    }
    if let Some(v) = budget_value(collect, budget, "wall_clock_ms", &format!("{budget_path}.wall_clock_ms")) {
        out.insert("wallClockMs".into(), json!(v));
    }
    Value::Object(out)
}

fn validate_stage(
    collect: &mut Collector,
    stage: &Value,
    base_path: &str,
    ctx: &mut StageCtx,
    depth: usize,
) -> Value {
    ctx.depth = depth;
    let empty = json!({
        "name": Value::Null, "agent": Value::Null, "agentfile": Value::Null,
        "env": {}, "sandbox": Value::Null, "steps": [], "parallel": Value::Null,
        "isolation": "sandbox", "budget": Value::Null,
    });
    if !is_map(stage) {
        collect.add(base_path, "must be a map (name, agent?, sandbox?, env?, steps?, parallel?, isolation?, budget?)");
        return empty;
    }
    let fork = stage.get("parallel").is_some() && !stage.get("parallel").unwrap().is_null();
    let allowed: &[&str] = if fork {
        &["name", "parallel"]
    } else {
        &["name", "agent", "sandbox", "env", "steps", "parallel", "isolation", "budget"]
    };
    for key in stage.as_object().unwrap().keys() {
        if !allowed.contains(&key.as_str()) {
            collect.add(
                &format!("{base_path}.{key}"),
                if fork {
                    "unknown field in a fork stage — fork stages are pure: {name, parallel: [stages]}"
                } else {
                    "unknown field (allowed: name, agent, sandbox, env, steps, parallel, isolation, budget)"
                },
            );
        }
    }

    let name = match stage.get("name") {
        Some(Value::String(s)) if !s.trim().is_empty() && s.chars().count() <= STAGE_NAME_MAX => {
            if ctx.stage_names.contains(s.as_str()) {
                collect.add(
                    &format!("{base_path}.name"),
                    format!("duplicate stage name '{s}' — stage names must be unique across the whole pipeline"),
                );
            } else {
                ctx.stage_names.insert(s.clone());
            }
            Some(s.clone())
        }
        Some(Value::String(s)) => {
            collect.add(&format!("{base_path}.name"), format!("must be at most {STAGE_NAME_MAX} characters"));
            Some(s.clone())
        }
        _ => {
            collect.add(&format!("{base_path}.name"), "is required and must be a non-empty string");
            None
        }
    };

    if fork {
        let branches = stage.get("parallel").cloned().unwrap_or(Value::Null);
        if !branches.is_array() {
            collect.add(&format!("{base_path}.parallel"), "must be a list of stages (fork/join branches)");
            return empty;
        }
        if depth >= 1 {
            collect.add(&format!("{base_path}.parallel"), "nested parallel stages are not supported — fork groups are one level deep");
            return empty;
        }
        let list: Vec<Value> = branches
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, b)| validate_stage(collect, b, &format!("{base_path}.parallel[{i}]"), ctx, depth + 1))
            .collect();
        let mut out = empty.clone();
        out["name"] = json!(name);
        out["parallel"] = json!(list);
        return out;
    }

    let mut agentfile = Value::Null;
    let agent_ref = match stage.get("agent") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if !s.trim().is_empty() => Some(s.clone()),
        Some(_) => {
            collect.add(
                &format!("{base_path}.agent"),
                "must be a path string to an Agentfile.yaml (e.g. ./agentfiles/collect.yaml)",
            );
            None
        }
    };
    if let Some(reference) = &agent_ref {
        let absolute = reference.starts_with('/');
        if ctx.inline && !absolute {
            collect.add(
                &format!("{base_path}.agent"),
                format!("inline Jobfiles must reference Agentfiles by ABSOLUTE path (got '{reference}')"),
            );
        } else {
            let path = if absolute {
                reference.clone()
            } else {
                format!("{}/{}", ctx.base_dir.trim_end_matches('/'), reference)
            };
            match std::fs::read_to_string(&path) {
                Ok(text) => match super::yaml::parse(&text) {
                    Ok(doc) => {
                        let render_ctx = super::agentfile::render_context(None);
                        let mut sub = Collector::new();
                        agentfile = validate_agentfile_doc(
                            &mut sub,
                            &doc,
                            &format!("{base_path}.agent"),
                            &render_ctx,
                        )
                        .unwrap_or(Value::Null);
                        for e in sub.errors {
                            collect.errors.push(e);
                        }
                    }
                    Err(e) => collect.add(
                        &format!("{base_path}.agent: $"),
                        format!("YAML parse error in '{reference}': {e}"),
                    ),
                },
                Err(e) => collect.add(
                    &format!("{base_path}.agent"),
                    format!("cannot read Agentfile '{reference}' (resolved '{path}'): {e}"),
                ),
            }
        }
    }

    let mut sandbox = Value::Null;
    if let Some(s) = stage.get("sandbox").filter(|v| !v.is_null()) {
        if agent_ref.is_some() {
            collect.add(
                &format!("{base_path}.sandbox"),
                "agentic stages take their sandbox from their Agentfile — inline stage.sandbox is for traditional stages only (remove one of the two)",
            );
        } else {
            sandbox = validate_sandbox_doc(collect, s, &format!("{base_path}.sandbox"))
                .unwrap_or(Value::Null);
        }
    }

    let env = validate_env_map(collect, stage.get("env").unwrap_or(&Value::Null), &format!("{base_path}.env"));
    let steps = validate_steps(
        collect,
        stage.get("steps").unwrap_or(&Value::Null),
        &format!("{base_path}.steps"),
        ctx,
        true,
    );

    let mut isolation = "sandbox".to_string();
    if let Some(v) = stage.get("isolation").filter(|v| !v.is_null()) {
        match v.as_str() {
            Some("sandbox") => {}
            Some("host") => {
                if stage.get("sandbox").filter(|v| !v.is_null()).is_some() {
                    collect.add(
                        &format!("{base_path}.isolation"),
                        "'host' contradicts stage.sandbox — a host stage never provisions a sandbox",
                    );
                } else {
                    isolation = "host".to_string();
                }
            }
            other => collect.add(
                &format!("{base_path}.isolation"),
                format!("must be 'host' or 'sandbox' (got {other:?})"),
            ),
        }
    }

    let budget = match stage.get("budget").filter(|v| !v.is_null()) {
        Some(b) => validate_budget(collect, b, base_path),
        None => Value::Null,
    };

    if steps.is_empty() && agent_ref.is_none() {
        collect.add(base_path, "must declare agent:, steps:, or parallel: — an empty stage is not runnable");
    }
    if agent_ref.is_none() {
        if let Some(list) = stage.get("steps").and_then(Value::as_array) {
            for (i, step) in list.iter().enumerate() {
                if is_map(step) && step.as_object().unwrap().len() == 1 && step.get("agent").is_some() {
                    collect.add(
                        &format!("{base_path}.steps[{i}].agent"),
                        "agent steps require the stage to bind an Agentfile (stage.agent:) — traditional stages execute shell/input/pr steps only",
                    );
                }
            }
        }
    }

    json!({
        "name": name,
        "agent": agent_ref,
        "agentfile": agentfile,
        "env": Value::Object(env),
        "sandbox": sandbox,
        "steps": steps,
        "parallel": Value::Null,
        "isolation": isolation,
        "budget": budget,
    })
}

fn validate_triggers(collect: &mut Collector, on: &Value, base_path: &str) -> Value {
    if !is_map(on) {
        collect.add(base_path, "must be a map: {pull_request: {repos?, branches?, actions?}}");
        return Value::Null;
    }
    for key in on.as_object().unwrap().keys() {
        if key != "pull_request" {
            collect.add(
                &format!("{base_path}.{key}"),
                "unknown trigger (allowed: pull_request)",
            );
        }
    }
    let mut out = Map::new();
    if let Some(pr) = on.get("pull_request") {
        if !is_map(pr) {
            collect.add(&format!("{base_path}.pull_request"), "must be a map: {repos?, branches?, actions?}");
            return Value::Null;
        }
        for key in pr.as_object().unwrap().keys() {
            if !matches!(key.as_str(), "repos" | "branches" | "actions") {
                collect.add(
                    &format!("{base_path}.pull_request.{key}"),
                    "unknown field (allowed: repos, branches, actions)",
                );
            }
        }
        let mut spec = Map::new();
        for (field, max) in [("repos", 64usize), ("branches", 64), ("actions", 8)] {
            match pr.get(field) {
                None | Some(Value::Null) => {}
                Some(Value::Array(items)) => {
                    let mut list = Vec::new();
                    for (i, item) in items.iter().enumerate() {
                        match item {
                            Value::String(s) if !s.trim().is_empty() && s.chars().count() <= 200 => {
                                list.push(Value::String(s.clone()));
                            }
                            _ => collect.add(
                                &format!("{base_path}.pull_request.{field}[{i}]"),
                                "must be a non-empty string",
                            ),
                        }
                    }
                    if list.len() > max {
                        collect.add(&format!("{base_path}.pull_request.{field}"), format!("must have at most {max} items"));
                    }
                    if field == "actions" {
                        for item in &list {
                            if item.as_str() != Some("opened") {
                                collect.add(
                                    &format!("{base_path}.pull_request.actions"),
                                    "only 'opened' is supported in this version",
                                );
                            }
                        }
                    }
                    spec.insert(field.into(), Value::Array(list));
                }
                Some(_) => collect.add(
                    &format!("{base_path}.pull_request.{field}"),
                    "must be a list of strings",
                ),
            }
        }
        out.insert("pullRequest".into(), Value::Object(spec));
    }
    Value::Object(out)
}

pub struct LoadedJobfile {
    pub definition: Value,
    pub raw: Value,
    pub source: Value,
}

pub fn load_jobfile_text(
    text: &str,
    base_dir: Option<&str>,
    origin_path: Option<&str>,
) -> Result<LoadedJobfile, crate::err::ApiError> {
    let mut collect = Collector::new();
    let doc = yaml::parse(text).map_err(|e| {
        crate::err::ApiError::validation(format!("invalid Jobfile: YAML parse error: {e}"))
            .detail(json!({ "errors": [{ "path": "$", "message": format!("YAML parse error: {e}") }] }))
    })?;
    if !is_map(&doc) {
        collect.add("$", "the Jobfile must be a YAML map (apiVersion/kind/job/env/params/stages/post)");
        collect.throw_if_any("Jobfile")?;
    }
    for key in doc.as_object().unwrap().keys() {
        if !matches!(key.as_str(), "apiVersion" | "kind" | "job" | "env" | "params" | "stages" | "post" | "on" | "triggers" | "true")
        {
            collect.add(
                &format!("$.{key}"),
                "unknown field (allowed: apiVersion, kind, job, env, params, stages, post, on)",
            );
        }
    }
    if doc.get("apiVersion").and_then(Value::as_str) != Some("psicom/v0") {
        collect.add(
            "apiVersion",
            format!("must be 'psicom/v0' (got {})", doc.get("apiVersion").cloned().unwrap_or(Value::Null)),
        );
    }
    if doc.get("kind").and_then(Value::as_str) != Some("Job") {
        collect.add(
            "kind",
            format!("must be 'Job' (got {})", doc.get("kind").cloned().unwrap_or(Value::Null)),
        );
    }

    let mut job = json!({ "name": Value::Null, "description": "", "schedule": Value::Null });
    match doc.get("job") {
        Some(Value::Object(map)) => {
            for key in map.keys() {
                if !matches!(key.as_str(), "name" | "description" | "schedule") {
                    collect.add(&format!("job.{key}"), "unknown field (allowed: name, description, schedule)");
                }
            }
            match map.get("name") {
                Some(Value::String(s)) if job_name_ok(s) => job["name"] = json!(s),
                Some(Value::String(s)) => collect.add(
                    "job.name",
                    format!("must match ^[a-z0-9][a-z0-9._-]{{0,98}}$ (got '{s}')"),
                ),
                _ => collect.add("job.name", "is required and must be a non-empty string"),
            }
            match map.get("description") {
                None | Some(Value::Null) => {}
                Some(Value::String(s)) if s.chars().count() <= DESCRIPTION_MAX => {
                    job["description"] = json!(s)
                }
                Some(_) => collect.add("job.description", format!("must be at most {DESCRIPTION_MAX} characters")),
            }
            if let Some(schedule) = map.get("schedule").filter(|v| !v.is_null()) {
                if !is_map(schedule) {
                    collect.add("job.schedule", "must be a map: {cron: \"<5-field crontab>\", params?: {string: string}}");
                } else {
                    for key in schedule.as_object().unwrap().keys() {
                        if key != "cron" && key != "params" {
                            collect.add(&format!("job.schedule.{key}"), "unknown field (allowed: cron, params)");
                        }
                    }
                    match schedule.get("cron") {
                        None => collect.add("job.schedule.cron", "is required when schedule: is present (omit schedule: entirely for manual/webhook-only jobs)"),
                        Some(Value::String(cron)) => match validate_cron_shape(cron) {
                            Ok(()) => {
                                let mut s = Map::new();
                                s.insert("cron".into(), json!(cron));
                                if let Some(params) = schedule.get("params").filter(|v| !v.is_null()) {
                                    let p = validate_env_map(&mut collect, params, "job.schedule.params");
                                    s.insert("params".into(), Value::Object(p));
                                }
                                job["schedule"] = Value::Object(s);
                            }
                            Err(e) => collect.add("job.schedule.cron", e),
                        },
                        Some(_) => collect.add("job.schedule.cron", "must be a string"),
                    }
                }
            }
        }
        _ => collect.add("job", "is required and must be a map (name, description, schedule)"),
    }

    let env = validate_env_map(&mut collect, doc.get("env").unwrap_or(&Value::Null), "env");
    let params = validate_params_doc(&mut collect, doc.get("params").unwrap_or(&Value::Null), "params");

    let mut ctx = StageCtx {
        stage_names: std::collections::HashSet::new(),
        base_dir: base_dir.map(str::to_string).unwrap_or_default(),
        inline: base_dir.is_none(),
        depth: 0,
    };
    let mut stages = Vec::new();
    match doc.get("stages") {
        None => collect.add("stages", "is required — the pipeline must declare at least one stage"),
        Some(Value::Array(list)) if list.is_empty() => {
            collect.add("stages", "must contain at least one stage")
        }
        Some(Value::Array(list)) => {
            for (i, stage) in list.iter().enumerate() {
                stages.push(validate_stage(&mut collect, stage, &format!("stages[{i}]"), &mut ctx, 0));
            }
        }
        Some(_) => collect.add("stages", "must be a list of stages"),
    }

    let mut post = json!({ "always": [], "success": [], "failure": [] });
    if let Some(p) = doc.get("post").filter(|v| !v.is_null()) {
        if !is_map(p) {
            collect.add("post", "must be a map with optional always/success/failure step lists");
        } else {
            for key in p.as_object().unwrap().keys() {
                if !matches!(key.as_str(), "always" | "success" | "failure") {
                    collect.add(&format!("post.{key}"), "unknown post-condition (allowed: always, success, failure)");
                }
            }
            for key in ["always", "success", "failure"] {
                if let Some(list) = p.get(key).filter(|v| !v.is_null()) {
                    post[key] = json!(validate_steps(&mut collect, list, &format!("post.{key}"), &mut ctx, false));
                }
            }
        }
    }

    let triggers = match doc.get("on").or_else(|| doc.get("triggers")).or_else(|| doc.get("true")) {
        Some(t) if !t.is_null() => validate_triggers(&mut collect, t, "on"),
        _ => Value::Null,
    };

    collect.throw_if_any("Jobfile")?;

    let mut definition = Map::new();
    definition.insert("apiVersion".into(), json!("psicom/v0"));
    definition.insert("kind".into(), json!("Job"));
    definition.insert("job".into(), job);
    definition.insert("env".into(), Value::Object(env));
    definition.insert("params".into(), params);
    definition.insert("on".into(), triggers);
    definition.insert("stages".into(), json!(stages));
    definition.insert("post".into(), post);

    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(text.as_bytes());
    let sha = format!("{:x}", hasher.finalize());

    Ok(LoadedJobfile {
        definition: Value::Object(definition),
        raw: json!({
            "jobfile": text,
            "agentfiles": [],
        }),
        source: json!({
            "kind": if base_dir.is_none() { "inline" } else { "path" },
            "jobfilePath": origin_path,
            "sha256": sha,
        }),
    })
}

pub fn load_jobfile_path(path: &str) -> Result<LoadedJobfile, crate::err::ApiError> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        crate::err::ApiError::validation(format!("cannot read Jobfile at '{path}': {e}")).detail(
            json!({ "errors": [{ "path": "$", "message": format!("cannot read '{path}': {e}") }] }),
        )
    })?;
    let dir = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string());
    load_jobfile_text(&text, dir.as_deref(), Some(path))
}

use chrono::{Datelike, Timelike};
use serde_json::{json, Value};

pub const NAME_MAX: usize = 200;
pub const DESCRIPTION_MAX: usize = 500;
pub const SYSTEM_PROMPT_MAX: usize = 50_000;
pub const BLACKBOARD_MAX: usize = 100_000;
pub const TOOL_ID_MAX: usize = 128;
pub const LABEL_MAX: usize = 100;
pub const LIST_ITEMS_MAX: usize = 64;
pub const URL_MAX: usize = 500;
pub const REF_MAX: usize = 200;
pub const VOLUME_NAME_MAX: usize = 200;
pub const MOUNT_MAX: usize = 200;
pub const BIND_PATH_MAX: usize = 500;
pub const PROMPT_TAGS_MAX: usize = 10_000;

pub struct Collector {
    pub errors: Vec<Value>,
}

impl Collector {
    pub fn new() -> Self {
        Collector { errors: Vec::new() }
    }

    pub fn add(&mut self, path: &str, message: impl Into<String>) {
        self.errors.push(json!({ "path": path, "message": message.into() }));
    }

    pub fn throw_if_any(&mut self, what: &str) -> Result<(), crate::err::ApiError> {
        if self.errors.is_empty() {
            return Ok(());
        }
        let n = self.errors.len();
        Err(crate::err::ApiError::validation(format!(
            "invalid {what}: {n} problem{} found",
            if n == 1 { "" } else { "s" }
        ))
        .detail(json!({ "errors": self.errors })))
    }
}

fn is_map(v: &Value) -> bool {
    v.is_object()
}

fn string_at(
    collect: &mut Collector,
    container: &Value,
    key: &str,
    path: &str,
    required: bool,
    max: usize,
) -> Option<String> {
    let value = container.get(key);
    match value {
        None | Some(Value::Null) => {
            if required {
                collect.add(path, "is required and must be a string");
            }
            None
        }
        Some(Value::String(s)) => {
            if required && s.trim().is_empty() {
                collect.add(path, "must not be empty");
                None
            } else if s.chars().count() > max {
                collect.add(path, format!("must be at most {max} characters"));
                None
            } else {
                Some(s.clone())
            }
        }
        Some(_) => {
            collect.add(path, "must be a string");
            None
        }
    }
}

pub fn render_context(sandbox: Option<&Value>) -> Value {
    let now = chrono::Utc::now();
    let mut ctx = json!({
        "process": {
            "platform": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "version": std::env::consts::OS,
        },
        "os": {
            "hostname": hostname(),
            "type": std::env::consts::OS,
            "platform": std::env::consts::OS,
            "release": "",
            "arch": std::env::consts::ARCH,
        },
        "Date": {
            "now": now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "year": format!("{:04}", now.year()),
            "month": format!("{:02}", now.month()),
            "day": format!("{:02}", now.day()),
            "hour": format!("{:02}", now.hour()),
            "minute": format!("{:02}", now.minute()),
        },
    });
    if let Some(s) = sandbox {
        if let Some(repos) = s.get("repos").and_then(Value::as_array).filter(|r| !r.is_empty()) {
            let mut used = std::collections::HashSet::new();
            let list: Vec<Value> = repos
                .iter()
                .map(|repo| {
                    let url = repo.get("url").and_then(Value::as_str).unwrap_or_default();
                    json!({
                        "url": url,
                        "ref": repo.get("ref").cloned().unwrap_or(Value::Null),
                        "path": repo_mount_path(url, &mut used),
                    })
                })
                .collect();
            ctx["sandbox"] = json!({
                "repos": list,
                "volumes": s.get("volumes").cloned().unwrap_or(json!([])),
            });
        }
    }
    ctx
}

fn hostname() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "localhost".to_string())
}

fn repo_mount_path(url: &str, used: &mut std::collections::HashSet<String>) -> String {
    let base = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("repo")
        .trim_end_matches(".git")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    let base: String = base.chars().take(64).collect();
    let base = if base.is_empty() { "repo".to_string() } else { base };
    let mut name = base.clone();
    let mut n = 2;
    while used.contains(&name) {
        name = format!("{base}-{n}");
        n += 1;
    }
    used.insert(name.clone());
    format!("/work/repos/{name}")
}

pub fn render_path_valid(path: &str) -> bool {
    let mut parts = path.split('.');
    let Some(first) = parts.next() else {
        return false;
    };
    if !identifier(first) {
        return false;
    }
    parts.all(|p| identifier(p) || (!p.is_empty() && p.chars().all(|c| c.is_ascii_digit())))
}

fn identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn resolve_path(ctx: &Value, path: &str) -> Option<Value> {
    let mut current = ctx;
    for seg in path.split('.') {
        match current {
            Value::Object(map) => current = map.get(seg)?,
            Value::Array(items) => {
                let idx: usize = seg.parse().ok()?;
                current = items.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(current.clone())
}

pub fn render_template(template: &str, ctx: &Value) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    let mut tag_count = 0usize;
    loop {
        let Some(open) = rest.find("<%") else {
            out.push_str(rest);
            return Ok(out);
        };
        out.push_str(&rest[..open]);
        rest = &rest[open..];
        if rest.starts_with("<%%") {
            out.push_str("<%");
            rest = &rest[3..];
            continue;
        }
        let Some(close) = rest.find("%>") else {
            return Err("unclosed EJS tag — expected '%>'".to_string());
        };
        let modifier = rest[2..3].chars().next();
        let body = rest[3..close].trim().to_string();
        tag_count += 1;
        if tag_count > PROMPT_TAGS_MAX {
            return Err("more than 10000 EJS tags — refusing to render".to_string());
        }
        match modifier {
            Some('#') => {}
            Some('=') | Some('-') => {
                if !render_path_valid(&body) {
                    return Err(format!(
                        "'{body}' is not a supported EJS expression — only dotted paths like <%= os.hostname %>"
                    ));
                }
                let value = resolve_path(ctx, &body).ok_or_else(|| {
                    format!("unknown render path '{body}' (exposed: process, os, Date, sandbox)")
                })?;
                match value {
                    Value::String(s) => out.push_str(&s),
                    Value::Null => out.push_str("null"),
                    Value::Bool(b) => out.push_str(if b { "true" } else { "false" }),
                    Value::Number(n) => out.push_str(&n.to_string()),
                    _ => {
                        return Err(format!(
                            "render path '{body}' resolves to an object; templates interpolate scalars only"
                        ))
                    }
                }
            }
            _ => {
                return Err(
                    "EJS scriptlets are not supported — system_prompt rendering is pure substitution"
                        .to_string(),
                )
            }
        }
        rest = &rest[close + 2..];
    }
}

pub fn volume_mode(collect: &mut Collector, volume: &Value, volume_path: &str) -> String {
    match volume.get("mode") {
        None | Some(Value::Null) => "rw".to_string(),
        Some(Value::String(m)) if m == "ro" || m == "rw" => m.clone(),
        Some(other) => {
            collect.add(
                &format!("{volume_path}.mode"),
                format!("must be 'ro' or 'rw' (got {other})"),
            );
            "rw".to_string()
        }
    }
}

pub fn validate_sandbox_doc(
    collect: &mut Collector,
    sandbox: &Value,
    base_path: &str,
) -> Option<Value> {
    if sandbox.is_null() {
        return None;
    }
    if !is_map(sandbox) {
        collect.add(base_path, "must be a map with optional repos: and volumes: lists");
        return None;
    }
    for key in sandbox.as_object().unwrap().keys() {
        if key != "repos" && key != "volumes" {
            collect.add(
                &format!("{base_path}.{key}"),
                "unknown field (allowed: repos, volumes)",
            );
        }
    }
    let mut repos: Vec<Value> = Vec::new();
    if let Some(list) = sandbox.get("repos") {
        if !list.is_array() {
            collect.add(&format!("{base_path}.repos"), "must be a list of {url, ref?} maps");
        } else {
            for (i, repo) in list.as_array().unwrap().iter().enumerate() {
                let repo_path = format!("{base_path}.repos[{i}]");
                if !is_map(repo) {
                    collect.add(&repo_path, "must be a map with a url (and optional ref)");
                    continue;
                }
                for key in repo.as_object().unwrap().keys() {
                    if key != "url" && key != "ref" {
                        collect.add(&format!("{repo_path}.{key}"), "unknown field (allowed: url, ref)");
                    }
                }
                let mut sub = Collector::new();
                let url = string_at(&mut sub, repo, "url", &format!("{repo_path}.url"), true, URL_MAX);
                let rref = string_at(&mut sub, repo, "ref", &format!("{repo_path}.ref"), false, REF_MAX);
                for e in sub.errors {
                    collect.errors.push(e);
                }
                if let Some(url) = url {
                    repos.push(json!({ "url": url, "ref": rref }));
                }
            }
        }
    }
    let mut volumes: Vec<Value> = Vec::new();
    if let Some(list) = sandbox.get("volumes") {
        if !list.is_array() {
            collect.add(
                &format!("{base_path}.volumes"),
                "must be a list of volumes — named {name, mount, mode?} or host bind {type: bind, path, mount, mode?}",
            );
        } else {
            for (i, volume) in list.as_array().unwrap().iter().enumerate() {
                let volume_path = format!("{base_path}.volumes[{i}]");
                if !is_map(volume) {
                    collect.add(
                        &volume_path,
                        "must be a map — named volume {name, mount, mode?} or host bind {type: bind, path, mount, mode?}",
                    );
                    continue;
                }
                let kind = volume
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("volume")
                    .to_string();
                if kind != "volume" && kind != "bind" {
                    collect.add(
                        &format!("{volume_path}.type"),
                        format!("must be 'volume' (named) or 'bind' (host-path mount) (got {kind})"),
                    );
                    continue;
                }
                if kind == "bind" {
                    for key in volume.as_object().unwrap().keys() {
                        if !matches!(key.as_str(), "type" | "path" | "mount" | "mode") {
                            collect.add(
                                &format!("{volume_path}.{key}"),
                                "unknown field in a bind volume (allowed: type, path, mount, mode)",
                            );
                        }
                    }
                    let mut sub = Collector::new();
                    let path = string_at(&mut sub, volume, "path", &format!("{volume_path}.path"), true, BIND_PATH_MAX);
                    let mount = string_at(&mut sub, volume, "mount", &format!("{volume_path}.mount"), true, MOUNT_MAX);
                    for e in sub.errors {
                        collect.errors.push(e);
                    }
                    if let Some(path) = &path {
                        let token_led = path.starts_with('<');
                        if path == "/"
                            || path.contains(' ')
                            || path.contains(':')
                            || path.contains(',')
                        {
                            collect.add(
                                &format!("{volume_path}.path"),
                                format!("must be a host directory path without spaces, ':' or ',' (and not '/' itself) (got '{path}')"),
                            );
                        } else if !path.starts_with('/') && !token_led {
                            collect.add(
                                &format!("{volume_path}.path"),
                                format!("must be an absolute HOST path starting with '/' — or a run-context token path like <evidence_dir> (got '{path}')"),
                            );
                        }
                    }
                    if let Some(mount) = &mount {
                        if !mount.starts_with('/') {
                            collect.add(
                                &format!("{volume_path}.mount"),
                                format!("must be an absolute guest path starting with '/' (got '{mount}')"),
                            );
                        }
                    }
                    let mode = volume_mode(collect, volume, &volume_path);
                    if let (Some(path), Some(mount)) = (path, mount) {
                        volumes.push(json!({ "type": "bind", "path": path, "mount": mount, "mode": mode }));
                    }
                    continue;
                }
                for key in volume.as_object().unwrap().keys() {
                    if !matches!(key.as_str(), "type" | "name" | "mount" | "mode") {
                        collect.add(
                            &format!("{volume_path}.{key}"),
                            "unknown field (allowed: type, name, mount, mode)",
                        );
                    }
                }
                let mut sub = Collector::new();
                let name = string_at(&mut sub, volume, "name", &format!("{volume_path}.name"), true, VOLUME_NAME_MAX);
                let mount = string_at(&mut sub, volume, "mount", &format!("{volume_path}.mount"), true, MOUNT_MAX);
                for e in sub.errors {
                    collect.errors.push(e);
                }
                if let Some(mount) = &mount {
                    if !mount.starts_with('/') {
                        collect.add(
                            &format!("{volume_path}.mount"),
                            format!("must be an absolute guest path starting with '/' (got '{mount}')"),
                        );
                    }
                }
                let mode = volume_mode(collect, volume, &volume_path);
                if let (Some(name), Some(mount)) = (name, mount) {
                    volumes.push(json!({ "type": "volume", "name": name, "mount": mount, "mode": mode }));
                }
            }
            let mut seen = std::collections::HashSet::new();
            for v in &volumes {
                let mount = v.get("mount").and_then(Value::as_str).unwrap_or_default();
                if !seen.insert(mount.to_string()) {
                    collect.add(
                        &format!("{base_path}.volumes"),
                        format!("duplicate mount '{mount}' — each guest mount path may be attached once"),
                    );
                }
            }
        }
    }
    if repos.is_empty() && volumes.is_empty() {
        return None;
    }
    Some(json!({ "repos": repos, "volumes": volumes }))
}

fn at(base_path: &str, inner: &str) -> String {
    if base_path == "$" {
        inner.to_string()
    } else if inner == "$" {
        base_path.to_string()
    } else {
        format!("{base_path}: {}", inner.trim_start_matches("$.").to_string())
    }
}

pub fn validate_agentfile_doc(
    collect: &mut Collector,
    doc: &Value,
    base_path: &str,
    ctx: &Value,
) -> Option<Value> {
    if !is_map(doc) {
        collect.add(&at(base_path, "$"), "must be a YAML map (apiVersion/kind/agent/sandbox)");
        return None;
    }
    for key in doc.as_object().unwrap().keys() {
        if !matches!(key.as_str(), "apiVersion" | "kind" | "agent" | "sandbox") {
            collect.add(
                &at(base_path, &format!("$.{key}")),
                "unknown field (allowed: apiVersion, kind, agent, sandbox)",
            );
        }
    }
    if doc.get("apiVersion").and_then(Value::as_str) != Some("psicom/v0") {
        collect.add(
            &at(base_path, "$.apiVersion"),
            format!("must be 'psicom/v0' (got {})", doc.get("apiVersion").cloned().unwrap_or(Value::Null)),
        );
    }
    if doc.get("kind").and_then(Value::as_str) != Some("Agent") {
        collect.add(
            &at(base_path, "$.kind"),
            format!("must be 'Agent' (got {})", doc.get("kind").cloned().unwrap_or(Value::Null)),
        );
    }
    let agent = doc.get("agent")?;
    if !is_map(agent) {
        collect.add(
            &at(base_path, "$.agent"),
            "is required and must be a map (name, description, model, system_prompt, tools, labels, blackboard)",
        );
        return None;
    }
    for key in agent.as_object().unwrap().keys() {
        if !matches!(
            key.as_str(),
            "name" | "description" | "model" | "system_prompt" | "tools" | "labels" | "blackboard"
        ) {
            collect.add(
                &at(base_path, &format!("$.agent.{key}")),
                "unknown field (allowed: name, description, model, system_prompt, tools, labels, blackboard)",
            );
        }
    }

    let name = string_at(collect, agent, "name", &at(base_path, "$.agent.name"), true, NAME_MAX);
    let description = string_at(collect, agent, "description", &at(base_path, "$.agent.description"), false, DESCRIPTION_MAX)
        .unwrap_or_default();
    let blackboard = string_at(collect, agent, "blackboard", &at(base_path, "$.agent.blackboard"), false, BLACKBOARD_MAX)
        .unwrap_or_default();

    let sandbox = validate_sandbox_doc(collect, doc.get("sandbox").unwrap_or(&Value::Null), &at(base_path, "$.sandbox"));

    let model = agent.get("model");
    let model_str = match model {
        Some(Value::String(s)) if !s.trim().is_empty() => Some(s.clone()),
        _ => {
            collect.add(
                &at(base_path, "$.agent.model"),
                "is required and must be a 'provider:model' string",
            );
            None
        }
    };
    if let Some(m) = &model_str {
        let ok = {
            let mut it = m.split(':');
            let p = it.next().unwrap_or("");
            let rest = it.next().unwrap_or("");
            it.next().is_none()
                && !p.is_empty()
                && !rest.is_empty()
                && p.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
                && p.len() <= 100
                && rest.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
                && rest.len() <= 200
        };
        if !ok {
            collect.add(
                &at(base_path, "$.agent.model"),
                format!("must be 'provider:model' — lowercase provider and model separated by exactly one ':' (got '{m}')"),
            );
        }
    }

    let system_prompt = string_at(
        collect,
        agent,
        "system_prompt",
        &at(base_path, "$.agent.system_prompt"),
        true,
        SYSTEM_PROMPT_MAX,
    );
    let mut rendered = Value::Null;
    if let Some(sp) = &system_prompt {
        if !sp.trim().is_empty() {
            let render_ctx = match &sandbox {
                Some(s) => render_context(Some(s)),
                None => ctx.clone(),
            };
            match render_template(sp, &render_ctx) {
                Ok(text) => rendered = Value::String(text),
                Err(e) => collect.add(&at(base_path, "$.agent.system_prompt"), e),
            }
        }
    }

    let mut labels: Vec<Value> = Vec::new();
    if let Some(list) = agent.get("labels") {
        if !list.is_array() {
            collect.add(&at(base_path, "$.agent.labels"), "must be a list of strings");
        } else {
            if list.as_array().unwrap().len() > LIST_ITEMS_MAX {
                collect.add(&at(base_path, "$.agent.labels"), "must have at most 64 items");
            }
            for (i, label) in list.as_array().unwrap().iter().enumerate() {
                let path = at(base_path, &format!("$.agent.labels[{i}]"));
                match label {
                    Value::String(s) if !s.trim().is_empty() && s.chars().count() <= LABEL_MAX => {
                        labels.push(Value::String(s.clone()));
                    }
                    Value::String(s) if s.chars().count() > LABEL_MAX => {
                        collect.add(&path, format!("must be at most {LABEL_MAX} characters"));
                    }
                    _ => collect.add(&path, "must be a non-empty string"),
                }
            }
        }
    }

    let mut tools: Vec<Value> = Vec::new();
    if let Some(list) = agent.get("tools") {
        if !list.is_array() {
            collect.add(&at(base_path, "$.agent.tools"), "must be a list of MCP server/tool ids");
        } else {
            if list.as_array().unwrap().len() > LIST_ITEMS_MAX {
                collect.add(&at(base_path, "$.agent.tools"), "must have at most 64 items");
            }
            for (i, tool) in list.as_array().unwrap().iter().enumerate() {
                let path = at(base_path, &format!("$.agent.tools[{i}]"));
                match tool {
                    Value::String(s) if !s.trim().is_empty() && s.chars().count() <= TOOL_ID_MAX => {
                        tools.push(Value::String(s.clone()));
                    }
                    Value::String(s) if s.chars().count() > TOOL_ID_MAX => {
                        collect.add(&path, format!("must be at most {TOOL_ID_MAX} characters"));
                    }
                    _ => collect.add(&path, "must be a non-empty string"),
                }
            }
        }
    }

    Some(json!({
        "name": name,
        "description": description,
        "model": model_str,
        "labels": labels,
        "tools": tools,
        "blackboard": blackboard,
        "systemPrompt": rendered,
        "sandbox": sandbox,
    }))
}

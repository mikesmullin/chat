use angela::http::{Request as ARequest, ResponseSink};
use angela::serve::Server as AngelaServer;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn registry_path(root: &Path) -> std::path::PathBuf {
    root.join(".angela").join("automations.json")
}

fn load(root: &Path) -> Value {
    let text = std::fs::read_to_string(registry_path(root)).unwrap_or_default();
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if v.is_object() {
        v
    } else {
        json!({ "automations": [] })
    }
}

fn save(root: &Path, v: &Value) -> Result<(), String> {
    let p = registry_path(root);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
}

fn rows_of(reg: &Value) -> Vec<Value> {
    reg.get("automations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

use std::str::FromStr;

fn parse(cron: &str) -> Result<cron::Schedule, String> {
    let normalized = if cron.split_whitespace().count() == 5 {
        format!("0 {cron}")
    } else {
        cron.to_string()
    };
    cron::Schedule::from_str(&normalized).map_err(|e| format!("not a cron schedule: {e}"))
}

pub fn validate_cron(cron: &str) -> Result<(), String> {
    parse(cron).map(|_| ())
}

fn next_after(cron: &str, from_ms: u64) -> Option<u64> {
    let schedule = parse(cron).ok()?;
    let from = chrono::DateTime::from_timestamp_millis(from_ms as i64)?;
    let from = from.with_timezone(&chrono::Local);
    schedule.after(&from).next().map(|t| t.timestamp_millis() as u64)
}

fn new_id() -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("a-{:x}-{:x}", now_ms(), n)
}

pub fn list(root: &Path) -> Value {
    let now = now_ms();
    let rows: Vec<Value> = rows_of(&load(root))
        .into_iter()
        .map(|mut a| {
            let next = if a.get("enabled").and_then(Value::as_bool).unwrap_or(false) {
                let cron = a.get("cron").and_then(Value::as_str).unwrap_or_default();
                let from = a
                    .get("nextRunAt")
                    .and_then(Value::as_u64)
                    .filter(|n| *n > now)
                    .unwrap_or(now);
                next_after(cron, from).unwrap_or(0)
            } else {
                0
            };
            if let Some(obj) = a.as_object_mut() {
                obj.insert("nextRun".into(), json!(next));
            }
            a
        })
        .collect();
    json!({ "automations": rows })
}

pub fn upsert(root: &Path, body: &Value) -> Result<String, String> {
    let name = body.get("name").and_then(Value::as_str).unwrap_or_default().trim().to_string();
    if name.is_empty() {
        return Err("an automation needs a name".into());
    }
    let cron = body.get("cron").and_then(Value::as_str).unwrap_or_default().trim().to_string();
    validate_cron(&cron)?;
    let prompt = body.get("prompt").and_then(Value::as_str).unwrap_or_default().trim().to_string();
    if prompt.is_empty() {
        return Err("an automation needs a prompt".into());
    }
    let timeout = body.get("timeoutMin").and_then(Value::as_f64).filter(|t| *t > 0.0).unwrap_or(60.0);
    let id = body.get("id").and_then(Value::as_str).map(str::to_string).filter(|s| !s.trim().is_empty());
    let now = now_ms();
    let mut reg = load(root);
    let mut rows = rows_of(&reg);
    let id = if let Some(id) = id {
        let mut found = false;
        for a in rows.iter_mut() {
            if a.get("id").and_then(Value::as_str) == Some(&id) {
                found = true;
                if let Some(obj) = a.as_object_mut() {
                    obj.insert("name".into(), json!(name));
                    obj.insert("enabled".into(), json!(body.get("enabled").and_then(Value::as_bool).unwrap_or(true)));
                    obj.insert("cron".into(), json!(cron));
                    obj.insert("prompt".into(), json!(prompt));
                    obj.insert("agent".into(), json!(body.get("agent").and_then(Value::as_str).unwrap_or_default()));
                    obj.insert("model".into(), json!(body.get("model").and_then(Value::as_str).unwrap_or_default()));
                    obj.insert("timeoutMin".into(), json!(timeout));
                    obj.insert("catchUp".into(), json!(body.get("catchUp").and_then(Value::as_bool).unwrap_or(false)));
                    let next = next_after(&cron, now).unwrap_or(0);
                    obj.insert("nextRunAt".into(), json!(next));
                }
                break;
            }
        }
        if !found {
            return Err("no such automation".into());
        }
        id
    } else {
        let id = new_id();
        let mut m = Map::new();
        m.insert("id".into(), json!(id));
        m.insert("name".into(), json!(name));
        m.insert("enabled".into(), json!(body.get("enabled").and_then(Value::as_bool).unwrap_or(true)));
        m.insert("cron".into(), json!(cron.clone()));
        m.insert("prompt".into(), json!(prompt));
        m.insert("agent".into(), json!(body.get("agent").and_then(Value::as_str).unwrap_or_default()));
        m.insert("model".into(), json!(body.get("model").and_then(Value::as_str).unwrap_or_default()));
        m.insert("timeoutMin".into(), json!(timeout));
        m.insert("catchUp".into(), json!(body.get("catchUp").and_then(Value::as_bool).unwrap_or(false)));
        m.insert("createdAt".into(), json!(now));
        m.insert("lastRunAt".into(), json!(0));
        m.insert("nextRunAt".into(), json!(next_after(&cron, now).unwrap_or(0)));
        rows.push(Value::Object(m));
        id
    };
    reg["automations"] = Value::Array(rows);
    save(root, &reg)?;
    Ok(id)
}

pub fn remove(root: &Path, id: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err("which automation?".into());
    }
    let mut reg = load(root);
    let before = rows_of(&reg).len();
    let rows: Vec<Value> = rows_of(&reg)
        .into_iter()
        .filter(|a| a.get("id").and_then(Value::as_str) != Some(id))
        .collect();
    if rows.len() == before {
        return Err("no such automation".into());
    }
    reg["automations"] = Value::Array(rows);
    save(root, &reg)
}

pub fn set_enabled(root: &Path, id: &str, enabled: bool) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err("which automation?".into());
    }
    let mut reg = load(root);
    let mut rows = rows_of(&reg);
    let mut found = false;
    for a in rows.iter_mut() {
        if a.get("id").and_then(Value::as_str) == Some(id) {
            found = true;
            if let Some(obj) = a.as_object_mut() {
                obj.insert("enabled".into(), json!(enabled));
            }
            break;
        }
    }
    if !found {
        return Err("no such automation".into());
    }
    reg["automations"] = Value::Array(rows);
    save(root, &reg)
}

struct RecordingSink {
    assistant_filter: crate::assistant_text::StreamFilter,
    session_id: String,
    prompt: String,
    turn_id: Option<u64>,
}

impl ResponseSink for RecordingSink {
    fn json(&mut self, _status: u16, body: &Value) {
        eprintln!("automation turn for {} answered json: {}", self.session_id, body);
    }
    fn begin_ndjson(&mut self) {
        self.turn_id = Some(crate::events::begin_turn(&self.session_id, Vec::new(), self.prompt.clone()));
    }
    fn row(&mut self, value: &Value) -> bool {
        if let Some(id) = self.turn_id {
            for row in self.assistant_filter.process(value) { crate::events::turn_row(id, &row); }
        }
        true
    }
    fn end(&mut self) {
        if let Some(id) = self.turn_id.take() {
            if let Some(row) = self.assistant_filter.finish() { crate::events::turn_row(id, &row); }
            crate::events::end_turn(id);
        }
    }
}

pub fn submit_prompt(session_id: &str, content: String, agent: String, model: String) {
    spawn_turn(session_id, content, agent, model, "coordinator", "");
}

fn spawn_turn(
    session_id: &str,
    content: String,
    agent: String,
    model: String,
    tab_id: &str,
    from: &str,
) {
    let server = crate::api::angela(0);
    let session_id = session_id.to_string();
    let tab_id = tab_id.to_string();
    let from = from.to_string();
    std::thread::spawn(move || {
        let mut sink = RecordingSink {
            session_id: session_id.clone(),
            prompt: content.clone(),
            turn_id: None,
            assistant_filter: crate::assistant_text::StreamFilter::default(),
        };
        let req = ARequest {
            method: "POST".to_string(),
            path: "/api/chat".to_string(),
            query: String::new(),
            headers: BTreeMap::new(),
            body: serde_json::to_vec(&json!({
                "content": content,
                "sessionId": session_id,
                "tabId": tab_id,
                "agent": if agent.is_empty() { Value::Null } else { json!(agent) },
                "model": if model.is_empty() { Value::Null } else { json!(model) },
                "from": if from.is_empty() { Value::Null } else { json!(from) },
            }))
            .unwrap_or_default(),
        };
        server.dispatch(&req, &mut sink);
    });
}

pub fn recover_queue() {
    let server = crate::api::angela(0);
    let rows = server.state.store.queue_all();
    if rows.is_empty() {
        return;
    }
    eprintln!("queue: re-submitting {} message(s) left waiting", rows.len());
    for row in rows {
        let Some(session) = row.get("sessionId").and_then(Value::as_str).map(str::to_string) else {
            continue;
        };
        let Some(id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        server.state.store.queue_remove(&session, id);
        spawn_turn(
            &session,
            row.get("text").and_then(Value::as_str).unwrap_or_default().to_string(),
            row.get("agent").and_then(Value::as_str).unwrap_or_default().to_string(),
            row.get("model").and_then(Value::as_str).unwrap_or_default().to_string(),
            row.get("tabId").and_then(Value::as_str).unwrap_or_default(),
            row.get("from").and_then(Value::as_str).unwrap_or_default(),
        );
    }
    crate::events::bump_queues();
}

struct Capture(Option<Value>);
impl ResponseSink for Capture {
    fn json(&mut self, _status: u16, body: &Value) {
        self.0 = Some(body.clone());
    }
    fn begin_ndjson(&mut self) {}
    fn row(&mut self, _value: &Value) -> bool {
        true
    }
    fn end(&mut self) {}
}

fn dispatch(server: &AngelaServer, path: &str, body: Value) -> Value {
    let req = ARequest {
        method: "POST".to_string(),
        path: path.to_string(),
        query: String::new(),
        headers: BTreeMap::new(),
        body: serde_json::to_vec(&body).unwrap_or_default(),
    };
    let mut sink = Capture(None);
    server.dispatch(&req, &mut sink);
    sink.0.unwrap_or(Value::Null)
}

fn fire(server: &'static AngelaServer, automation: Value) {
    let name = automation.get("name").and_then(Value::as_str).unwrap_or("automation").to_string();
    let prompt = automation.get("prompt").and_then(Value::as_str).unwrap_or_default().to_string();
    if prompt.trim().is_empty() {
        return;
    }
    let want_agent = automation.get("agent").and_then(Value::as_str).unwrap_or_default().trim().to_string();
    let model = automation.get("model").and_then(Value::as_str).unwrap_or_default().trim().to_string();
    let timeout_min = automation.get("timeoutMin").and_then(Value::as_f64).filter(|t| *t > 0.0).unwrap_or(60.0);

    let defined: Vec<String> = server
        .state
        .store
        .agents()
        .list()
        .into_iter()
        .map(|a| a.name)
        .collect();
    let agent = if !want_agent.is_empty() && defined.iter().any(|n| n == &want_agent) {
        Some(want_agent)
    } else {
        defined.into_iter().next()
    };

    let title = format!("{} · {}", name, chrono::Local::now().format("%H:%M"));
    let created = match server.state.store.create(angela::session_store::NewSession {
        agent: agent.clone(),
        model: if model.is_empty() { None } else { Some(model.clone()) },
        title: Some(title),
        ..Default::default()
    }) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("automation {name}: could not create session: {e}");
            return;
        }
    };
    let session_id = created.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
    if session_id.is_empty() {
        return;
    }

    if let Some(agent_name) = agent.clone() {
        let seated = dispatch(
            server,
            &format!("/api/sessions/{session_id}/participants"),
            json!({ "kind": "agent", "agentName": agent_name, "handle": agent_name }),
        );
        if seated.get("ok").and_then(Value::as_bool) != Some(true) {
            eprintln!("automation {name}: could not seat @{agent_name}: {seated}");
        }
    }

    let cancels = &server.state.cancels;
    let watchdog_id = session_id.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs_f64(timeout_min * 60.0));
        cancels.stop(&watchdog_id);
    });

    let mut sink = RecordingSink { session_id: session_id.clone(), prompt, turn_id: None, assistant_filter: crate::assistant_text::StreamFilter::default() };
    let req = ARequest {
        method: "POST".to_string(),
        path: "/api/chat".to_string(),
        query: String::new(),
        headers: BTreeMap::new(),
        body: serde_json::to_vec(&json!({
            "content": automation.get("prompt"),
            "sessionId": session_id,
            "tabId": "automation",
            "agent": agent,
            "model": if model.is_empty() { Value::Null } else { json!(model) },
        }))
        .unwrap_or_default(),
    };
    server.dispatch(&req, &mut sink);
}

fn tick(server: &'static AngelaServer) {
    let root = crate::api::project_root();
    let now = now_ms();
    let due: Vec<Value> = rows_of(&load(&root))
        .into_iter()
        .filter(|a| {
            a.get("enabled").and_then(Value::as_bool).unwrap_or(false)
                && a.get("nextRunAt").and_then(Value::as_u64).is_some_and(|n| n <= now)
        })
        .collect();
    for automation in due {
        let id = automation.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
        let cron = automation.get("cron").and_then(Value::as_str).unwrap_or_default().to_string();
        let mut reg = load(&root);
        let mut rows = rows_of(&reg);
        for a in rows.iter_mut() {
            if a.get("id").and_then(Value::as_str) == Some(id.as_str()) {
                if let Some(obj) = a.as_object_mut() {
                    obj.insert("lastRunAt".into(), json!(now));
                    obj.insert("nextRunAt".into(), json!(next_after(&cron, now).unwrap_or(0)));
                }
                break;
            }
        }
        reg["automations"] = Value::Array(rows);
        if save(&root, &reg).is_err() {
            continue;
        }
        let mut claimed = automation.clone();
        if let Some(obj) = claimed.as_object_mut() {
            obj.insert("lastRunAt".into(), json!(now));
        }
        std::thread::spawn(move || fire(server, claimed));
    }
}

fn missed_since(cron: &str, baseline: u64, now: u64) -> bool {
    next_after(cron, baseline).is_some_and(|first| first <= now)
}

fn catch_up(server: &'static AngelaServer) {
    let root = crate::api::project_root();
    let now = now_ms();
    let mut reg = load(&root);
    let mut fired: Vec<Value> = Vec::new();
    let mut changed = false;
    if let Some(Value::Array(rows)) = reg.get_mut("automations") {
        for a in rows.iter_mut() {
            let Some(obj) = a.as_object_mut() else { continue };
            if !obj.get("enabled").and_then(Value::as_bool).unwrap_or(false) {
                continue;
            }
            let cron = obj
                .get("cron")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let baseline = obj
                .get("lastRunAt")
                .and_then(Value::as_u64)
                .filter(|t| *t > 0)
                .or_else(|| obj.get("createdAt").and_then(Value::as_u64))
                .unwrap_or(0);
            if !missed_since(&cron, baseline, now) {
                continue;
            }
            obj.insert("nextRunAt".into(), json!(next_after(&cron, now).unwrap_or(0)));
            changed = true;
            if obj.get("catchUp").and_then(Value::as_bool).unwrap_or(false) {
                obj.insert("lastRunAt".into(), json!(now));
                fired.push(Value::Object(obj.clone()));
            }
        }
    }
    if changed {
        let _ = save(&root, &reg);
    }
    for automation in fired {
        std::thread::spawn(move || fire(server, automation));
    }
}

pub fn start() {
    std::thread::spawn(|| {
        catch_up(crate::api::angela(0));
        loop {
            let server = crate::api::angela(0);
            tick(server);
            std::thread::sleep(Duration::from_secs(30));
        }
    });
}

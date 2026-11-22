//! Scheduled automations: named prompts that fire on a cron schedule.
//!
//! Each run creates a fresh session and submits the preconfigured prompt
//! through the same angela dispatch the browser uses — the turn is a real
//! turn (feed rows, notifications, transcript), not a side channel. Firing
//! happens on a poll thread; each due automation gets its own thread so a
//! long turn never blocks the next schedule check.
//!
//! Storage is a JSON registry beside the host registry (`hostgit` owns the
//! pattern: read whole, write-then-rename). One automation looks like:
//!
//! ```json
//! { "id": "a-...", "name": "issue-triage", "enabled": true,
//!   "cron": "0 9 * * 1-5", "prompt": "Triage ...",
//!   "agent": "scout", "model": "", "timeoutMin": 60,
//!   "createdAt": 0, "lastRunAt": 0, "nextRunAt": 0 }
//! ```
//!
//! Empty `agent`/`model` mean "the default", exactly like a blank composer
//! row. `nextRunAt` is recomputed on every read for enabled automations,
//! so a stale file can never schedule the past.

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

/// Read the registry, or an empty one. A corrupt file reads as empty rather
/// than fatal, for the same reason as the host registry: it is a cache of
/// things the user can re-create, and refusing to start over it would be
/// the worse failure.
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

/// Parse one schedule. Five fields are the cron everyone writes (the form
/// shows them); the crate speaks six, seconds first — so a five-field
/// expression fires at second zero rather than erroring. Six- and
/// seven-field expressions and `@shorthand`s pass through untouched.
fn parse(cron: &str) -> Result<cron::Schedule, String> {
    let normalized = if cron.split_whitespace().count() == 5 {
        format!("0 {cron}")
    } else {
        cron.to_string()
    };
    cron::Schedule::from_str(&normalized).map_err(|e| format!("not a cron schedule: {e}"))
}

/// Is this cron expression parseable? The client pre-checks the shape; the
/// server decides, so a hand-edited registry cannot wedge the scheduler.
pub fn validate_cron(cron: &str) -> Result<(), String> {
    parse(cron).map(|_| ())
}

/// The next fire strictly after `from_ms`, in epoch millis. `None` when the
/// expression never fires again (or does not parse — the tick skips those
/// rather than dying on them).
fn next_after(cron: &str, from_ms: u64) -> Option<u64> {
    let schedule = parse(cron).ok()?;
    let from = chrono::DateTime::from_timestamp_millis(from_ms as i64)?;
    let from = from.with_timezone(&chrono::Local);
    schedule.after(&from).next().map(|t| t.timestamp_millis() as u64)
}

/// A fresh id. Time-ordered like turn ids, unique like session ids.
fn new_id() -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("a-{:x}-{:x}", now_ms(), n)
}

/// The list the picker and the inspector read: each automation with its
/// display `nextRun` freshly computed, so the rail never shows a stale time.
pub fn list(root: &Path) -> Value {
    let now = now_ms();
    let rows: Vec<Value> = rows_of(&load(root))
        .into_iter()
        .map(|mut a| {
            let next = if a.get("enabled").and_then(Value::as_bool).unwrap_or(false) {
                let cron = a.get("cron").and_then(Value::as_str).unwrap_or_default();
                // Schedule from the later of now and the stored next run,
                // so a just-fired automation does not re-list its past slot.
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

/// Create or replace. Returns the id. A fresh automation fires next on its
/// first upcoming slot — never a backfill for time that passed while it did
/// not exist.
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

/// The sink half of a headless turn: rows go to the event feed, exactly as
/// if a browser were watching, so notifications, the tab strip and the
/// transcript all learn about the run. There is no client to disconnect,
/// so `row` always answers true — answering false would abort a turn
/// nobody asked to stop.
struct RecordingSink {
    assistant_filter: crate::assistant_text::StreamFilter,
    session_id: String,
    prompt: String,
    turn_id: Option<u64>,
}

impl ResponseSink for RecordingSink {
    fn json(&mut self, _status: u16, body: &Value) {
        // Only errors arrive here (chat answers NDJSON). With no client to
        // show them to, the server log is the honest destination.
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

/// A prompt submitted from *inside* a turn — the `chat__sessions_prompt`
/// tool's enqueue path (a coordinator agent driving a session other than
/// its own). Fire-and-forget: the turn runs on its own thread and waits for
/// a free model slot like any other turn, because the caller is a tool
/// handler inside another turn that is holding its own slot — blocking here
/// would deadlock a single-slot local model. Its rows reach the same event
/// feed a typed prompt would (the `RecordingSink` bridge).
pub fn submit_prompt(session_id: &str, content: String, agent: String, model: String) {
    let server = crate::api::angela(0);
    let session_id = session_id.to_string();
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
                "tabId": "coordinator",
                "agent": if agent.is_empty() { Value::Null } else { json!(agent) },
                "model": if model.is_empty() { Value::Null } else { json!(model) },
            }))
            .unwrap_or_default(),
        };
        server.dispatch(&req, &mut sink);
    });
}

/// One request through the same router the browser uses, answered
/// synchronously. Only JSON answers are captured — NDJSON streams go to a
/// recording sink instead (see `RecordingSink`).
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

/// Fire one automation: a fresh session, the agent seated, the prompt
/// submitted, a watchdog for the timeout. Runs on its own thread — a turn
/// can run for minutes, and the scheduler must keep checking the others.
fn fire(server: &'static AngelaServer, automation: Value) {
    let name = automation.get("name").and_then(Value::as_str).unwrap_or("automation").to_string();
    let prompt = automation.get("prompt").and_then(Value::as_str).unwrap_or_default().to_string();
    if prompt.trim().is_empty() {
        return;
    }
    let want_agent = automation.get("agent").and_then(Value::as_str).unwrap_or_default().trim().to_string();
    let model = automation.get("model").and_then(Value::as_str).unwrap_or_default().trim().to_string();
    let timeout_min = automation.get("timeoutMin").and_then(Value::as_f64).filter(|t| *t > 0.0).unwrap_or(60.0);

    // Which agent runs: the automation's pick when it names a defined one,
    // else the project's first. None defined and none named means the run
    // still happens — `run_round` stores the prompt and says plainly that
    // nobody is seated, which the owner can fix and re-send from.
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

    // Seat the agent first: an empty roster answers nothing (see
    // `run_round`'s no-agents path), and the request's agent field only
    // picks the definition, never the seat.
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

    // The timeout cannot pre-empt the create above, only the turn: arm the
    // watchdog now, and it stops a turn that is still talking when the
    // minutes run out. `stop` on a finished session is a harmless no-op
    // (and arms nothing that outlives its 5s window). The `&'static`
    // borrow rides into the thread because the server outlives every run.
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

/// One schedule check: claim every due automation (persist the claim first,
/// so a slow fire cannot double-run), then fire each on its own thread.
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
        // Claim the slot before spawning: the next tick must see a future
        // nextRun, not this one, or a long turn fires twice.
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

/// Did ≥1 cron occurrence pass unclaimed in `(baseline, now]`? This is the
/// "the machine was off for this" test behind the catch-up checkbox (B3).
fn missed_since(cron: &str, baseline: u64, now: u64) -> bool {
    next_after(cron, baseline).is_some_and(|first| first <= now)
}

/// Startup catch-up (B3): a slot that came due while the process was down
/// runs **once** at startup — only when the automation's `catchUp` flag is
/// set, and exactly once however many slots were missed. Without the flag
/// the stale slot is skipped forward, never backfilled.
///
/// Every enabled automation with a missed slot is claimed here (its
/// `nextRunAt` moves to the next future occurrence either way), so the
/// regular `tick` can never see the stale slot and the two paths cannot
/// both fire it. `lastRunAt` moves only on an actual run.
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
            // Claim the slot either way — fired or skipped — so tick cannot.
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

/// The scheduler. Thirty seconds beats minute-resolution cron with room to
/// spare, and a scan is one small JSON read — cheaper than the sandbox poll
/// beside it.
pub fn start() {
    std::thread::spawn(|| {
        // Before the first tick: the catch-up pass owns every stale slot
        // exactly once (B3), so tick only ever sees future schedules.
        catch_up(crate::api::angela(0));
        loop {
            // The server half is process-wide; the first tick happens after boot
            // has initialised it (see `main`), so this never builds a second one.
            let server = crate::api::angela(0);
            tick(server);
            std::thread::sleep(Duration::from_secs(30));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cron_shapes_parse() {
        for good in ["0 * * * *", "0 9 * * 1-5", "*/15 8-18 * * 1,3,5", "30 17 * * 5"] {
            assert!(validate_cron(good).is_ok(), "{good}");
        }
        for bad in ["", "nope", "0 9 * *", "61 * * * *", "0 24 * * *", "* * L * *"] {
            assert!(validate_cron(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn next_run_is_the_upcoming_slot() {
        // Monday 2026-11-23 08:00 local is before the 09:00 weekday slot.
        let from = chrono::Local::now();
        let _ = from;
        let base = chrono::NaiveDate::from_ymd_opt(2026, 11, 23)
            .unwrap()
            .and_hms_opt(8, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap()
            .timestamp_millis() as u64;
        let next = next_after("0 9 * * 1-5", base).expect("fires");
        let dt = chrono::DateTime::from_timestamp_millis(next as i64).unwrap().with_timezone(&chrono::Local);
        assert_eq!(dt.format("%H:%M").to_string(), "09:00");
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2026-11-23");
    }

    #[test]
    fn registry_roundtrip() {
        let root = std::env::temp_dir().join(format!("chat-automation-test-{}", now_ms()));
        let _ = std::fs::create_dir_all(root.join(".angela"));
        let id = upsert(
            &root,
            &json!({ "name": "triage", "cron": "0 9 * * 1-5", "prompt": "hi",
                         "agent": "scout", "timeoutMin": 30 }),
        )
        .expect("upsert");
        let listed = list(&root);
        let rows = listed.get("automations").and_then(Value::as_array).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("nextRun").and_then(Value::as_u64).unwrap_or(0) > 0, true);
        set_enabled(&root, &id, false).expect("toggle");
        let listed = list(&root);
        let rows = listed.get("automations").and_then(Value::as_array).unwrap();
        assert_eq!(rows[0].get("enabled"), Some(&json!(false)));
        assert_eq!(rows[0].get("nextRun"), Some(&json!(0)));
        remove(&root, &id).expect("remove");
        let listed = list(&root);
        assert_eq!(listed.get("automations").and_then(Value::as_array).unwrap().len(), 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> u64 {
        chrono::NaiveDate::from_ymd_opt(y, mo, d)
            .unwrap()
            .and_hms_opt(h, mi, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap()
            .timestamp_millis() as u64
    }

    #[test]
    fn catch_up_fires_only_when_a_slot_went_unclaimed() {
        // Daily at 09:00. The machine was off from Monday 08:00 to
        // Wednesday 10:00: Monday's and Tuesday's slots both passed.
        let cron = "0 9 * * *";
        assert!(missed_since(cron, at(2026, 11, 23, 8, 0), at(2026, 11, 25, 10, 0)));
        // The run happened (lastRunAt) after the slot: nothing was missed.
        assert!(!missed_since(cron, at(2026, 11, 25, 9, 0), at(2026, 11, 25, 10, 0)));
        // Between two slots, still warm: nothing to catch up.
        assert!(!missed_since(cron, at(2026, 11, 25, 9, 0), at(2026, 11, 25, 9, 30)));
        // An unparseable cron never fires — never missed.
        assert!(!missed_since("nope", at(2026, 11, 23, 8, 0), at(2026, 11, 25, 10, 0)));
    }

    #[test]
    fn catch_up_flag_roundtrips_through_the_registry() {
        let root = std::env::temp_dir().join(format!("chat-automation-test-catchup-{}", now_ms()));
        let _ = std::fs::create_dir_all(root.join(".angela"));
        let id = upsert(
            &root,
            &json!({ "name": "triage", "cron": "0 9 * * *", "prompt": "hi",
                     "catchUp": true }),
        )
        .expect("upsert");
        let rows = list(&root);
        let rows = rows.get("automations").and_then(Value::as_array).unwrap();
        assert_eq!(rows[0].get("catchUp"), Some(&json!(true)));
        // A save without the flag clears it — the checkbox is the truth.
        upsert(
            &root,
            &json!({ "id": id, "name": "triage", "cron": "0 9 * * *", "prompt": "hi" }),
        )
        .expect("re-save");
        let rows = list(&root);
        let rows = rows.get("automations").and_then(Value::as_array).unwrap();
        assert_eq!(rows[0].get("catchUp"), Some(&json!(false)));
        let _ = std::fs::remove_dir_all(&root);
    }
}

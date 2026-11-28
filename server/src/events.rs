use rustex1_devserver::{State, Stream};
use serde_json::{json, Value};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Default)]
struct Snapshot {
    values: [Option<Value>; 7],
    versions: [u64; 7],
    turns: HashMap<u64, Turn>,
    turn_version: u64,
}
struct Turn {
    session_id: String,
    baseline: Vec<Value>,
    prompt: String,
    rows: Vec<Value>,
    completed: Option<Instant>,
    completed_at_ms: Option<u64>,
}

struct Feed {
    snapshot: Mutex<Snapshot>,
    changed: Condvar,
}
fn feed() -> &'static Feed {
    static FEED: OnceLock<Feed> = OnceLock::new();
    FEED.get_or_init(|| Feed {
        snapshot: Mutex::new(Snapshot::default()),
        changed: Condvar::new(),
    })
}
pub fn bump_sessions() {
    publish(0, crate::api::sessions(0));
}

pub fn bump_queues() {
    publish(4, crate::api::queues_topic(0));
}

pub fn bump_jobs() {
    publish(5, crate::api::jobs_topic(0));
}

pub fn bump_prs() {
    publish(6, crate::api::prs_topic(0));
}

pub(crate) fn publish_models(value: Value) {
    publish(1, value);
}
fn publish(index: usize, value: Value) {
    let f = feed();
    let mut s = f.snapshot.lock().unwrap();
    if s.values[index].as_ref() != Some(&value) {
        s.values[index] = Some(value);
        s.versions[index] += 1;
        f.changed.notify_all();
    }
}

pub fn begin_turn(session_id: &str, baseline: Vec<Value>, prompt: String) -> u64 {
    static NEXT: OnceLock<AtomicU64> = OnceLock::new();
    let id = NEXT.get_or_init(|| AtomicU64::new(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64)).fetch_add(1, Ordering::Relaxed);
    let f = feed();
    let mut s = f.snapshot.lock().unwrap();
    s.turns.retain(|_, t| t.completed.is_none_or(|at| at.elapsed() < Duration::from_secs(120)));
    s.turns.insert(id, Turn { session_id: session_id.to_string(), baseline, prompt, rows: Vec::new(), completed: None, completed_at_ms: None });
    s.turn_version += 1;
    f.changed.notify_all();
    id
}

pub(crate) fn if_session_idle<T>(id: &str, f: impl FnOnce() -> T) -> Option<T> {
    let snapshot = feed().snapshot.lock().unwrap();
    if snapshot.turns.values().any(|turn| turn.session_id == id && turn.completed.is_none()) {
        return None;
    }
    Some(f())
}

pub fn turn_row(id: u64, row: &Value) {
    let f = feed();
    let mut s = f.snapshot.lock().unwrap();
    if let Some(turn) = s.turns.get_mut(&id) {
        turn.rows.push(row.clone());
        if row.get("type").and_then(Value::as_str) == Some("done") {
            turn.completed = Some(Instant::now());
            turn.completed_at_ms = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64);
        }
        s.turn_version += 1;
        f.changed.notify_all();
    }
}

pub fn end_turn(id: u64) {
    let f = feed();
    let mut s = f.snapshot.lock().unwrap();
    if let Some(turn) = s.turns.get_mut(&id) {
        if turn.completed.is_none() {
            turn.rows.push(json!({"type":"done", "ok":false, "error":"Turn ended unexpectedly", "sessionId":turn.session_id}));
            turn.completed = Some(Instant::now());
            turn.completed_at_ms = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64);
            s.turn_version += 1;
            f.changed.notify_all();
        }
    }
}

fn sandbox_delta(previous: &Value, current: &Value) -> Value {
    let mut patch = delta(previous, current, "sandboxes");
    if let Some(map) = patch.as_object_mut() {
        map.remove("projectRoot");
        map.insert("sboxUp".into(), current.get("sboxUp").cloned().unwrap_or(Value::Bool(false)));
    }
    patch
}

fn sandboxes_visible_to(viewer: Option<i64>, value: &Value) -> Value {
    let mut out = value.clone();
    let Some(rows) = out.get("sandboxes").and_then(Value::as_array).cloned() else {
        return out;
    };
    let keep: Vec<Value> = rows
        .into_iter()
        .filter(|r| {
            crate::api::sandbox_visible(
                viewer,
                r.get("agent").and_then(Value::as_str).unwrap_or_default(),
            )
        })
        .collect();
    if let Some(map) = out.as_object_mut() {
        map.insert("sandboxes".into(), Value::Array(keep));
    }
    out
}

fn visible_to(viewer: Option<i64>, value: &Value) -> Value {
    let Some(uid) = viewer else {
        return value.clone();
    };
    let Some(rows) = value.get("sessions").and_then(Value::as_array) else {
        return value.clone();
    };
    let names: Vec<String> = rows
        .iter()
        .filter_map(|r| r.get("id").and_then(Value::as_str).map(str::to_string))
        .collect();
    let keep: HashSet<String> = crate::api::visible_session_ids(Some(uid), &names)
        .into_iter()
        .collect();
    let mut out = value.clone();
    out["sessions"] = Value::Array(
        rows.iter()
            .filter(|r| {
                r.get("id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| keep.contains(id))
            })
            .cloned()
            .collect(),
    );
    out
}

fn queues_visible_to(viewer: Option<i64>, value: &Value) -> Value {
    let Some(uid) = viewer else {
        return value.clone();
    };
    let Some(map) = value.get("queues").and_then(Value::as_object) else {
        return value.clone();
    };
    let names: Vec<String> = map.keys().cloned().collect();
    let keep: HashSet<String> = crate::api::visible_session_ids(Some(uid), &names)
        .into_iter()
        .collect();
    let mut out = value.clone();
    if let Some(outer) = out.as_object_mut() {
        outer.insert(
            "queues".into(),
            Value::Object(
                map.iter()
                    .filter(|(sid, _)| keep.contains(*sid))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            ),
        );
    }
    out
}

fn turn_visible(viewer: Option<i64>, session_id: &str) -> bool {
    crate::api::session_open_to(viewer, session_id, "read")
}

pub fn start(state: Arc<State>) {
    let port = state.config.port;
    std::thread::spawn(move || {
        let server = crate::api::angela(port);
        let mut last = json!([]);
        loop {
            let up = match angela::sandbox::list_for_menu(&server.state.store) {
                Ok(rows) => {
                    last = rows;
                    true
                }
                Err(_) => false,
            };
            publish(3, json!({
                "sandboxes": last,
                "sboxUp": up,
            }));
            std::thread::sleep(Duration::from_secs(2));
        }
    });
    std::thread::spawn(move || {
        let server = crate::api::angela(state.config.port);
        let mut sessions = Value::Null;
        let mut session_at = Instant::now() - Duration::from_secs(4);
        let mut model_at = Instant::now() - Duration::from_secs(600);
        let mut jobs_at = Instant::now() - Duration::from_secs(3);
        let mut prs_at = Instant::now() - Duration::from_secs(3);
        loop {
            if session_at.elapsed() >= Duration::from_secs(4) {
                sessions = crate::api::sessions(state.config.port);
                publish(0, sessions.clone());
                session_at = Instant::now();
            }
            if model_at.elapsed() >= Duration::from_secs(600) {
                publish(1, crate::api::models());
                model_at = Instant::now();
            }
            publish(4, crate::api::queues_topic(state.config.port));
            if jobs_at.elapsed() >= Duration::from_secs(2) {
                publish(5, crate::api::jobs_topic(state.config.port));
                jobs_at = Instant::now();
            }
            if prs_at.elapsed() >= Duration::from_secs(2) {
                publish(6, crate::api::prs_topic(state.config.port));
                prs_at = Instant::now();
            }
            let mut pending = serde_json::Map::new();
            if let Some(rows) = sessions.get("sessions").and_then(Value::as_array) {
                for row in rows {
                    if let Some(id) = row.get("id").and_then(Value::as_str) {
                        let questions = server.state.approvals.pending(id);
                        if !questions.is_empty() {
                            pending.insert(id.to_string(), json!(questions));
                        }
                    }
                }
            }
            publish(2, Value::Object(pending));
            {
                let mut snapshot = feed().snapshot.lock().unwrap();
                snapshot.turns.retain(|_, t| t.completed.is_none_or(|at| at.elapsed() < Duration::from_secs(120)));
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

fn delta(previous: &Value, current: &Value, key: &str) -> Value {
    let old = previous
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let new = current
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let lookup = |rows: &[Value]| -> std::collections::HashMap<String, Value> {
        rows.iter()
            .filter_map(|v| {
                v.get("id")
                    .and_then(Value::as_str)
                    .map(|id| (id.to_string(), v.clone()))
            })
            .collect()
    };
    let old_map = lookup(&old);
    let new_map = lookup(&new);
    let changed: Vec<Value> = new
        .iter()
        .filter(|v| {
            v.get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| old_map.get(id) != Some(*v))
        })
        .cloned()
        .collect();
    let removed: Vec<String> = old_map
        .keys()
        .filter(|id| !new_map.contains_key(*id))
        .cloned()
        .collect();
    let order: Vec<&str> = new
        .iter()
        .filter_map(|v| v.get("id").and_then(Value::as_str))
        .collect();
    json!({"changed":changed, "removed":removed, "order":order, "projectRoot":current.get("projectRoot")})
}

fn approval_delta(previous: &Value, current: &Value) -> Value {
    let old = previous.as_object();
    let new = current.as_object();
    let mut changed = serde_json::Map::new();
    if let Some(new) = new {
        for (key, value) in new {
            if old.and_then(|m| m.get(key)) != Some(value) {
                changed.insert(key.clone(), value.clone());
            }
        }
    }
    let removed: Vec<String> = old
        .into_iter()
        .flat_map(|m| m.keys())
        .filter(|key| new.and_then(|m| m.get(*key)).is_none())
        .cloned()
        .collect();
    json!({"changed":changed, "removed":removed})
}

pub fn stream(out: &mut Stream<'_>, viewer: Option<i64>) {
    if out
        .start(
            200,
            &[
                ("content-type", "text/event-stream"),
                ("cache-control", "no-cache"),
                ("x-accel-buffering", "no"),
            ],
        )
        .is_err()
    {
        return;
    }
    let f = feed();
    let mut seen = [0; 7];
    let mut previous: [Option<Value>; 7] = [None, None, None, None, None, None, None];
    let mut turns_seen: HashMap<u64, usize> = HashMap::new();
    let mut turn_version = 0;
    let mut initial = true;
    loop {
        let mut guard = f.snapshot.lock().unwrap();
        if !initial && guard.versions == seen && guard.turn_version == turn_version {
            let (g, _) = f
                .changed
                .wait_timeout(guard, Duration::from_secs(15))
                .unwrap();
            guard = g;
        }
        let mut frames = Vec::new();
        for (i, name) in ["sessions", "models", "approvals", "sandboxes", "queues", "jobs", "prs"].iter().enumerate() {
            if guard.versions[i] != seen[i] {
                seen[i] = guard.versions[i];
                if let Some(value) = &guard.values[i] {
                    let (old_f, new_f) = match *name {
                        "sessions" => (
                            previous[i].as_ref().map(|p| visible_to(viewer, p)),
                            visible_to(viewer, value),
                        ),
                        "sandboxes" => (
                            previous[i]
                                .as_ref()
                                .map(|p| sandboxes_visible_to(viewer, p)),
                            sandboxes_visible_to(viewer, value),
                        ),
                        "queues" => (
                            previous[i].as_ref().map(|p| queues_visible_to(viewer, p)),
                            queues_visible_to(viewer, value),
                        ),
                        _ => (previous[i].clone(), value.clone()),
                    };
                    let (event, data) = match (&old_f, *name) {
                        (Some(old), "sessions") => {
                            ("sessions-delta", delta(old, &new_f, "sessions"))
                        }
                        (Some(old), "approvals") => ("approvals-delta", approval_delta(old, &new_f)),
                        (Some(old), "sandboxes") => ("sandboxes-delta", sandbox_delta(old, &new_f)),
                        _ => (*name, new_f),
                    };
                    frames.push(format!("event: {event}\ndata: {data}\n\n"));
                    previous[i] = Some(value.clone());
                }
            }
        }
        turn_version = guard.turn_version;
        for (id, turn) in &guard.turns {
            if !turn_visible(viewer, &turn.session_id) {
                continue;
            }
            if let Some(offset) = turns_seen.get_mut(id) {
                for (index, row) in turn.rows.iter().enumerate().skip(*offset) {
                    let data = json!({"turnId":id.to_string(), "sessionId":turn.session_id, "seq":index, "row":row});
                    frames.push(format!("event: turn-event\ndata: {data}\n\n"));
                }
                *offset = turn.rows.len();
            } else {
                if !initial || turn.completed.is_none() {
                    let data = json!({"turnId":id.to_string(), "sessionId":turn.session_id,
                        "baseline":turn.baseline, "prompt":turn.prompt, "rows":turn.rows});
                    frames.push(format!("event: turn-snapshot\ndata: {data}\n\n"));
                }
                if initial {
                    if let Some(at) = turn.completed_at_ms {
                        let data = json!({"turnId":id.to_string(), "sessionId":turn.session_id, "completedAt":at});
                        frames.push(format!("event: turn-finished\ndata: {data}\n\n"));
                    }
                }
                turns_seen.insert(*id, turn.rows.len());
            }
        }
        initial = false;
        drop(guard);
        if frames.is_empty() {
            frames.push(": keepalive\n\n".to_string());
        }
        for frame in frames {
            if out.chunk(frame.as_bytes()).is_err() {
                return;
            }
        }
    }
}

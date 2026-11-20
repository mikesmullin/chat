//! One server-side sampler feeds all browsers. Only changed topics are sent.
use rustex1_devserver::{State, Stream};
use serde_json::{json, Value};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Default)]
struct Snapshot {
    values: [Option<Value>; 3],
    versions: [u64; 3],
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

/// The journal belongs to the server, not to the originating HTTP socket.
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

pub fn start(state: Arc<State>) {
    std::thread::spawn(move || {
        let server = crate::api::angela(state.config.port);
        let mut sessions = Value::Null;
        let mut session_at = Instant::now() - Duration::from_secs(4);
        let mut model_at = Instant::now() - Duration::from_secs(600);
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

/// Delta encoding keeps only rows that changed, plus the authoritative order.
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

/// A reconnect receives the latest snapshot before change-only updates resume.
pub fn stream(out: &mut Stream<'_>) {
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
    let mut seen = [0; 3];
    let mut previous: [Option<Value>; 3] = [None, None, None];
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
        for (i, name) in ["sessions", "models", "approvals"].iter().enumerate() {
            if guard.versions[i] != seen[i] {
                seen[i] = guard.versions[i];
                if let Some(value) = &guard.values[i] {
                    let (event, data) = match (&previous[i], *name) {
                        (Some(old), "sessions") => {
                            ("sessions-delta", delta(old, value, "sessions"))
                        }
                        (Some(old), "approvals") => ("approvals-delta", approval_delta(old, value)),
                        _ => (*name, value.clone()),
                    };
                    frames.push(format!("event: {event}\ndata: {data}\n\n"));
                    previous[i] = Some(value.clone());
                }
            }
        }
        turn_version = guard.turn_version;
        for (id, turn) in &guard.turns {
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
                // Completed turns that predate this connection come from the
                // persisted transcript, including after later feed wakes.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_turn_replays_on_reconnect_and_continues_in_order() {
        use std::io::Read;
        use std::net::{TcpListener, TcpStream};
        fn connect() -> TcpStream {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
            let (mut server, _) = listener.accept().unwrap();
            std::thread::spawn(move || { stream(&mut Stream::new(&mut server)); });
            client.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            client
        }
        fn read_until(socket: &mut TcpStream, wanted: &str) -> String {
            let mut result = String::new();
            let mut buf = [0; 8192];
            while !result.contains(wanted) {
                let n = socket.read(&mut buf).expect("expected event frame");
                assert!(n > 0);
                result.push_str(&String::from_utf8_lossy(&buf[..n]));
            }
            result
        }
        let id = begin_turn("test-reconnect-session", vec![json!({"kind":"user", "text":"before"})], "new prompt".into());
        turn_row(id, &json!({"type":"assistant_delta", "text":"first"}));
        let mut client = connect();
        let snapshot = read_until(&mut client, "event: turn-snapshot");
        assert!(snapshot.contains("test-reconnect-session"));
        assert!(snapshot.contains("new prompt"));
        assert!(snapshot.contains("first"));
        turn_row(id, &json!({"type":"assistant_delta", "text":"second"}));
        let live = read_until(&mut client, "event: turn-event");
        assert!(live.contains("second"));
        let mut reconnected = connect();
        let replay = read_until(&mut reconnected, "event: turn-snapshot");
        assert!(replay.contains("first"));
        assert!(replay.contains("second"));
        turn_row(id, &json!({"type":"done"}));
        let done = read_until(&mut reconnected, "event: turn-event");
        assert!(done.contains("done"));
        let mut later = connect();
        let finished = read_until(&mut later, "event: turn-finished");
        assert!(finished.contains("test-reconnect-session"));
        assert!(finished.contains("completedAt"));
    }

    #[test]
    fn session_delta_contains_only_changes_and_order() {
        let before = json!({"projectRoot":"/p", "sessions":[{"id":"a","title":"old"},{"id":"b","title":"same"},{"id":"c"}]});
        let after = json!({"projectRoot":"/p", "sessions":[{"id":"b","title":"same"},{"id":"a","title":"new"},{"id":"d"}]});
        let patch = delta(&before, &after, "sessions");
        assert_eq!(
            patch["changed"],
            json!([{"id":"a","title":"new"},{"id":"d"}])
        );
        assert_eq!(patch["removed"], json!(["c"]));
        assert_eq!(patch["order"], json!(["b", "a", "d"]));
    }

    #[test]
    fn approval_delta_carries_additions_and_clears() {
        let before = json!({"a":[{"id":"1"}], "b":[{"id":"2"}]});
        let after = json!({"b":[{"id":"3"}], "c":[{"id":"4"}]});
        let patch = approval_delta(&before, &after);
        assert_eq!(patch["changed"], json!({"b":[{"id":"3"}],"c":[{"id":"4"}]}));
        assert_eq!(patch["removed"], json!(["a"]));
    }
}

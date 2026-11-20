//! One server-side sampler feeds all browsers. Only changed topics are sent.
use rustex1_devserver::{State, Stream};
use serde_json::{json, Value};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Snapshot {
    values: [Option<Value>; 3],
    versions: [u64; 3],
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
    loop {
        let mut guard = f.snapshot.lock().unwrap();
        if guard.versions == seen {
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

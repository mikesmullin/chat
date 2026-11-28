use std::path::Path;

use serde_json::Value;

use super::store;

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn start(root: &Path) {
    let root = root.to_path_buf();
    std::thread::spawn(move || loop {
        tick(&root);
        std::thread::sleep(std::time::Duration::from_secs(30));
    });
}

fn tick(root: &Path) {
    let now = now_ms();
    let due = crate::db::with(root, |c| store::due_jobs(c, now)).unwrap_or_default();
    for job in due {
        let id = job.get("id").and_then(Value::as_i64).unwrap_or(0);
        let name = job.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
        let cron = job
            .get("cron")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_default();
        let next = super::registry::next_after(&cron, now).unwrap_or(0);
        let _ = crate::db::with(root, |c| {
            store::mark_job_fired(c, id, next);
            Ok::<(), String>(())
        });
        match super::queue::enqueue(root, &name, "cron", &serde_json::json!({}), None, None, None, None) {
            Ok(_) => {
                crate::events::bump_jobs();
            }
            Err(e) => {
                eprintln!("cron job {name}: could not enqueue: {}", e.message);
            }
        }
    }
}

pub fn ensure_scheduled(root: &Path) {
    let now = now_ms();
    let jobs = crate::db::with(root, |c| store::list_jobs(c)).unwrap_or_default();
    for job in jobs {
        let id = job.get("id").and_then(Value::as_i64).unwrap_or(0);
        let cron = job.get("cron").and_then(Value::as_str).map(str::to_string);
        let Some(cron) = cron else { continue };
        let (_, next, _) = crate::db::with(root, |c| store::job_flags(c, id)).unwrap_or((true, 0, 0));
        if next > 0 {
            continue;
        }
        if let Some(n) = super::registry::next_after(&cron, now) {
            let _ = crate::db::with(root, |c| store::set_job_flags(c, id, true, n));
        }
    }
}

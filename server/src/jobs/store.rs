use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

use crate::db::now_iso;

pub fn insert_job(
    c: &Connection,
    name: &str,
    description: &str,
    basedir: &str,
    cron: Option<&str>,
    env: &str,
    definition: &str,
    source_kind: &str,
    source_path: Option<&str>,
    source_sha: &str,
) -> Result<i64, String> {
    let now = now_iso();
    c.execute(
        "INSERT INTO jobs (name, description, basedir, cron, env, definition, source_kind, source_path, source_sha, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![name, description, basedir, cron, env, definition, source_kind, source_path, source_sha, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(c.last_insert_rowid())
}

pub fn update_job(
    c: &Connection,
    id: i64,
    description: &str,
    cron: Option<&str>,
    env: &str,
    definition: &str,
    source_path: Option<&str>,
    source_sha: &str,
) -> Result<(), String> {
    let now = now_iso();
    c.execute(
        "UPDATE jobs SET description = ?2, cron = ?3, env = ?4, definition = ?5, source_path = ?6, source_sha = ?7, updated_at = ?8 WHERE id = ?1",
        params![id, description, cron, env, definition, source_path, source_sha, now],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn job_row(row: &rusqlite::Row) -> rusqlite::Result<Value> {
    Ok(json!({
        "id": row.get::<_, i64>(0)?,
        "name": row.get::<_, String>(1)?,
        "description": row.get::<_, String>(2)?,
        "basedir": row.get::<_, String>(3)?,
        "cron": row.get::<_, Option<String>>(4)?,
        "env": crate::db::parse_json(&row.get::<_, String>(5)?),
        "definition": crate::db::parse_json(&row.get::<_, String>(6)?),
        "sourceKind": row.get::<_, String>(7)?,
        "sourcePath": row.get::<_, Option<String>>(8)?,
        "sourceSha": row.get::<_, String>(9)?,
        "createdAt": row.get::<_, String>(10)?,
        "updatedAt": row.get::<_, String>(11)?,
    }))
}

const JOB_COLS: &str =
    "id, name, description, basedir, cron, env, definition, source_kind, source_path, source_sha, created_at, updated_at";

pub fn job_flags(c: &Connection, id: i64) -> (bool, i64, i64) {
    c.query_row(
        "SELECT enabled, next_run_at, last_run_at FROM jobs WHERE id = ?1",
        [id],
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)),
    )
    .map(|(e, n, l)| (e != 0, n, l))
    .unwrap_or((true, 0, 0))
}

pub fn set_job_flags(c: &Connection, id: i64, enabled: bool, next_run_at: i64) -> Result<(), String> {
    c.execute(
        "UPDATE jobs SET enabled = ?2, next_run_at = ?3 WHERE id = ?1",
        params![id, if enabled { 1 } else { 0 }, next_run_at],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn mark_job_fired(c: &Connection, id: i64, next_run_at: i64) {
    let now = chrono::Utc::now().timestamp_millis();
    let _ = c.execute(
        "UPDATE jobs SET last_run_at = ?2, next_run_at = ?3 WHERE id = ?1",
        params![id, now, next_run_at],
    );
}

pub fn due_jobs(c: &Connection, now: i64) -> Vec<Value> {
    c.prepare(&format!("SELECT {JOB_COLS} FROM jobs WHERE enabled = 1 AND cron IS NOT NULL AND next_run_at > 0 AND next_run_at <= ?1"))
        .unwrap()
        .query_map(params![now], job_row)
        .unwrap()
        .flatten()
        .collect()
}

pub fn list_jobs(c: &Connection) -> Vec<Value> {
    c.prepare(&format!("SELECT {JOB_COLS} FROM jobs ORDER BY basedir, name"))
        .unwrap()
        .query_map([], job_row)
        .unwrap()
        .flatten()
        .collect()
}

pub fn job_by_name(c: &Connection, name: &str) -> Option<Value> {
    c.query_row(
        &format!("SELECT {JOB_COLS} FROM jobs WHERE name = ?1"),
        [name],
        job_row,
    )
    .optional()
    .ok()
    .flatten()
}

pub fn job_by_id(c: &Connection, id: i64) -> Option<Value> {
    c.query_row(
        &format!("SELECT {JOB_COLS} FROM jobs WHERE id = ?1"),
        [id],
        job_row,
    )
    .optional()
    .ok()
    .flatten()
}

pub fn delete_job(c: &Connection, id: i64) -> Result<usize, String> {
    c.execute("DELETE FROM jobs WHERE id = ?1", [id])
        .map_err(|e| e.to_string())
}

pub fn next_run_number(c: &Connection, job_id: i64) -> i64 {
    c.query_row(
        "SELECT COALESCE(MAX(number), 0) + 1 FROM runs WHERE job_id = ?1",
        [job_id],
        |r| r.get(0),
    )
    .unwrap_or(1)
}

pub fn insert_queue_item(
    c: &Connection,
    job_id: i64,
    source: &str,
    payload: &str,
) -> Result<i64, String> {
    let now = now_iso();
    c.execute(
        "INSERT INTO job_queue_items (job_id, source, payload, state, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'queued', ?4, ?4)",
        params![job_id, source, payload, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(c.last_insert_rowid())
}

pub fn claim_queue_item(c: &Connection) -> Option<i64> {
    let id: Option<i64> = c
        .query_row(
            "SELECT id FROM job_queue_items WHERE state = 'queued' ORDER BY id ASC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    let id = id?;
    let now = now_iso();
    c.execute(
        "UPDATE job_queue_items SET state = 'claimed', claimed_at = ?2, updated_at = ?2 WHERE id = ?1 AND state = 'queued'",
        params![id, now],
    )
    .ok()?;
    Some(id)
}

pub fn insert_run(
    c: &Connection,
    job_id: i64,
    job_name: &str,
    number: i64,
    queue_item_id: i64,
    trigger_source: &str,
    env: &str,
    repo_id: Option<&str>,
    pr_id: Option<i64>,
) -> Result<i64, String> {
    let now = now_iso();
    c.execute(
        "INSERT INTO runs (job_id, job_name, number, queue_item_id, trigger_source, state, env, repo_id, pr_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'queued', ?6, ?7, ?8, ?9, ?9)",
        params![job_id, job_name, number, queue_item_id, trigger_source, env, repo_id, pr_id, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(c.last_insert_rowid())
}

fn run_row(row: &rusqlite::Row) -> rusqlite::Result<Value> {
    Ok(json!({
        "id": row.get::<_, i64>(0)?,
        "jobId": row.get::<_, i64>(1)?,
        "jobName": row.get::<_, String>(2)?,
        "number": row.get::<_, i64>(3)?,
        "triggerSource": row.get::<_, String>(4)?,
        "state": row.get::<_, String>(5)?,
        "env": crate::db::parse_json(&row.get::<_, String>(6)?),
        "error": row.get::<_, Option<String>>(7)?,
        "repoId": row.get::<_, Option<String>>(8)?,
        "prId": row.get::<_, Option<i64>>(9)?,
        "createdAt": row.get::<_, String>(10)?,
        "startedAt": row.get::<_, Option<String>>(11)?,
        "endedAt": row.get::<_, Option<String>>(12)?,
        "updatedAt": row.get::<_, String>(13)?,
    }))
}

const RUN_COLS: &str =
    "id, job_id, job_name, number, trigger_source, state, env, error, repo_id, pr_id, created_at, started_at, ended_at, updated_at";

pub fn run_by_id(c: &Connection, id: i64) -> Option<Value> {
    c.query_row(
        &format!("SELECT {RUN_COLS} FROM runs WHERE id = ?1"),
        [id],
        run_row,
    )
    .optional()
    .ok()
    .flatten()
}

pub fn list_runs(c: &Connection, job_name: Option<&str>, state: &str, limit: i64) -> Vec<Value> {
    match (job_name, state) {
        (Some(job), "all") => c
            .prepare(&format!("SELECT {RUN_COLS} FROM runs WHERE job_name = ?1 ORDER BY id DESC LIMIT ?2"))
            .unwrap()
            .query_map(params![job, limit], run_row)
            .unwrap()
            .flatten()
            .collect(),
        (Some(job), s) => c
            .prepare(&format!("SELECT {RUN_COLS} FROM runs WHERE job_name = ?1 AND state = ?2 ORDER BY id DESC LIMIT ?3"))
            .unwrap()
            .query_map(params![job, s, limit], run_row)
            .unwrap()
            .flatten()
            .collect(),
        (None, "all") => c
            .prepare(&format!("SELECT {RUN_COLS} FROM runs ORDER BY id DESC LIMIT ?1"))
            .unwrap()
            .query_map(params![limit], run_row)
            .unwrap()
            .flatten()
            .collect(),
        (None, s) => c
            .prepare(&format!("SELECT {RUN_COLS} FROM runs WHERE state = ?1 ORDER BY id DESC LIMIT ?2"))
            .unwrap()
            .query_map(params![s, limit], run_row)
            .unwrap()
            .flatten()
            .collect(),
    }
}

pub fn runs_for_pr(c: &Connection, pr_id: i64) -> Vec<Value> {
    c.prepare(&format!("SELECT {RUN_COLS} FROM runs WHERE pr_id = ?1 ORDER BY id ASC"))
        .unwrap()
        .query_map(params![pr_id], run_row)
        .unwrap()
        .flatten()
        .map(|mut r| {
            let job = r.get("jobName").and_then(Value::as_str).unwrap_or_default().to_string();
            let number = r.get("number").and_then(Value::as_i64).unwrap_or(0);
            let basedir: String = c
                .query_row("SELECT basedir FROM jobs WHERE name = ?1", [&job], |row| {
                    row.get(0)
                })
                .unwrap_or_default();
            if let Some(obj) = r.as_object_mut() {
                let label = if basedir.is_empty() {
                    format!("{job}#{number}")
                } else {
                    format!("{basedir}:{job}#{number}")
                };
                obj.insert("label".into(), json!(label));
            }
            r
        })
        .collect()
}

pub fn run_ids_for_pr(c: &Connection, pr_id: i64) -> Vec<i64> {
    runs_for_pr(c, pr_id)
        .into_iter()
        .filter_map(|r| r.get("id").and_then(Value::as_i64))
        .collect()
}

pub fn set_run_state(
    c: &Connection,
    id: i64,
    state: &str,
    error: Option<&str>,
    started: bool,
    ended: bool,
) -> Result<(), String> {
    let now = now_iso();
    if started {
        c.execute(
            "UPDATE runs SET state = ?2, error = ?3, started_at = COALESCE(started_at, ?4), updated_at = ?4 WHERE id = ?1",
            params![id, state, error, now],
        )
        .map_err(|e| e.to_string())?;
    } else if ended {
        c.execute(
            "UPDATE runs SET state = ?2, error = ?3, ended_at = ?4, updated_at = ?4 WHERE id = ?1",
            params![id, state, error, now],
        )
        .map_err(|e| e.to_string())?;
    } else {
        c.execute(
            "UPDATE runs SET state = ?2, error = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, state, error, now],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn insert_stage(
    c: &Connection,
    run_id: i64,
    name: &str,
    position: i64,
    group_name: Option<&str>,
) -> Result<i64, String> {
    let now = now_iso();
    c.execute(
        "INSERT INTO run_stages (run_id, name, position, group_name, state, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?5)",
        params![run_id, name, position, group_name, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(c.last_insert_rowid())
}

pub fn update_stage(
    c: &Connection,
    run_id: i64,
    name: &str,
    state: &str,
    sandbox_id: Option<&str>,
    session_id: Option<&str>,
    error: Option<&str>,
    started: bool,
    ended: bool,
) -> Result<(), String> {
    let now = now_iso();
    c.execute(
        "UPDATE run_stages SET state = ?3,
           sandbox_id = COALESCE(?4, sandbox_id),
           session_id = COALESCE(?5, session_id),
           error = COALESCE(?6, error),
           started_at = CASE WHEN ?7 THEN COALESCE(started_at, ?8) ELSE started_at END,
           ended_at = CASE WHEN ?8 IS NOT NULL AND ?9 THEN ?8 ELSE ended_at END,
           updated_at = ?8
         WHERE run_id = ?1 AND name = ?2",
        params![run_id, name, state, sandbox_id, session_id, error, started, now, ended],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn stage_rows(c: &Connection, run_id: i64) -> Vec<Value> {
    c.prepare(
        "SELECT name, position, group_name, state, sandbox_id, session_id, artifacts, error, created_at, started_at, ended_at
         FROM run_stages WHERE run_id = ?1 ORDER BY position ASC, id ASC",
    )
    .unwrap()
    .query_map([run_id], |row| {
        Ok(json!({
            "name": row.get::<_, String>(0)?,
            "position": row.get::<_, i64>(1)?,
            "group": row.get::<_, Option<String>>(2)?,
            "state": row.get::<_, String>(3)?,
            "sandboxId": row.get::<_, Option<String>>(4)?,
            "sessionId": row.get::<_, Option<String>>(5)?,
            "artifacts": crate::db::parse_json(&row.get::<_, String>(6)?),
            "error": row.get::<_, Option<String>>(7)?,
            "createdAt": row.get::<_, String>(8)?,
            "startedAt": row.get::<_, Option<String>>(9)?,
            "endedAt": row.get::<_, Option<String>>(10)?,
        }))
    })
    .unwrap()
    .flatten()
    .collect()
}

pub fn set_stage_artifacts(c: &Connection, run_id: i64, name: &str, artifacts: &Value) {
    let _ = c.execute(
        "UPDATE run_stages SET artifacts = ?3 WHERE run_id = ?1 AND name = ?2",
        params![run_id, name, artifacts.to_string()],
    );
}

pub fn journal(
    c: &Connection,
    run_id: i64,
    stage: Option<&str>,
    kind: &str,
    level: &str,
    message: &str,
    data: &Value,
) {
    let now = now_iso();
    let _ = c.execute(
        "INSERT INTO journal_entries (run_id, stage_name, kind, level, message, data, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![run_id, stage, kind, level, message, data.to_string(), now],
    );
}

pub fn journal_tail(c: &Connection, run_id: i64, limit: i64) -> Vec<Value> {
    c.prepare(
        "SELECT id, stage_name, kind, level, message, data, created_at FROM journal_entries
         WHERE run_id = ?1 ORDER BY id DESC LIMIT ?2",
    )
    .unwrap()
    .query_map(params![run_id, limit], |row| {
        Ok(json!({
            "id": row.get::<_, i64>(0)?,
            "stage": row.get::<_, Option<String>>(1)?,
            "kind": row.get::<_, String>(2)?,
            "level": row.get::<_, String>(3)?,
            "message": row.get::<_, String>(4)?,
            "data": crate::db::parse_json(&row.get::<_, String>(5)?),
            "createdAt": row.get::<_, String>(6)?,
        }))
    })
    .unwrap()
    .flatten()
    .collect::<Vec<_>>()
    .into_iter()
    .rev()
    .collect()
}

pub fn pending_question(c: &Connection, run_id: i64) -> Option<Value> {
    let rows = journal_tail(c, run_id, 400);
    let mut asked: Option<Value> = None;
    for entry in rows {
        if entry.get("kind").and_then(Value::as_str) != Some("qa") {
            continue;
        }
        let data = entry.get("data").cloned().unwrap_or(Value::Null);
        if data.get("answer").is_some() {
            asked = None;
        } else {
            asked = Some(data);
        }
    }
    asked
}

pub fn insert_trigger(
    c: &Connection,
    job_id: i64,
    job_name: &str,
    source: &str,
    dedupe_key: Option<&str>,
    payload: &str,
    queue_item_id: i64,
) -> Result<(), String> {
    let now = now_iso();
    c.execute(
        "INSERT INTO triggers (job_id, job_name, source, dedupe_key, payload, queue_item_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![job_id, job_name, source, dedupe_key, payload, queue_item_id, now],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn trigger_seen(c: &Connection, job_id: i64, dedupe_key: &str) -> bool {
    c.query_row(
        "SELECT COUNT(*) FROM triggers WHERE job_id = ?1 AND dedupe_key = ?2",
        params![job_id, dedupe_key],
        |r| r.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}

pub fn list_triggers(c: &Connection, job_name: Option<&str>, limit: i64) -> Vec<Value> {
    let rows: Vec<Value> = match job_name {
        Some(job) => c
            .prepare(
                "SELECT id, job_name, source, dedupe_key, payload, created_at FROM triggers
                 WHERE job_name = ?1 ORDER BY id DESC LIMIT ?2",
            )
            .unwrap()
            .query_map(params![job, limit], trigger_row)
            .unwrap()
            .flatten()
            .collect(),
        None => c
            .prepare(
                "SELECT id, job_name, source, dedupe_key, payload, created_at FROM triggers
                 ORDER BY id DESC LIMIT ?1",
            )
            .unwrap()
            .query_map(params![limit], trigger_row)
            .unwrap()
            .flatten()
            .collect(),
    };
    rows
}

fn trigger_row(row: &rusqlite::Row) -> rusqlite::Result<Value> {
    Ok(json!({
        "id": row.get::<_, i64>(0)?,
        "jobName": row.get::<_, String>(1)?,
        "source": row.get::<_, String>(2)?,
        "dedupeKey": row.get::<_, Option<String>>(3)?,
        "payload": crate::db::parse_json(&row.get::<_, String>(4)?),
        "createdAt": row.get::<_, String>(5)?,
    }))
}

pub fn active_run_states() -> &'static [&'static str] {
    &["queued", "running", "blocked", "posting", "stopping"]
}

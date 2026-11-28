use std::path::Path;

use serde_json::{json, Map, Value};

use super::store;
use crate::err::ApiError;

fn json_map(v: &Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

fn resolve_env(
    root: &Path,
    definition: &Value,
    job_env: &Value,
    payload: &Value,
    repo_hint: Option<&str>,
    branch_hint: Option<&str>,
) -> (String, Option<String>, Option<String>, String) {
    let mut env = json_map(job_env);
    let params = definition.get("params").cloned().unwrap_or(json!({}));
    let overlay = payload.get("params").cloned().unwrap_or(json!({}));
    let mut repo_name = repo_hint.map(str::to_string);
    let mut branch = branch_hint.map(str::to_string);
    if let Some(map) = params.as_object() {
        for (key, spec) in map {
            let value = overlay
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| spec.get("default").and_then(Value::as_str).map(str::to_string));
            match (key.as_str(), value) {
                ("repo", Some(v)) if !v.trim().is_empty() => {
                    repo_name = Some(v.trim().to_string());
                }
                ("branch", Some(v)) if !v.trim().is_empty() => {
                    branch = Some(v.trim().to_string());
                }
                (_, Some(v)) => {
                    env.insert(key.clone(), json!(v));
                }
                _ => {}
            }
        }
    }
    let mut repo_id = None;
    if let Some(name) = &repo_name {
        let repos = crate::db::with(root, |c| super::super::vcs::store::list_repos(c)).unwrap_or_default();
        if let Some(repo) = repos.iter().find(|r| {
            r.get("name").and_then(Value::as_str) == Some(name.as_str())
                || r.get("id").and_then(Value::as_str) == Some(name.as_str())
        }) {
            repo_id = repo.get("id").and_then(Value::as_str).map(str::to_string);
            env.insert(
                "REPO_NAME".into(),
                json!(repo.get("name").and_then(Value::as_str).unwrap_or(name.as_str())),
            );
            env.insert("REPO_PATH".into(), json!(repo.get("path").and_then(Value::as_str).unwrap_or_default()));
            env.insert(
                "REPO_DEFAULT_BRANCH".into(),
                json!(repo.get("defaultBranch").and_then(Value::as_str).unwrap_or("main")),
            );
        }
    }
    if let Some(b) = &branch {
        env.insert("REPO_BRANCH".into(), json!(b));
    }
    (Value::Object(env).to_string(), repo_id, repo_name, branch.unwrap_or_default())
}

pub fn enqueue(
    root: &Path,
    job_name: &str,
    source: &str,
    payload: &Value,
    repo_hint: Option<&str>,
    branch_hint: Option<&str>,
    pr_id: Option<i64>,
    dedupe_key: Option<&str>,
) -> Result<Value, ApiError> {
    crate::db::with(root, |c| {
        let job = store::job_by_name(c, job_name).ok_or_else(|| {
            ApiError::not_found("job_not_found", format!("job '{job_name}' not found"))
        })?;
        let job_id = job.get("id").and_then(Value::as_i64).unwrap_or(0);
        if let Some(key) = dedupe_key {
            if store::trigger_seen(c, job_id, key) {
                return Ok(json!({ "deduplicated": true, "job": job_name }));
            }
        }
        let definition = job.get("definition").cloned().unwrap_or(Value::Null);
        let job_env = job.get("env").cloned().unwrap_or(json!({}));
        let (env, repo_id, _repo_name, _branch) =
            resolve_env(root, &definition, &job_env, payload, repo_hint, branch_hint);
        let number = store::next_run_number(c, job_id);
        let queue_item = store::insert_queue_item(c, job_id, source, &payload.to_string())?;
        let run_id = store::insert_run(
            c,
            job_id,
            job_name,
            number,
            queue_item,
            source,
            &env,
            repo_id.as_deref(),
            pr_id,
        )?;
        store::insert_trigger(
            c,
            job_id,
            job_name,
            source,
            dedupe_key,
            &payload.to_string(),
            queue_item,
        )?;
        Ok(json!({
            "runId": run_id,
            "number": number,
            "job": job_name,
            "queueItemId": queue_item,
        }))
    })?
}

pub fn queue_view(root: &Path) -> Result<Value, ApiError> {
    crate::db::with(root, |c| {
        let rows: Vec<Value> = c
            .prepare(
                "SELECT id, job_id, source, payload, state, created_at, claimed_at, updated_at
                 FROM job_queue_items ORDER BY id DESC LIMIT 100",
            )
            .unwrap()
            .query_map([], |row| {
                Ok(json!({
                    "id": row.get::<_, i64>(0)?,
                    "jobId": row.get::<_, i64>(1)?,
                    "source": row.get::<_, String>(2)?,
                    "payload": crate::db::parse_json(&row.get::<_, String>(3)?),
                    "state": row.get::<_, String>(4)?,
                    "createdAt": row.get::<_, String>(5)?,
                    "claimedAt": row.get::<_, Option<String>>(6)?,
                    "updatedAt": row.get::<_, String>(7)?,
                }))
            })
            .unwrap()
            .flatten()
            .collect();
        let counts = |state: &str| -> i64 {
            c.query_row(
                "SELECT COUNT(*) FROM job_queue_items WHERE state = ?1",
                [state],
                |r| r.get(0),
            )
            .unwrap_or(0)
        };
        Ok(json!({
            "items": rows,
            "depth": {
                "queued": counts("queued"),
                "claimed": counts("claimed"),
                "cancelled": counts("cancelled"),
            },
        }))
    })?
}

pub fn claim_next(c: &rusqlite::Connection) -> Option<i64> {
    store::claim_queue_item(c)
}
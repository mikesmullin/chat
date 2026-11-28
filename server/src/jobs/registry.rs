use std::path::Path;

use serde_json::{json, Value};

use super::store;
use crate::err::ApiError;

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn next_after(cron: &str, from_ms: i64) -> Option<i64> {
    use cron::Schedule;
    use std::str::FromStr;
    let normalized = if cron.split_whitespace().count() == 5 {
        format!("0 {cron}")
    } else {
        cron.to_string()
    };
    let schedule = Schedule::from_str(&normalized).ok()?;
    let from = chrono::DateTime::from_timestamp_millis(from_ms)?;
    let from = from.with_timezone(&chrono::Local);
    schedule.after(&from).next().map(|t| t.timestamp_millis())
}

fn basedir_for(root: &Path, jobfile_path: &str) -> String {
    let p = Path::new(jobfile_path);
    let parent = p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
    let repos = crate::db::with(root, |c| super::super::vcs::store::list_repos(c)).unwrap_or_default();
    for repo in &repos {
        let repo_path = repo.get("path").and_then(Value::as_str).unwrap_or_default();
        if !repo_path.is_empty() && (parent == repo_path || parent.starts_with(&format!("{repo_path}/"))) {
            return repo.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
        }
    }
    Path::new(&parent)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "jobs".to_string())
}

fn public_job(j: &Value) -> Value {
    j.clone()
}

pub fn register(root: &Path, body: &Value) -> Result<Value, ApiError> {
    let path = body.get("path").and_then(Value::as_str).unwrap_or_default().trim().to_string();
    let loaded = if !path.is_empty() {
        let loaded = super::jobfile::load_jobfile_path(&path)?;
        loaded
    } else {
        let text = body
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| ApiError::validation("a job needs a 'path' to a Jobfile on disk"))?;
        super::jobfile::load_jobfile_text(text, None, None)?
    };
    let name = loaded
        .definition
        .pointer("/job/name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let description = loaded
        .definition
        .pointer("/job/description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let cron = loaded
        .definition
        .pointer("/job/schedule/cron")
        .and_then(Value::as_str)
        .map(str::to_string);
    let env = loaded.definition.get("env").cloned().unwrap_or(json!({})).to_string();
    let mut stored = loaded.definition.clone();
    if let Some(obj) = stored.as_object_mut() {
        obj.insert("raw".into(), loaded.raw.clone());
        obj.insert("source".into(), loaded.source.clone());
    }
    let definition = stored.to_string();
    let source_kind = loaded.source.get("kind").and_then(Value::as_str).unwrap_or("path");
    let source_path = if source_kind == "path" { Some(path.clone()) } else { None };
    let source_sha = loaded
        .source
        .get("sha256")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let basedir = if source_path.is_some() {
        basedir_for(root, &path)
    } else {
        "inline".to_string()
    };

    crate::db::with(root, |c| {
        let existing = store::job_by_name(c, &name);
        let next = cron
            .as_deref()
            .and_then(|cr| next_after(cr, now_ms()))
            .unwrap_or(0);
        if let Some(job) = existing {
            let job_path = job.get("sourcePath").and_then(Value::as_str).unwrap_or_default().to_string();
            let same = (job_path.is_empty() && source_path.is_none())
                || Some(job_path.as_str()) == source_path.as_deref();
            if !same {
                return Err(ApiError::conflict(
                    "job_exists",
                    format!("job '{name}' is already registered from '{job_path}'"),
                ));
            }
            let id = job.get("id").and_then(Value::as_i64).unwrap_or(0);
            store::update_job(
                c,
                id,
                &description,
                cron.as_deref(),
                &env,
                &definition,
                source_path.as_deref(),
                &source_sha,
            )?;
            store::set_job_flags(c, id, true, next)?;
            Ok(json!({ "job": store::job_by_id(c, id) }))
        } else {
            let id = store::insert_job(
                c,
                &name,
                &description,
                &basedir,
                cron.as_deref(),
                &env,
                &definition,
                source_kind,
                source_path.as_deref(),
                &source_sha,
            )?;
            store::set_job_flags(c, id, true, next)?;
            Ok(json!({ "job": store::job_by_id(c, id) }))
        }
    })?
}

pub fn list(root: &Path) -> Result<Value, ApiError> {
    crate::db::with(root, |c| {
        let mut jobs = store::list_jobs(c);
        for job in &mut jobs {
            let id = job.get("id").and_then(Value::as_i64).unwrap_or(0);
            let (enabled, next_run_at, last_run_at) = store::job_flags(c, id);
            let display = format!(
                "{}:{}",
                job.get("basedir").and_then(Value::as_str).unwrap_or_default(),
                job.get("name").and_then(Value::as_str).unwrap_or_default()
            );
            if let Some(obj) = job.as_object_mut() {
                obj.insert("enabled".into(), json!(enabled));
                obj.insert("nextRunAt".into(), json!(next_run_at));
                obj.insert("lastRunAt".into(), json!(last_run_at));
                obj.insert("display".into(), json!(display));
            }
        }
        Ok(json!({ "jobs": jobs, "count": jobs.len(), "total": jobs.len() }))
    })?
}

pub fn get(root: &Path, name: &str) -> Result<Value, ApiError> {
    crate::db::with(root, |c| {
        let job = store::job_by_name(c, name)
            .ok_or_else(|| ApiError::not_found("job_not_found", format!("job '{name}' not found")))?;
        let mut out = public_job(&job);
        let id = out.get("id").and_then(Value::as_i64).unwrap_or(0);
        let (enabled, next_run_at, last_run_at) = store::job_flags(c, id);
        if let Some(obj) = out.as_object_mut() {
            obj.insert("enabled".into(), json!(enabled));
            obj.insert("nextRunAt".into(), json!(next_run_at));
            obj.insert("lastRunAt".into(), json!(last_run_at));
        }
        Ok(json!({ "job": out }))
    })?
}

pub fn remove(root: &Path, name: &str) -> Result<Value, ApiError> {
    crate::db::with(root, |c| {
        let job = store::job_by_name(c, name)
            .ok_or_else(|| ApiError::not_found("job_not_found", format!("job '{name}' not found")))?;
        let id = job.get("id").and_then(Value::as_i64).unwrap_or(0);
        let busy: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM runs WHERE job_id = ?1 AND state IN ('queued','running','blocked','posting','stopping')",
                [id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if busy > 0 {
            return Err(ApiError::conflict(
                "job_busy",
                format!("job '{name}' has {busy} unfinished run(s)"),
            ));
        }
        store::delete_job(c, id)?;
        Ok(json!({ "deleted": name }))
    })?
}

pub fn set_enabled(root: &Path, name: &str, enabled: bool) -> Result<Value, ApiError> {
    crate::db::with(root, |c| {
        let job = store::job_by_name(c, name)
            .ok_or_else(|| ApiError::not_found("job_not_found", format!("job '{name}' not found")))?;
        let id = job.get("id").and_then(Value::as_i64).unwrap_or(0);
        let cron = job.get("cron").and_then(Value::as_str).map(str::to_string);
        let next = if enabled {
            cron.as_deref()
                .and_then(|cr| next_after(cr, now_ms()))
                .unwrap_or(0)
        } else {
            0
        };
        store::set_job_flags(c, id, enabled, next)?;
        Ok(json!({ "job": store::job_by_id(c, id) }))
    })?
}

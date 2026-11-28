pub mod agentfile;
pub mod drivers;
pub mod jobfile;
pub mod orchestrator;
pub mod queue;
pub mod registry;
pub mod scheduler;
pub mod store;
pub mod triggers;
pub mod worktree;
pub mod yaml;

use std::path::Path;

use serde_json::{json, Value};

use crate::err::ApiError;

pub fn mod_label(run: &Value) -> String {
    let job_name = run.get("jobName").and_then(Value::as_str).unwrap_or_default();
    let number = run.get("number").and_then(Value::as_i64).unwrap_or(0);
    let root = crate::api::project_root();
    let basedir = crate::db::with(&root, |c| {
        store::job_by_name(c, job_name)
            .and_then(|j| j.get("basedir").and_then(Value::as_str).map(str::to_string))
            .unwrap_or_default()
    })
    .unwrap_or_default();
    if basedir.is_empty() {
        format!("{job_name}#{number}")
    } else {
        format!("{basedir}:{job_name}#{number}")
    }
}

fn job_dir(root: &Path, run: &Value) -> String {
    let job_name = run.get("jobName").and_then(Value::as_str).unwrap_or_default();
    crate::db::with(root, |c| {
        store::job_by_name(c, job_name)
            .and_then(|j| {
                j.get("sourcePath")
                    .and_then(Value::as_str)
                    .map(|p| {
                        std::path::Path::new(p)
                            .parent()
                            .map(|d| d.to_string_lossy().to_string())
                            .unwrap_or_default()
                    })
            })
            .unwrap_or_default()
    })
    .unwrap_or_default()
}

pub fn project_for(root: &Path, run: &Value) -> String {
    let dir = job_dir(root, run);
    let repo_path = run
        .get("env")
        .and_then(|e| e.get("REPO_PATH"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let projects = crate::db::with(root, |c| {
        c.prepare("SELECT name, settings FROM projects ORDER BY name")
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    crate::db::parse_json(&row.get::<_, String>(1)?),
                ))
            })
            .unwrap()
            .flatten()
            .collect::<Vec<(String, Value)>>()
    })
    .unwrap_or_default();
    for (name, settings) in &projects {
        let root_path = settings
            .get("workspaceRoot")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if root_path.is_empty() {
            continue;
        }
        let matches = (!dir.is_empty() && (dir == *root_path || dir.starts_with(&format!("{root_path}/"))))
            || (!repo_path.is_empty()
                && (repo_path == *root_path || repo_path.starts_with(&format!("{root_path}/"))));
        if matches {
            return name.clone();
        }
    }
    if !repo_path.is_empty() {
        if let Some(name) = crate::db::with(root, |c| {
            super::vcs::store::list_repos(c)
                .into_iter()
                .find(|r| r.get("path").and_then(Value::as_str) == Some(repo_path.as_str()))
                .and_then(|r| r.get("name").and_then(Value::as_str).map(str::to_string))
        })
        .unwrap_or_default()
        {
            return name;
        }
    }
    let basedir = mod_label(run)
        .split(':')
        .next()
        .unwrap_or_default()
        .to_string();
    if basedir.is_empty() {
        String::new()
    } else {
        format!("jobfiles/{basedir}")
    }
}

pub fn with_label(mut run: Value) -> Value {
    let label = mod_label(&run);
    if let Some(obj) = run.as_object_mut() {
        obj.insert("label".into(), json!(label));
    }
    run
}

pub fn handle(
    root: &Path,
    method: &str,
    action: &str,
    query: &str,
    body: &Value,
) -> Result<(u16, Value), ApiError> {
    if method == "GET" {
        let value = match action {
            "jobs" => registry::list(root)?,
            "job" => {
                let name = super::vcs::query_value(query, "name");
                let name = body
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .unwrap_or(name);
                registry::get(root, &name)?
            }
            "runs" => {
                let job = super::vcs::query_value(query, "job");
                let state = super::vcs::query_value(query, "state");
                let state = if state.is_empty() { "all" } else { &state };
                let limit: i64 = super::vcs::query_value(query, "limit")
                    .parse()
                    .unwrap_or(50);
                let job = if job.is_empty() { None } else { Some(job) };
                crate::db::with(root, |c| {
                    let runs: Vec<Value> = store::list_runs(c, job.as_deref(), state, limit)
                        .into_iter()
                        .map(with_label)
                        .collect();
                    json!({ "runs": runs, "count": runs.len(), "total": runs.len() })
                })?
            }
            "run" => {
                let id: i64 = super::vcs::query_value(query, "id")
                    .parse()
                    .or_else(|_| body.get("id").and_then(Value::as_i64).ok_or(()))
                    .unwrap_or(0);
                let journal: i64 = super::vcs::query_value(query, "journal")
                    .parse()
                    .unwrap_or(20);
                crate::db::with(root, |c| {
                    let run = store::run_by_id(c, id).ok_or_else(|| {
                        ApiError::not_found("run_not_found", format!("run {id} not found"))
                    })?;
                    let mut run = with_label(run);
                    run["stages"] = json!(store::stage_rows(c, id));
                    run["pendingQuestion"] = store::pending_question(c, id).unwrap_or(Value::Null);
                    let tail = if journal > 0 {
                        json!(store::journal_tail(c, id, journal))
                    } else {
                        json!([])
                    };
                    Ok::<Value, ApiError>(json!({ "run": run, "journalTail": tail }))
                })??
            }
            "queue" => queue::queue_view(root)?,
            "triggers" => {
                let job = super::vcs::query_value(query, "job");
                let job = if job.is_empty() { None } else { Some(job) };
                crate::db::with(root, |c| {
                    let rows = store::list_triggers(c, job.as_deref(), 100);
                    json!({ "triggers": rows, "count": rows.len(), "total": rows.len() })
                })?
            }
            _ => {
                return Err(ApiError::not_found(
                    "not_found",
                    format!("unknown jobs action {action}"),
                ))
            }
        };
        return Ok((200, value));
    }

    let value = match action {
        "job-register" => registry::register(root, body)?,
        "job-delete" => {
            let name = body.get("name").and_then(Value::as_str).unwrap_or_default();
            registry::remove(root, name)?
        }
        "job-enable" => {
            let name = body.get("name").and_then(Value::as_str).unwrap_or_default();
            let enabled = body.get("enabled").and_then(Value::as_bool).unwrap_or(true);
            registry::set_enabled(root, name, enabled)?
        }
        "run-enqueue" => {
            let name = body.get("name").and_then(Value::as_str).unwrap_or_default();
            let params = body.get("params").cloned().unwrap_or(json!({}));
            let payload = json!({ "params": params });
            let repo = body
                .get("repo")
                .and_then(Value::as_str)
                .map(str::to_string);
            let branch = body
                .get("branch")
                .and_then(Value::as_str)
                .map(str::to_string);
            let pr_id = body.get("prId").and_then(Value::as_i64);
            let queued = queue::enqueue(
                root,
                name,
                "manual",
                &payload,
                repo.as_deref(),
                branch.as_deref(),
                pr_id,
                None,
            )?;
            crate::events::bump_jobs();
            queued
        }
        "run-answer" => {
            let id = body
                .get("runId")
                .and_then(Value::as_i64)
                .or_else(|| body.get("runId").and_then(Value::as_str).and_then(|s| s.parse().ok()))
                .unwrap_or(0);
            let answer = body.get("answer").and_then(Value::as_str).unwrap_or_default();
            orchestrator::answer(root, id, answer)?
        }
        "run-stop" => {
            let id = body
                .get("runId")
                .and_then(Value::as_i64)
                .or_else(|| body.get("runId").and_then(Value::as_str).and_then(|s| s.parse().ok()))
                .unwrap_or(0);
            let force = body.get("force").and_then(Value::as_bool).unwrap_or(false);
            orchestrator::stop(root, id, force)?
        }
        _ => {
            return Err(ApiError::not_found(
                "not_found",
                format!("unknown jobs action {action}"),
            ))
        }
    };
    Ok((200, value))
}

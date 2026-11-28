use std::path::Path;

use rusqlite::Connection;
use serde_json::{json, Value};

use super::git;
use super::store;
use crate::err::{validation_field, ApiError};

pub const MIN_APPROVALS: i64 = 1;

pub const PR_STATES: &[&str] = &["open", "review", "approved", "merged", "rejected"];
pub const TERMINAL_STATES: &[&str] = &["merged", "rejected"];
pub const REVIEW_DECISIONS: &[&str] = &["comment", "approve", "request_changes", "reject"];

fn str_field(
    input: &Value,
    field: &str,
    max: usize,
    required: bool,
) -> Result<Option<String>, ApiError> {
    match input.get(field) {
        None | Some(Value::Null) => {
            if required {
                Err(validation_field(
                    field,
                    "required",
                    format!("field '{field}' is required and must be a string"),
                ))
            } else {
                Ok(None)
            }
        }
        Some(Value::String(s)) => {
            let v = s.clone();
            if required && v.trim().is_empty() {
                return Err(validation_field(
                    field,
                    "required",
                    format!("field '{field}' must not be empty"),
                ));
            }
            if v.len() > max {
                return Err(validation_field(
                    field,
                    &format!("max_length_{max}"),
                    format!("field '{field}' must be at most {max} characters"),
                ));
            }
            Ok(Some(v))
        }
        Some(_) => Err(validation_field(
            field,
            "not_a_string",
            format!("field '{field}' must be a string"),
        )),
    }
}

fn require_repo(c: &Connection, name: &str) -> Result<Value, ApiError> {
    store::repo_by_name(c, name)
        .ok_or_else(|| ApiError::not_found("repo_not_found", format!("repo '{name}' not found")))
}

fn parse_pr_number(raw: &str) -> Result<i64, ApiError> {
    let value: i64 = raw.trim().parse().unwrap_or(0);
    if value <= 0 {
        return Err(ApiError::not_found(
            "pr_not_found",
            format!("pr '{raw}' is not a positive integer"),
        ));
    }
    Ok(value)
}

fn pr_or_404(c: &Connection, repo_name: &str, number: i64) -> Result<Value, ApiError> {
    store::get_pr(c, repo_name, number).ok_or_else(|| {
        ApiError::not_found(
            "pr_not_found",
            format!("pr {number} not found in repo '{repo_name}'"),
        )
    })
}

pub fn is_approval_policy_met(approvals: i64, latest_decisive: Option<&str>) -> bool {
    approvals >= MIN_APPROVALS && latest_decisive == Some("approve")
}

pub fn transition_for_review(state: &str, decision: &str, policy_met: bool) -> Result<String, ApiError> {
    match decision {
        "reject" => Ok("rejected".to_string()),
        "request_changes" => Ok("review".to_string()),
        "approve" => {
            if policy_met {
                Ok("approved".to_string())
            } else {
                Ok("review".to_string())
            }
        }
        "comment" => Ok(if state == "open" { "review".to_string() } else { state.to_string() }),
        _ => Err(validation_field(
            "decision",
            "invalid_decision",
            format!(
                "field 'decision' must be one of {}",
                REVIEW_DECISIONS.join(", ")
            ),
        )),
    }
}

pub fn create_pr(c: &Connection, repo_name: &str, input: &Value) -> Result<Value, ApiError> {
    let repo = require_repo(c, repo_name)?;
    let source = str_field(input, "sourceBranch", 200, true)?
        .unwrap_or_default()
        .trim()
        .to_string();
    git::validate_branch_field("sourceBranch", &source)?;
    let target = match str_field(input, "targetBranch", 200, false)? {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => repo
            .get("defaultBranch")
            .and_then(Value::as_str)
            .unwrap_or("main")
            .to_string(),
    };
    git::validate_branch_field("targetBranch", &target)?;
    let title = str_field(input, "title", 200, true)?.unwrap_or_default();
    let description = str_field(input, "description", 10000, false)?.unwrap_or_default();
    let author = str_field(input, "author", 100, false)?
        .filter(|a| !a.trim().is_empty());

    let repo_path = Path::new(repo.get("path").and_then(Value::as_str).unwrap_or_default());
    let source_head = git::resolve_branch_head(repo_path, &source);
    if source_head.is_none() {
        return Err(ApiError::not_found(
            "branch_not_found",
            format!("branch '{source}' not found"),
        )
        .detail(json!({ "branch": source })));
    }
    if git::resolve_branch_head(repo_path, &target).is_none() {
        return Err(ApiError::not_found(
            "branch_not_found",
            format!("branch '{target}' not found"),
        )
        .detail(json!({ "branch": target })));
    }
    if source == target {
        return Err(validation_field(
            "targetBranch",
            "same_as_source",
            format!("sourceBranch and targetBranch must differ (both '{source}')"),
        ));
    }
    let repo_id = repo.get("id").and_then(Value::as_str).unwrap_or_default();
    if let Some(existing) = store::find_open_pr_number(c, repo_id, &source, &target) {
        return Err(ApiError::conflict(
            "pr_already_open",
            format!("an open PR already exists for '{source}' → '{target}'"),
        )
        .detail(json!({ "number": existing })));
    }
    let number = store::next_pr_number(c, repo_id);
    let pr_id = store::insert_pr(
        c,
        repo_id,
        number,
        &source,
        &target,
        &title,
        &description,
        author.as_deref(),
    )?;
    store::note_pr_number(c, repo_id, number);
    Ok(json!({ "pr": store::get_pr_by_id(c, pr_id) }))
}

pub fn get_pr_json(c: &Connection, repo_name: &str, raw_number: &str) -> Result<Value, ApiError> {
    require_repo(c, repo_name)?;
    let number = parse_pr_number(raw_number)?;
    let pr = pr_or_404(c, repo_name, number)?;
    let pr_id = pr.get("id").and_then(Value::as_i64).unwrap_or(0);
    let mut out = pr.clone();
    out["reviews"] = Value::Array(store::list_reviews(c, pr_id));
    out["attachments"] = Value::Array(store::list_attachments(c, pr_id));
    out["approvals"] = json!(store::count_approvals(c, pr_id));
    out["latestDecisive"] = json!(store::latest_decisive(c, pr_id));
    out["approved"] = json!(is_approval_policy_met(
        store::count_approvals(c, pr_id),
        store::latest_decisive(c, pr_id).as_deref()
    ));
    out["runIds"] = json!(crate::jobs::store::run_ids_for_pr(c, pr_id));
    Ok(json!({ "pr": out }))
}

pub fn list_prs_json(
    c: &Connection,
    repo_name: &str,
    state: &str,
    limit: i64,
    offset: i64,
) -> Result<Value, ApiError> {
    require_repo(c, repo_name)?;
    let state = if state.is_empty() { "open" } else { state };
    if state != "all" && state != "active" && !PR_STATES.contains(&state) {
        return Err(validation_field(
            "state",
            "invalid_state",
            format!("query parameter 'state' must be one of {}", {
                let mut all: Vec<&str> = PR_STATES.to_vec();
                all.push("active");
                all.push("all");
                all.join(", ")
            }),
        ));
    }
    if limit < 1 || limit > 200 {
        return Err(validation_field(
            "limit",
            "integer_1_200",
            "query parameter 'limit' must be an integer 1–200",
        ));
    }
    if offset < 0 {
        return Err(validation_field(
            "offset",
            "integer_min_0",
            "query parameter 'offset' must be an integer ≥ 0",
        ));
    }
    let (prs, total) = store::list_prs(c, repo_name, state, limit, offset);
    Ok(json!({ "prs": prs, "count": prs.len(), "total": total }))
}

pub fn add_review(
    c: &Connection,
    repo_name: &str,
    raw_number: &str,
    input: &Value,
) -> Result<Value, ApiError> {
    require_repo(c, repo_name)?;
    let number = parse_pr_number(raw_number)?;
    let pr = pr_or_404(c, repo_name, number)?;
    let state = pr.get("state").and_then(Value::as_str).unwrap_or_default();
    if TERMINAL_STATES.contains(&state) {
        return Err(ApiError::conflict(
            "state_frozen",
            format!("pr {number} is {state}; reviews are closed"),
        )
        .detail(json!({ "state": state })));
    }
    let reviewer = str_field(input, "reviewer", 100, true)?.unwrap_or_default();
    let decision = str_field(input, "decision", 20, true)?.unwrap_or_default();
    if !REVIEW_DECISIONS.contains(&decision.as_str()) {
        return Err(validation_field(
            "decision",
            "invalid_decision",
            format!(
                "field 'decision' must be one of {}",
                REVIEW_DECISIONS.join(", ")
            ),
        ));
    }
    let comment = str_field(input, "comment", 5000, false)?.unwrap_or_default();
    let pr_id = pr.get("id").and_then(Value::as_i64).unwrap_or(0);
    let review_id = store::insert_review(c, pr_id, &reviewer, &decision, &comment)?;
    let approvals = store::count_approvals(c, pr_id);
    let latest = store::latest_decisive(c, pr_id);
    let policy_met = is_approval_policy_met(approvals, latest.as_deref());
    let next_state = transition_for_review(state, &decision, policy_met)?;
    if next_state == state {
        store::touch_pr(c, pr_id)?;
    } else {
        store::update_pr_state(c, pr_id, &next_state)?;
    }
    Ok(json!({
        "review": {
            "id": review_id,
            "reviewer": reviewer,
            "decision": decision,
            "comment": comment,
            "createdAt": crate::db::now_iso(),
        },
        "pr": store::get_pr_by_id(c, pr_id),
    }))
}

pub fn get_status(
    c: &Connection,
    repo_path: &Path,
    repo_name: &str,
    raw_number: &str,
) -> Result<Value, ApiError> {
    require_repo(c, repo_name)?;
    let number = parse_pr_number(raw_number)?;
    let pr = pr_or_404(c, repo_name, number)?;
    let pr_id = pr.get("id").and_then(Value::as_i64).unwrap_or(0);
    let state = pr.get("state").and_then(Value::as_str).unwrap_or_default();
    let approvals = store::count_approvals(c, pr_id);
    let latest = store::latest_decisive(c, pr_id);
    let approved = is_approval_policy_met(approvals, latest.as_deref());
    let mut status = json!({
        "state": state,
        "minApprovals": MIN_APPROVALS,
        "approvals": approvals,
        "latestDecisive": latest,
        "approved": approved,
        "mergeable": Value::Null,
        "conflictedFiles": [],
        "outOfDate": Value::Null,
        "baseHead": Value::Null,
        "sourceHead": Value::Null,
        "mergeCommit": pr.get("mergeCommit").cloned().unwrap_or(Value::Null),
    });
    if TERMINAL_STATES.contains(&state) {
        return Ok(json!({ "status": status, "checks": pr_checks(c, pr_id) }));
    }
    let target = pr.get("targetBranch").and_then(Value::as_str).unwrap_or_default();
    let source = pr.get("sourceBranch").and_then(Value::as_str).unwrap_or_default();
    let base_head = git::resolve_branch_head(repo_path, target);
    let source_head = git::resolve_branch_head(repo_path, source);
    if let (Some(base), Some(src)) = (&base_head, &source_head) {
        status["baseHead"] = json!(base);
        status["sourceHead"] = json!(src);
        status["outOfDate"] = json!(!git::is_ancestor(repo_path, base, src));
        match git::merge_tree_probe(repo_path, base, src) {
            git::MergeProbe::Clean { .. } => {
                status["mergeable"] = json!("clean");
            }
            git::MergeProbe::Conflicts { files } => {
                status["mergeable"] = json!("conflict");
                status["conflictedFiles"] = json!(files);
            }
            git::MergeProbe::Unknown => {
                status["mergeable"] = Value::Null;
            }
        }
    }
    Ok(json!({ "status": status, "checks": pr_checks(c, pr_id) }))
}

fn pr_checks(c: &Connection, pr_id: i64) -> Value {
    Value::Array(
        crate::jobs::store::runs_for_pr(c, pr_id)
            .into_iter()
            .map(|r| {
                json!({
                    "runId": r.get("id").cloned().unwrap_or(Value::Null),
                    "label": r.get("label").cloned().unwrap_or(Value::Null),
                    "state": r.get("state").cloned().unwrap_or(Value::Null),
                })
            })
            .collect::<Vec<_>>(),
    )
}

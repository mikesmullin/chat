use std::path::Path;

use rusqlite::Connection;
use serde_json::{json, Value};

use super::engine::{self, TERMINAL_STATES};
use super::git;
use super::store;
use crate::err::ApiError;

fn identity(root: &Path) -> (String, String) {
    let name = crate::db::with(root, |c| {
        c.query_row(
            "SELECT value FROM settings WHERE key = 'vcs__merge_name'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_default()
    })
    .unwrap_or_default();
    let email = crate::db::with(root, |c| {
        c.query_row(
            "SELECT value FROM settings WHERE key = 'vcs__merge_email'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_default()
    })
    .unwrap_or_default();
    (
        if name.trim().is_empty() {
            "AVCS".to_string()
        } else {
            name.trim().to_string()
        },
        if email.trim().is_empty() {
            "avcs@localhost".to_string()
        } else {
            email.trim().to_string()
        },
    )
}

fn check_gate(c: &Connection, pr_id: i64) -> Result<(), ApiError> {
    let runs = crate::jobs::store::runs_for_pr(c, pr_id);
    for run in &runs {
        let state = run.get("state").and_then(Value::as_str).unwrap_or_default();
        let label = run.get("label").and_then(Value::as_str).unwrap_or_default();
        match state {
            "success" | "aborted" => {}
            "failure" => {
                return Err(ApiError::conflict(
                    "check_failed",
                    format!("check '{label}' failed; merge requires green checks"),
                )
                .detail(json!({ "label": label, "state": state })));
            }
            _ => {
                return Err(ApiError::conflict(
                    "check_pending",
                    format!("check '{label}' is {state}; merge requires green checks"),
                )
                .detail(json!({ "label": label, "state": state })));
            }
        }
    }
    Ok(())
}

pub fn merge_pr(
    root: &Path,
    c: &Connection,
    repo_name: &str,
    raw_number: &str,
    merged_by: Option<&str>,
) -> Result<Value, ApiError> {
    let _ = engine::get_pr_json(c, repo_name, raw_number)?;
    let number: i64 = raw_number.trim().parse().unwrap_or(0);
    let pr = store::get_pr(c, repo_name, number).ok_or_else(|| {
        ApiError::not_found("pr_not_found", format!("pr {raw_number} not found"))
    })?;
    let pr_id = pr.get("id").and_then(Value::as_i64).unwrap_or(0);
    let state = pr.get("state").and_then(Value::as_str).unwrap_or_default();

    if state == "open" || state == "review" {
        return Err(ApiError::conflict(
            "pr_not_approved",
            format!("pr {number} is '{state}'; merge requires 'approved'"),
        )
        .detail(json!({ "state": state })));
    }
    if TERMINAL_STATES.contains(&state) {
        return Err(ApiError::conflict(
            "invalid_state",
            format!("pr {number} is '{state}' (terminal); merge not allowed"),
        )
        .detail(json!({ "state": state })));
    }
    check_gate(c, pr_id)?;

    let repo = store::repo_by_name(c, repo_name)
        .ok_or_else(|| ApiError::not_found("repo_not_found", format!("repo '{repo_name}' not found")))?;
    let repo_path = Path::new(repo.get("path").and_then(Value::as_str).unwrap_or_default());
    let target = pr.get("targetBranch").and_then(Value::as_str).unwrap_or_default().to_string();
    let source = pr.get("sourceBranch").and_then(Value::as_str).unwrap_or_default().to_string();

    let target_sha = git::resolve_branch_head(repo_path, &target).ok_or_else(|| {
        ApiError::not_found("branch_not_found", format!("branch '{target}' not found"))
            .detail(json!({ "branch": target }))
    })?;
    let source_sha = git::resolve_branch_head(repo_path, &source).ok_or_else(|| {
        ApiError::not_found("branch_not_found", format!("branch '{source}' not found"))
            .detail(json!({ "branch": source }))
    })?;

    if git::is_ancestor(repo_path, &source_sha, &target_sha) {
        return Err(ApiError::conflict(
            "merge_noop",
            format!("source '{source}' is already an ancestor of '{target}'; nothing to merge"),
        )
        .detail(json!({ "sourceSha": source_sha, "targetSha": target_sha })));
    }

    let tree = match git::merge_tree_probe(repo_path, &target_sha, &source_sha) {
        git::MergeProbe::Clean { tree } => tree,
        git::MergeProbe::Conflicts { files } => {
            return Err(ApiError::conflict(
                "merge_conflict",
                format!("merge of '{source}' into '{target}' conflicts"),
            )
            .detail(json!({ "conflictedFiles": files })));
        }
        git::MergeProbe::Unknown => {
            return Err(ApiError::internal("merge probe failed"));
        }
    };

    let title = pr.get("title").and_then(Value::as_str).unwrap_or_default();
    let subject = format!("Merge pull request #{number} from {source}");
    let (name, email) = identity(root);
    let env = [
        ("GIT_AUTHOR_NAME", name.as_str()),
        ("GIT_AUTHOR_EMAIL", email.as_str()),
        ("GIT_COMMITTER_NAME", name.as_str()),
        ("GIT_COMMITTER_EMAIL", email.as_str()),
    ];
    let out = git::run_env(
        repo_path,
        &[
            "commit-tree",
            &tree,
            "-p",
            &target_sha,
            "-p",
            &source_sha,
            "-m",
            &subject,
            "-m",
            title,
        ],
        &env,
    )
    .map_err(ApiError::internal)?;
    if !out.status_ok {
        return Err(ApiError::internal(format!(
            "git commit-tree failed: {}",
            out.stderr.trim()
        )));
    }
    let commit = out.stdout_str().trim().to_string();
    git::git(repo_path, &["update-ref", &format!("refs/heads/{target}"), &commit])
        .map_err(ApiError::internal)?;

    store::update_pr_merge(c, pr_id, &commit, merged_by)?;

    Ok(json!({
        "pr": store::get_pr_by_id(c, pr_id),
        "merge": {
            "commit": commit,
            "sourceSha": source_sha,
            "targetSha": target_sha,
            "sourceBranch": source,
            "targetBranch": target,
            "mergedAt": crate::db::now_iso(),
        },
    }))
}

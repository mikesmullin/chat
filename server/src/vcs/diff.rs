use std::path::Path;

use super::engine;
use super::git;
use super::store;
use crate::err::ApiError;

pub fn pr_diff(
    c: &rusqlite::Connection,
    repo_path: &Path,
    repo_name: &str,
    raw_number: &str,
) -> Result<Vec<u8>, ApiError> {
    engine::get_pr_json(c, repo_name, raw_number)?;
    let number: i64 = raw_number.trim().parse().unwrap_or(0);
    let pr = store::get_pr(c, repo_name, number)
        .ok_or_else(|| ApiError::not_found("pr_not_found", format!("pr {raw_number} not found")))?;
    let target = pr.get("targetBranch").and_then(|v| v.as_str()).unwrap_or_default();
    let source = pr.get("sourceBranch").and_then(|v| v.as_str()).unwrap_or_default();
    git::resolve_branch_head(repo_path, target).ok_or_else(|| {
        ApiError::not_found("branch_not_found", format!("branch '{target}' not found"))
            .detail(serde_json::json!({ "branch": target }))
    })?;
    git::resolve_branch_head(repo_path, source).ok_or_else(|| {
        ApiError::not_found("branch_not_found", format!("branch '{source}' not found"))
            .detail(serde_json::json!({ "branch": source }))
    })?;
    let out = git::run(
        repo_path,
        &[
            "diff",
            &format!("refs/heads/{target}...refs/heads/{source}"),
        ],
    )
    .map_err(ApiError::internal)?;
    if !out.status_ok && out.code != 1 {
        return Err(ApiError::internal(format!(
            "git diff failed: {}",
            out.stderr.trim()
        )));
    }
    Ok(out.stdout)
}

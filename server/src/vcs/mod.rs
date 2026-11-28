pub mod agent_tool;
pub mod attachments;
pub mod diff;
pub mod engine;
pub mod git;
pub mod merge;
pub mod store;

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::err::ApiError;

pub fn id_for(prefix: &str, path: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{prefix}{h:012x}")
}

pub fn check_path(raw: &str) -> Result<PathBuf, ApiError> {
    let t = raw.trim();
    if t.is_empty() {
        return Err(ApiError::validation("a repo needs an absolute path"));
    }
    if !t.starts_with('/') {
        return Err(ApiError::validation(format!("path must be absolute: {t}")));
    }
    if t.contains(':') || t.contains(',') {
        return Err(ApiError::validation(
            "path cannot contain ':' or ',' — a mount spec is delimited by both",
        ));
    }
    let p = PathBuf::from(t);
    let canon = p
        .canonicalize()
        .map_err(|_| ApiError::validation(format!("no such directory: {t}")))?;
    if !canon.is_dir() {
        return Err(ApiError::validation(format!("not a directory: {t}")));
    }
    let s = canon.to_string_lossy().to_string();
    if s.contains(':') || s.contains(',') {
        return Err(ApiError::validation("resolved path contains ':' or ','"));
    }
    Ok(canon)
}

fn is_git_dir(p: &Path) -> bool {
    let g = p.join(".git");
    g.is_dir() || g.is_file()
}

pub fn query_value(query: &str, key: &str) -> String {
    let prefix = format!("{key}=");
    for pair in query.split('&') {
        if let Some(v) = pair.strip_prefix(&prefix) {
            return percent_decode(v);
        }
    }
    String::new()
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn repo_add(root: &Path, body: &Value) -> Result<Value, ApiError> {
    let path = check_path(body.get("path").and_then(Value::as_str).unwrap_or_default())?;
    if !is_git_dir(&path) {
        return Err(ApiError::validation(format!(
            "not a git repository: {}",
            path.display()
        )));
    }
    let path_s = path.to_string_lossy().to_string();
    let id = id_for("r", &path_s);
    let name = body
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path_s.clone())
        });
    let description = body
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let default_branch = body
        .get("defaultBranch")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|b| !b.is_empty())
        .unwrap_or("main")
        .to_string();
    git::validate_branch_field("defaultBranch", &default_branch)?;

    crate::db::with(root, |c| {
        if store::repo_by_name(c, &name).is_some() {
            return Err(ApiError::conflict(
                "repo_exists",
                format!("repo '{name}' already exists"),
            ));
        }
        if store::repo_by_id(c, &id).is_some() {
            return Err(ApiError::conflict(
                "repo_exists",
                format!("repo at '{path_s}' is already tracked"),
            ));
        }
        store::insert_repo(c, &id, &name, &path_s, &description, &default_branch)?;
        Ok(json!({
            "id": id,
            "name": name,
            "select": format!("{id}:{default_branch}"),
            "selectKind": "branch",
        }))
    })?
}

fn repo_update(root: &Path, body: &Value) -> Result<Value, ApiError> {
    let id = body.get("id").and_then(Value::as_str).unwrap_or_default();
    let patch = body
        .get("patch")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    crate::db::with(root, |c| {
        store::repo_by_id(c, id)
            .ok_or_else(|| ApiError::not_found("repo_not_found", format!("repo '{id}' not found")))?;
        store::update_repo(c, id, &patch)?;
        Ok(json!({ "repo": store::repo_by_id(c, id) }))
    })?
}

fn repo_remove(root: &Path, body: &Value) -> Result<Value, ApiError> {
    let id = body.get("id").and_then(Value::as_str).unwrap_or_default();
    crate::db::with(root, |c| {
        let repo = store::repo_by_id(c, id)
            .ok_or_else(|| ApiError::not_found("repo_not_found", format!("repo '{id}' not found")))?;
        let open = store::count_open_prs(c, id);
        if open > 0 {
            return Err(ApiError::conflict(
                "repo_has_open_prs",
                format!("repo has {open} open pr(s); resolve them before removing"),
            ));
        }
        store::delete_repo(c, id)?;
        let worktree_ids: Vec<String> = store::list_worktrees(c)
            .into_iter()
            .filter(|w| w.get("repoId").and_then(Value::as_str) == Some(id))
            .filter_map(|w| w.get("id").and_then(Value::as_str).map(str::to_string))
            .collect();
        for wid in worktree_ids {
            let _ = store::delete_worktree(c, &wid);
        }
        Ok(json!({ "removed": id, "repo": repo }))
    })?
}

fn branches_json(root: &Path, body: &Value, query: &str) -> Result<Value, ApiError> {
    let name = body
        .get("repo")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| query_value(query, "repo"));
    crate::db::with(root, |c| {
        let repo = store::repo_by_name(c, &name).ok_or_else(|| {
            ApiError::not_found("repo_not_found", format!("repo '{name}' not found"))
        })?;
        let path = PathBuf::from(repo.get("path").and_then(Value::as_str).unwrap_or_default());
        let default_branch = repo
            .get("defaultBranch")
            .and_then(Value::as_str)
            .unwrap_or("main")
            .to_string();
        let branches: Vec<Value> = git::branches(&path)
            .into_iter()
            .map(|(name, sha)| {
                json!({
                    "name": name,
                    "sha": sha,
                    "isDefault": name == default_branch,
                })
            })
            .collect();
        Ok(json!({ "branches": branches }))
    })?
}

fn pr_create(root: &Path, body: &Value, author: Option<&str>) -> Result<Value, ApiError> {
    let repo_name = body.get("repo").and_then(Value::as_str).unwrap_or_default().to_string();
    let result = crate::db::with(root, |c| {
        let mut input = body.clone();
        if input.get("author").is_none() {
            if let Some(obj) = input.as_object_mut() {
                if let Some(a) = author.filter(|a| !a.trim().is_empty()) {
                    obj.insert("author".into(), json!(a));
                }
            }
        }
        let created = engine::create_pr(c, &repo_name, &input)?;
        let pr_id = created
            .get("pr")
            .and_then(|p| p.get("id"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let mut attachments = Vec::new();
        if let Some(list) = body.get("attachments").and_then(Value::as_array) {
            for item in list {
                let source = item
                    .get("path")
                    .and_then(Value::as_str)
                    .or_else(|| item.as_str())
                    .unwrap_or_default();
                if source.trim().is_empty() {
                    continue;
                }
                let name = item.get("name").and_then(Value::as_str);
                attachments.push(attachments::attach_from_path(
                    root,
                    c,
                    pr_id,
                    source,
                    name,
                )?);
            }
        }
        let mut out = created;
        out["attachments"] = Value::Array(attachments);
        Ok::<Value, ApiError>(out)
    })??;
    crate::jobs::triggers::on_pr_opened(root, &repo_name, &result);
    Ok(result)
}

fn pr_review(root: &Path, body: &Value, reviewer: Option<&str>) -> Result<Value, ApiError> {
    let repo_name = body.get("repo").and_then(Value::as_str).unwrap_or_default();
    let number = body.get("number").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| {
        body.get("number")
            .and_then(Value::as_i64)
            .map(|n| n.to_string())
            .unwrap_or_default()
    });
    let mut input = body.clone();
    if let Some(obj) = input.as_object_mut() {
        if obj.get("reviewer").is_none() {
            if let Some(r) = reviewer.filter(|r| !r.trim().is_empty()) {
                obj.insert("reviewer".into(), json!(r));
            }
        }
    }
    crate::db::with(root, |c| engine::add_review(c, repo_name, &number, &input))?
}

fn pr_merge(root: &Path, body: &Value, merged_by: Option<&str>) -> Result<Value, ApiError> {
    let repo_name = body.get("repo").and_then(Value::as_str).unwrap_or_default().to_string();
    let number = body
        .get("number")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            body.get("number")
                .and_then(Value::as_i64)
                .map(|n| n.to_string())
                .unwrap_or_default()
        });
    let who = body
        .get("mergedBy")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| merged_by.map(str::to_string));
    let result = crate::db::with(root, |c| {
        merge::merge_pr(root, c, &repo_name, &number, who.as_deref())
    })??;
    Ok(result)
}

fn pr_get(root: &Path, query: &str, body: &Value, status: bool) -> Result<Value, ApiError> {
    let repo_name = body
        .get("repo")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| query_value(query, "repo"));
    let number = body
        .get("number")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            body.get("number")
                .and_then(Value::as_i64)
                .map(|n| n.to_string())
                .unwrap_or_else(|| query_value(query, "number"))
        });
    crate::db::with(root, |c| {
        let repo = store::repo_by_name(c, &repo_name).ok_or_else(|| {
            ApiError::not_found("repo_not_found", format!("repo '{repo_name}' not found"))
        })?;
        let path = PathBuf::from(repo.get("path").and_then(Value::as_str).unwrap_or_default());
        if status {
            engine::get_status(c, &path, &repo_name, &number)
        } else {
            engine::get_pr_json(c, &repo_name, &number)
        }
    })?
}

fn pr_list(root: &Path, query: &str, body: &Value) -> Result<Value, ApiError> {
    let repo_name = body
        .get("repo")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| query_value(query, "repo"));
    let state = body
        .get("state")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| query_value(query, "state"));
    let state = if state.is_empty() { "all".to_string() } else { state };
    let limit: i64 = query_value(query, "limit")
        .parse()
        .ok()
        .or_else(|| body.get("limit").and_then(Value::as_i64))
        .unwrap_or(50);
    let offset: i64 = query_value(query, "offset")
        .parse()
        .ok()
        .or_else(|| body.get("offset").and_then(Value::as_i64))
        .unwrap_or(0);
    crate::db::with(root, |c| {
        if repo_name.is_empty() {
            let prs = store::list_all_prs(c, &state, limit);
            let total = prs.len() as i64;
            return Ok(json!({ "prs": prs, "count": total, "total": total }));
        }
        engine::list_prs_json(c, &repo_name, &state, limit, offset)
    })?
}

fn pr_attach(root: &Path, query: &str, body: &Value, raw: &[u8]) -> Result<Value, ApiError> {
    let repo_name = body
        .get("repo")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| query_value(query, "repo"));
    let number = body
        .get("number")
        .and_then(Value::as_i64)
        .or_else(|| body.get("number").and_then(Value::as_str).and_then(|s| s.parse().ok()))
        .unwrap_or_else(|| query_value(query, "number").parse().unwrap_or(0));
    crate::db::with(root, |c| {
        let pr = store::get_pr(c, &repo_name, number).ok_or_else(|| {
            ApiError::not_found("pr_not_found", format!("pr {number} not found"))
        })?;
        let pr_id = pr.get("id").and_then(Value::as_i64).unwrap_or(0);
        let name = body
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string)
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| query_value(query, "name"));
        let name = name.as_str();
        let name = if name.is_empty() { "file" } else { name };
        if let Some(path) = body.get("path").and_then(Value::as_str).filter(|p| !p.trim().is_empty())
        {
            attachments::attach_from_path(root, c, pr_id, path, Some(name))
        } else if !raw.is_empty() {
            attachments::attach_bytes(root, c, pr_id, name, raw)
        } else {
            Err(ApiError::validation("an attachment needs a path or bytes"))
        }
    })?
}

fn pr_cleanup(root: &Path, body: &Value) -> Result<Value, ApiError> {
    let repo_name = body.get("repo").and_then(Value::as_str).unwrap_or_default().to_string();
    let number = body
        .get("number")
        .and_then(Value::as_i64)
        .or_else(|| body.get("number").and_then(Value::as_str).and_then(|s| s.parse().ok()))
        .unwrap_or(0);
    let delete_branch = body
        .get("deleteBranch")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let reap_worktree = body
        .get("reapWorktree")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    crate::db::with(root, |c| {
        let pr = store::get_pr(c, &repo_name, number).ok_or_else(|| {
            ApiError::not_found("pr_not_found", format!("pr {number} not found"))
        })?;
        let state = pr.get("state").and_then(Value::as_str).unwrap_or_default();
        if state != "merged" && state != "rejected" {
            return Err(ApiError::conflict(
                "invalid_state",
                format!("pr {number} is '{state}'; cleanup works on merged or rejected prs"),
            ));
        }
        let repo = store::repo_by_name(c, &repo_name).ok_or_else(|| {
            ApiError::not_found("repo_not_found", format!("repo '{repo_name}' not found"))
        })?;
        let repo_id = repo.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
        let path = PathBuf::from(repo.get("path").and_then(Value::as_str).unwrap_or_default());
        let source = pr
            .get("sourceBranch")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let mut notes = Vec::new();
        if reap_worktree {
            let ids: Vec<String> = store::list_worktrees(c)
                .into_iter()
                .filter(|w| {
                    w.get("repoId").and_then(Value::as_str) == Some(repo_id.as_str())
                        && w.get("branch").and_then(Value::as_str) == Some(source.as_str())
                })
                .filter_map(|w| w.get("id").and_then(Value::as_str).map(str::to_string))
                .collect();
            for id in ids {
                crate::hostgit::remove_worktree(root, &id)?;
                notes.push(format!("worktree {id} removed"));
            }
        }
        if delete_branch {
            let default_branch = repo
                .get("defaultBranch")
                .and_then(Value::as_str)
                .unwrap_or("main")
                .to_string();
            if source == default_branch {
                return Err(ApiError::validation(
                    "refusing to delete the default branch",
                ));
            }
            if let Some(target) = pr.get("targetBranch").and_then(Value::as_str) {
                let source_head = git::resolve_branch_head(&path, &source);
                let target_head = git::resolve_branch_head(&path, target);
                if let (Some(sh), Some(th)) = (source_head, target_head) {
                    if state == "merged" && !git::is_ancestor(&path, &sh, &th) {
                        return Err(ApiError::conflict(
                            "branch_not_merged",
                            format!("branch '{source}' is not fully merged into '{target}'"),
                        ));
                    }
                }
            }
            git::git(&path, &["branch", "-D", &source]).map_err(ApiError::internal)?;
            notes.push(format!("branch {source} deleted"));
        }
        Ok(json!({ "ok": true, "notes": notes }))
    })?
}

fn pr_delete(root: &Path, query: &str, body: &Value) -> Result<Value, ApiError> {
    let repo_name = body
        .get("repo")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| query_value(query, "repo"));
    let number = body
        .get("number")
        .and_then(Value::as_i64)
        .or_else(|| body.get("number").and_then(Value::as_str).and_then(|s| s.parse().ok()))
        .unwrap_or_else(|| query_value(query, "number").parse().unwrap_or(0));
    crate::db::with(root, |c| {
        let pr = store::get_pr(c, &repo_name, number).ok_or_else(|| {
            ApiError::not_found("pr_not_found", format!("pr {number} not found"))
        })?;
        let state = pr.get("state").and_then(Value::as_str).unwrap_or_default();
        if state != "merged" && state != "rejected" {
            return Err(ApiError::conflict(
                "invalid_state",
                format!("pr {number} is '{state}'; only merged or rejected prs can be deleted"),
            ));
        }
        let pr_id = pr.get("id").and_then(Value::as_i64).unwrap_or(0);
        let repo_id = pr.get("repoId").and_then(Value::as_str).unwrap_or_default().to_string();
        let number_kept = pr.get("number").and_then(Value::as_i64).unwrap_or(0);
        attachments::delete_pr_files(root, c, pr_id);
        store::delete_pr(c, pr_id)?;
        store::note_pr_number(c, &repo_id, number_kept);
        Ok(json!({ "deleted": number }))
    })?
}

pub fn handle(
    root: &Path,
    method: &str,
    action: &str,
    query: &str,
    body: &Value,
    raw: &[u8],
    actor: Option<&str>,
) -> Result<(u16, Value), ApiError> {
    if method == "GET" {
        let value = match action {
            "repos" => crate::db::with(root, |c| {
                json!({ "repos": store::list_repos(c), "worktrees": store::list_worktrees(c) })
            })?,
            "branches" => branches_json(root, body, query)?,
            "prs" => pr_list(root, query, body)?,
            "pr" => pr_get(root, query, body, false)?,
            "pr-status" => pr_get(root, query, body, true)?,
            _ => return Err(ApiError::not_found("not_found", format!("unknown vcs action {action}"))),
        };
        return Ok((200, value));
    }
    let value = match action {
        "repo-add" => repo_add(root, body)?,
        "repo-update" => repo_update(root, body)?,
        "repo-remove" => repo_remove(root, body)?,
        "pr-create" => pr_create(root, body, actor)?,
        "pr-review" => pr_review(root, body, actor)?,
        "pr-merge" => pr_merge(root, body, actor)?,
        "pr-attach" => pr_attach(root, query, body, raw)?,
        "pr-cleanup" => pr_cleanup(root, body)?,
        "pr-delete" => pr_delete(root, query, body)?,
        _ => return Err(ApiError::not_found("not_found", format!("unknown vcs action {action}"))),
    };
    crate::events::bump_prs();
    Ok((200, value))
}

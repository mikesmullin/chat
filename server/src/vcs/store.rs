use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

use crate::db::now_iso;

pub fn list_repos(c: &Connection) -> Vec<Value> {
    let mut st = c
        .prepare(
            "SELECT id, name, path, description, default_branch, settings, created_at, updated_at
             FROM repos ORDER BY name ASC",
        )
        .unwrap();
    st.query_map([], |row| {
        Ok(json!({
            "id": row.get::<_, String>(0)?,
            "name": row.get::<_, String>(1)?,
            "path": row.get::<_, String>(2)?,
            "description": row.get::<_, String>(3)?,
            "defaultBranch": row.get::<_, String>(4)?,
            "settings": crate::db::parse_json(&row.get::<_, String>(5)?),
            "createdAt": row.get::<_, String>(6)?,
            "updatedAt": row.get::<_, String>(7)?,
        }))
    })
    .unwrap()
    .flatten()
    .collect()
}

pub fn repo_by_name(c: &Connection, name: &str) -> Option<Value> {
    c.query_row(
        "SELECT id, name, path, description, default_branch, settings, created_at, updated_at
         FROM repos WHERE name = ?1",
        [name],
        |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "path": row.get::<_, String>(2)?,
                "description": row.get::<_, String>(3)?,
                "defaultBranch": row.get::<_, String>(4)?,
                "settings": crate::db::parse_json(&row.get::<_, String>(5)?),
                "createdAt": row.get::<_, String>(6)?,
                "updatedAt": row.get::<_, String>(7)?,
            }))
        },
    )
    .optional()
    .ok()
    .flatten()
}

pub fn repo_by_id(c: &Connection, id: &str) -> Option<Value> {
    c.query_row(
        "SELECT id, name, path, description, default_branch, settings, created_at, updated_at
         FROM repos WHERE id = ?1",
        [id],
        |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "path": row.get::<_, String>(2)?,
                "description": row.get::<_, String>(3)?,
                "defaultBranch": row.get::<_, String>(4)?,
                "settings": crate::db::parse_json(&row.get::<_, String>(5)?),
                "createdAt": row.get::<_, String>(6)?,
                "updatedAt": row.get::<_, String>(7)?,
            }))
        },
    )
    .optional()
    .ok()
    .flatten()
}

pub fn insert_repo(
    c: &Connection,
    id: &str,
    name: &str,
    path: &str,
    description: &str,
    default_branch: &str,
) -> Result<(), String> {
    let now = now_iso();
    c.execute(
        "INSERT INTO repos (id, name, path, description, default_branch, settings, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, '{}', ?6, ?6)",
        params![id, name, path, description, default_branch, now],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn update_repo(
    c: &Connection,
    id: &str,
    patch: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    let now = now_iso();
    if let Some(v) = patch.get("description").and_then(Value::as_str) {
        c.execute(
            "UPDATE repos SET description = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, v, now],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(v) = patch.get("defaultBranch").and_then(Value::as_str) {
        c.execute(
            "UPDATE repos SET default_branch = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, v, now],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(v) = patch.get("settings") {
        c.execute(
            "UPDATE repos SET settings = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, v.to_string(), now],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn delete_repo(c: &Connection, id: &str) -> Result<usize, String> {
    c.execute("DELETE FROM repos WHERE id = ?1", [id])
        .map_err(|e| e.to_string())
}

pub fn count_open_prs(c: &Connection, repo_id: &str) -> i64 {
    c.query_row(
        "SELECT COUNT(*) FROM prs WHERE repo_id = ?1 AND state IN ('open', 'review', 'approved')",
        [repo_id],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

pub fn list_worktrees(c: &Connection) -> Vec<Value> {
    let mut st = c
        .prepare(
            "SELECT id, repo_id, branch, path, session_id, branch_pattern, path_pattern, sandbox_id, created_at
             FROM worktrees ORDER BY created_at ASC",
        )
        .unwrap();
    st.query_map([], |row| {
        Ok(json!({
            "id": row.get::<_, String>(0)?,
            "repoId": row.get::<_, String>(1)?,
            "branch": row.get::<_, String>(2)?,
            "path": row.get::<_, String>(3)?,
            "sessionId": row.get::<_, String>(4)?,
            "branchPattern": row.get::<_, String>(5)?,
            "pathPattern": row.get::<_, String>(6)?,
            "sandboxId": row.get::<_, String>(7)?,
            "createdAt": row.get::<_, String>(8)?,
        }))
    })
    .unwrap()
    .flatten()
    .collect()
}

pub fn upsert_worktree(c: &Connection, w: &Value) -> Result<(), String> {
    let id = w.get("id").and_then(Value::as_str).unwrap_or_default();
    let now = now_iso();
    c.execute(
        "INSERT INTO worktrees (id, repo_id, branch, path, session_id, branch_pattern, path_pattern, sandbox_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(id) DO UPDATE SET
           repo_id = excluded.repo_id, branch = excluded.branch, path = excluded.path,
           session_id = excluded.session_id, branch_pattern = excluded.branch_pattern,
           path_pattern = excluded.path_pattern, sandbox_id = excluded.sandbox_id",
        params![
            id,
            w.get("repoId").and_then(Value::as_str).unwrap_or_default(),
            w.get("branch").and_then(Value::as_str).unwrap_or_default(),
            w.get("path").and_then(Value::as_str).unwrap_or_default(),
            w.get("sessionId").and_then(Value::as_str).unwrap_or_default(),
            w.get("branchPattern").and_then(Value::as_str).unwrap_or_default(),
            w.get("pathPattern").and_then(Value::as_str).unwrap_or_default(),
            w.get("sandboxId").and_then(Value::as_str).unwrap_or_default(),
            now,
        ],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn delete_worktree(c: &Connection, id: &str) -> Result<(), String> {
    c.execute("DELETE FROM worktrees WHERE id = ?1", [id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn next_pr_number(c: &Connection, repo_id: &str) -> i64 {
    let max_row: i64 = c
        .query_row(
            "SELECT COALESCE(MAX(number), 0) FROM prs WHERE repo_id = ?1",
            [repo_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let last_used = c
        .query_row("SELECT settings FROM repos WHERE id = ?1", [repo_id], |r| {
            r.get::<_, String>(0)
        })
        .ok()
        .map(|s| {
            crate::db::parse_json(&s)
                .get("lastPrNumber")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        })
        .unwrap_or(0);
    max_row.max(last_used) + 1
}

pub fn note_pr_number(c: &Connection, repo_id: &str, number: i64) {
    let settings = c
        .query_row("SELECT settings FROM repos WHERE id = ?1", [repo_id], |r| {
            r.get::<_, String>(0)
        })
        .unwrap_or_else(|_| "{}".to_string());
    let mut doc = crate::db::parse_json(&settings);
    let last = doc
        .get("lastPrNumber")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if number <= last {
        return;
    }
    if let Some(obj) = doc.as_object_mut() {
        obj.insert("lastPrNumber".into(), json!(number));
    }
    let _ = c.execute(
        "UPDATE repos SET settings = ?2 WHERE id = ?1",
        params![repo_id, doc.to_string()],
    );
}

const PR_COLS: &str = "p.id, p.repo_id, p.number, p.source_branch, p.target_branch, p.title, p.description, p.author, p.merged_by, p.state, p.merge_commit, p.created_at, p.updated_at, r.name";

fn pr_row_to_json(row: &rusqlite::Row) -> rusqlite::Result<Value> {
    Ok(json!({
        "id": row.get::<_, i64>(0)?,
        "repoId": row.get::<_, String>(1)?,
        "number": row.get::<_, i64>(2)?,
        "sourceBranch": row.get::<_, String>(3)?,
        "targetBranch": row.get::<_, String>(4)?,
        "title": row.get::<_, String>(5)?,
        "description": row.get::<_, String>(6)?,
        "author": row.get::<_, Option<String>>(7)?,
        "mergedBy": row.get::<_, Option<String>>(8)?,
        "state": row.get::<_, String>(9)?,
        "mergeCommit": row.get::<_, Option<String>>(10)?,
        "createdAt": row.get::<_, String>(11)?,
        "updatedAt": row.get::<_, String>(12)?,
        "repo": row.get::<_, String>(13)?,
    }))
}

pub fn insert_pr(
    c: &Connection,
    repo_id: &str,
    number: i64,
    source_branch: &str,
    target_branch: &str,
    title: &str,
    description: &str,
    author: Option<&str>,
) -> Result<i64, String> {
    let now = now_iso();
    c.execute(
        "INSERT INTO prs (repo_id, number, source_branch, target_branch, title, description, author, state, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'open', ?8, ?8)",
        params![repo_id, number, source_branch, target_branch, title, description, author, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(c.last_insert_rowid())
}

pub fn get_pr(c: &Connection, repo_name: &str, number: i64) -> Option<Value> {
    c.query_row(
        &format!(
            "SELECT {PR_COLS} FROM prs p JOIN repos r ON r.id = p.repo_id
             WHERE r.name = ?1 AND p.number = ?2"
        ),
        params![repo_name, number],
        pr_row_to_json,
    )
    .optional()
    .ok()
    .flatten()
}

pub fn get_pr_by_id(c: &Connection, pr_id: i64) -> Option<Value> {
    c.query_row(
        &format!(
            "SELECT {PR_COLS} FROM prs p JOIN repos r ON r.id = p.repo_id WHERE p.id = ?1"
        ),
        [pr_id],
        pr_row_to_json,
    )
    .optional()
    .ok()
    .flatten()
}

pub fn list_prs(
    c: &Connection,
    repo_name: &str,
    state: &str,
    limit: i64,
    offset: i64,
) -> (Vec<Value>, i64) {
    let rows: Vec<Value> = if state == "active" {
        c.prepare(&format!(
            "SELECT {PR_COLS} FROM prs p JOIN repos r ON r.id = p.repo_id
             WHERE r.name = ?1 AND p.state NOT IN ('merged', 'rejected')
             ORDER BY p.number DESC LIMIT ?2 OFFSET ?3"
        ))
        .unwrap()
        .query_map(params![repo_name, limit, offset], pr_row_to_json)
        .unwrap()
        .flatten()
        .collect()
    } else if state == "all" {
        c.prepare(&format!(
            "SELECT {PR_COLS} FROM prs p JOIN repos r ON r.id = p.repo_id
             WHERE r.name = ?1 ORDER BY p.number DESC LIMIT ?2 OFFSET ?3"
        ))
        .unwrap()
        .query_map(params![repo_name, limit, offset], pr_row_to_json)
        .unwrap()
        .flatten()
        .collect()
    } else {
        c.prepare(&format!(
            "SELECT {PR_COLS} FROM prs p JOIN repos r ON r.id = p.repo_id
             WHERE r.name = ?1 AND p.state = ?2 ORDER BY p.number DESC LIMIT ?3 OFFSET ?4"
        ))
        .unwrap()
        .query_map(params![repo_name, state, limit, offset], pr_row_to_json)
        .unwrap()
        .flatten()
        .collect()
    };
    let total: i64 = if state == "active" {
        c.query_row(
            "SELECT COUNT(*) FROM prs p JOIN repos r ON r.id = p.repo_id WHERE r.name = ?1 AND p.state NOT IN ('merged', 'rejected')",
            [repo_name],
            |r| r.get(0),
        )
        .unwrap_or(0)
    } else if state == "all" {
        c.query_row(
            "SELECT COUNT(*) FROM prs p JOIN repos r ON r.id = p.repo_id WHERE r.name = ?1",
            [repo_name],
            |r| r.get(0),
        )
        .unwrap_or(0)
    } else {
        c.query_row(
            "SELECT COUNT(*) FROM prs p JOIN repos r ON r.id = p.repo_id WHERE r.name = ?1 AND p.state = ?2",
            params![repo_name, state],
            |r| r.get(0),
        )
        .unwrap_or(0)
    };
    (rows, total)
}

pub fn list_all_prs(c: &Connection, state: &str, limit: i64) -> Vec<Value> {
    if state == "active" {
        c.prepare(&format!(
            "SELECT {PR_COLS} FROM prs p JOIN repos r ON r.id = p.repo_id
             WHERE p.state NOT IN ('merged', 'rejected') ORDER BY p.updated_at DESC LIMIT ?1"
        ))
        .unwrap()
        .query_map(params![limit], pr_row_to_json)
        .unwrap()
        .flatten()
        .collect()
    } else if state == "all" {
        c.prepare(&format!(
            "SELECT {PR_COLS} FROM prs p JOIN repos r ON r.id = p.repo_id ORDER BY p.updated_at DESC LIMIT ?1"
        ))
        .unwrap()
        .query_map(params![limit], pr_row_to_json)
        .unwrap()
        .flatten()
        .collect()
    } else {
        c.prepare(&format!(
            "SELECT {PR_COLS} FROM prs p JOIN repos r ON r.id = p.repo_id WHERE p.state = ?2 ORDER BY p.updated_at DESC LIMIT ?1"
        ))
        .unwrap()
        .query_map(params![limit, state], pr_row_to_json)
        .unwrap()
        .flatten()
        .collect()
    }
}

pub fn find_open_pr_number(c: &Connection, repo_id: &str, source: &str, target: &str) -> Option<i64> {
    c.query_row(
        "SELECT number FROM prs
         WHERE repo_id = ?1 AND source_branch = ?2 AND target_branch = ?3
           AND state NOT IN ('merged', 'rejected')
         ORDER BY number LIMIT 1",
        params![repo_id, source, target],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

pub fn update_pr_state(c: &Connection, pr_id: i64, state: &str) -> Result<(), String> {
    let now = now_iso();
    c.execute(
        "UPDATE prs SET state = ?2, updated_at = ?3 WHERE id = ?1",
        params![pr_id, state, now],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn update_pr_merge(c: &Connection, pr_id: i64, commit: &str, merged_by: Option<&str>) -> Result<(), String> {
    let now = now_iso();
    c.execute(
        "UPDATE prs SET state = 'merged', merge_commit = ?2, merged_by = ?3, updated_at = ?4 WHERE id = ?1",
        params![pr_id, commit, merged_by, now],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn touch_pr(c: &Connection, pr_id: i64) -> Result<(), String> {
    let now = now_iso();
    c.execute(
        "UPDATE prs SET updated_at = ?2 WHERE id = ?1",
        params![pr_id, now],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn delete_pr(c: &Connection, pr_id: i64) -> Result<(), String> {
    c.execute("DELETE FROM prs WHERE id = ?1", [pr_id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn insert_review(
    c: &Connection,
    pr_id: i64,
    reviewer: &str,
    decision: &str,
    comment: &str,
) -> Result<i64, String> {
    let now = now_iso();
    c.execute(
        "INSERT INTO reviews (pr_id, reviewer, decision, comment, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![pr_id, reviewer, decision, comment, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(c.last_insert_rowid())
}

pub fn list_reviews(c: &Connection, pr_id: i64) -> Vec<Value> {
    let mut st = c
        .prepare(
            "SELECT id, reviewer, decision, comment, created_at FROM reviews
             WHERE pr_id = ?1 ORDER BY created_at ASC, id ASC",
        )
        .unwrap();
    st.query_map([pr_id], |row| {
        Ok(json!({
            "id": row.get::<_, i64>(0)?,
            "reviewer": row.get::<_, String>(1)?,
            "decision": row.get::<_, String>(2)?,
            "comment": row.get::<_, String>(3)?,
            "createdAt": row.get::<_, String>(4)?,
        }))
    })
    .unwrap()
    .flatten()
    .collect()
}

pub fn count_approvals(c: &Connection, pr_id: i64) -> i64 {
    c.query_row(
        "SELECT COUNT(*) FROM reviews WHERE pr_id = ?1 AND decision = 'approve'",
        [pr_id],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

pub fn latest_decisive(c: &Connection, pr_id: i64) -> Option<String> {
    c.query_row(
        "SELECT decision FROM reviews WHERE pr_id = ?1 AND decision IN ('approve', 'request_changes')
         ORDER BY created_at DESC, id DESC LIMIT 1",
        [pr_id],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

pub fn insert_attachment(
    c: &Connection,
    pr_id: i64,
    name: &str,
    path: &str,
    size: i64,
    mime: &str,
) -> Result<i64, String> {
    let now = now_iso();
    c.execute(
        "INSERT INTO pr_attachments (pr_id, name, path, size, mime, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![pr_id, name, path, size, mime, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(c.last_insert_rowid())
}

pub fn list_attachments(c: &Connection, pr_id: i64) -> Vec<Value> {
    let mut st = c
        .prepare(
            "SELECT id, name, path, size, mime, created_at FROM pr_attachments
             WHERE pr_id = ?1 ORDER BY id ASC",
        )
        .unwrap();
    st.query_map([pr_id], |row| {
        Ok(json!({
            "id": row.get::<_, i64>(0)?,
            "name": row.get::<_, String>(1)?,
            "path": row.get::<_, String>(2)?,
            "size": row.get::<_, i64>(3)?,
            "mime": row.get::<_, String>(4)?,
            "createdAt": row.get::<_, String>(5)?,
        }))
    })
    .unwrap()
    .flatten()
    .collect()
}

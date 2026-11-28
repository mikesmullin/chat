use std::path::PathBuf;

use serde_json::{json, Value};

use super::{attachments, engine};

fn session_workspace(root: &std::path::Path, session: &str) -> String {
    crate::db::with(root, |c| {
        c.query_row(
            "SELECT COALESCE(NULLIF(workspace_root, ''), '') FROM conversation WHERE id = ?1",
            [session],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_default()
    })
    .unwrap_or_default()
}

fn resolve_attachment(base: &str, raw: &str) -> String {
    let trimmed = raw.trim();
    let candidate = if trimmed.starts_with('/') {
        PathBuf::from(trimmed)
    } else {
        PathBuf::from(base).join(trimmed)
    };
    if candidate.is_file() {
        return candidate.to_string_lossy().to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("/work/") {
        let mapped = PathBuf::from(base).join(rest);
        if mapped.is_file() {
            return mapped.to_string_lossy().to_string();
        }
    }
    trimmed.to_string()
}

pub fn register(agent: &mut agl::agent::Agent, session: &str) {
    let session_owned = session.to_string();
    let _ = agent.tool(
        "chat__vcs_open_pr",
        "Open a pull request against a tracked repo and attach files to its \
         description WITHOUT committing them (demo videos, notes, reports). \
         `attachments` are file paths — absolute, or relative to the session's \
         workspace; sandbox paths under /work map to the workspace. Returns \
         the PR number.",
        json!({
            "type": "object",
            "properties": {
                "repo": { "type": "string", "description": "tracked repo name, e.g. vibe-arcade" },
                "sourceBranch": { "type": "string", "description": "the branch carrying the work" },
                "targetBranch": { "type": "string", "description": "defaults to the repo's default branch" },
                "title": { "type": "string", "description": "pull request title" },
                "description": { "type": "string", "description": "markdown description" },
                "attachments": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "files to attach to the description instead of committing them"
                }
            }
        }),
        vec![
            "repo".to_string(),
            "sourceBranch".to_string(),
            "title".to_string(),
        ],
        move |args: &Value| {
            let root = crate::api::project_root();
            let repo = args
                .get("repo")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let mut input = args.clone();
            if let Some(obj) = input.as_object_mut() {
                obj.insert("author".into(), json!(format!("agent:{session_owned}")));
            }
            let created = crate::db::with(&root, |c| engine::create_pr(c, &repo, &input))
                .and_then(|r| r.map_err(|e| e.message))?;
            let pr = created.get("pr").cloned().unwrap_or(Value::Null);
            let pr_id = pr.get("id").and_then(Value::as_i64).unwrap_or(0);
            let number = pr.get("number").and_then(Value::as_i64).unwrap_or(0);
            let base = session_workspace(&root, &session_owned);
            let mut attached = Vec::new();
            if let Some(list) = args.get("attachments").and_then(Value::as_array) {
                crate::db::with(&root, |c| {
                    for item in list {
                        let Some(raw) = item.as_str() else { continue };
                        let path = resolve_attachment(&base, raw);
                        match attachments::attach_from_path(&root, c, pr_id, &path, None) {
                            Ok(row) => attached.push(row),
                            Err(e) => attached.push(json!({ "error": e.message, "path": raw })),
                        }
                    }
                    Ok::<(), String>(())
                })
                .and_then(|r| r)?;
            }
            crate::jobs::triggers::on_pr_opened(&root, &repo, &created);
            Ok(json!({
                "ok": true,
                "repo": repo,
                "number": number,
                "state": pr.get("state").cloned().unwrap_or(Value::Null),
                "attachments": attached,
            })
            .to_string())
        },
    );
}

use std::path::Path;

use serde_json::Value;

fn list_match(list: &Value, value: &str) -> bool {
    match list {
        Value::Array(items) => items.is_empty()
            || items
                .iter()
                .any(|i| i.as_str().map(|s| s == value).unwrap_or(false)),
        _ => true,
    }
}

pub fn on_pr_opened(root: &Path, repo_name: &str, created: &Value) {
    let Some(pr) = created.get("pr") else {
        return;
    };
    let number = pr.get("number").and_then(Value::as_i64).unwrap_or(0);
    let source_branch = pr
        .get("sourceBranch")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let target_branch = pr
        .get("targetBranch")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let pr_id = pr.get("id").and_then(Value::as_i64);
    let repo_id = pr.get("repoId").and_then(Value::as_str).map(str::to_string);

    let jobs = crate::db::with(root, |c| super::store::list_jobs(c)).unwrap_or_default();
    for job in jobs {
        let name = job.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
        let definition = job.get("definition").cloned().unwrap_or(Value::Null);
        let Some(spec) = definition.pointer("/on/pullRequest") else {
            continue;
        };
        let actions = spec.get("actions").cloned().unwrap_or(Value::Null);
        if !list_match(&actions, "opened") {
            continue;
        }
        if !list_match(&spec.get("repos").cloned().unwrap_or(Value::Null), repo_name) {
            continue;
        }
        if !list_match(&spec.get("branches").cloned().unwrap_or(Value::Null), &target_branch) {
            continue;
        }
        let payload = serde_json::json!({
            "params": {
                "repo": repo_name,
                "branch": source_branch,
            },
            "pr": { "repo": repo_name, "number": number },
        });
        let dedupe = format!("pr-opened:{repo_name}#{number}:{name}");
        let _ = super::queue::enqueue(
            root,
            &name,
            "trigger",
            &payload,
            repo_id.as_deref(),
            Some(&source_branch),
            pr_id,
            Some(&dedupe),
        );
        crate::events::bump_jobs();
    }
}

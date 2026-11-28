use std::path::{Path, PathBuf};

use super::super::vcs::git;

pub fn run_worktree_dir(root: &Path, run_id: i64) -> PathBuf {
    root.join(".angela")
        .join("vcs")
        .join("runs")
        .join(run_id.to_string())
        .join("wt")
}

fn tail(stderr: &str) -> String {
    let t = stderr.trim();
    if t.len() > 400 {
        t[t.len() - 400..].to_string()
    } else {
        t.to_string()
    }
}

pub fn create_run_worktree(
    repo_path: &Path,
    branch: &str,
    worktree_dir: &Path,
) -> Result<(String, Option<String>), String> {
    let _ = git::run(repo_path, &["worktree", "prune"]);
    if let Some(parent) = worktree_dir.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let dir_s = worktree_dir.to_string_lossy().to_string();
    let branch_ref = if branch.trim().is_empty() {
        String::new()
    } else {
        format!("refs/heads/{}", branch.trim())
    };
    let added = if branch_ref.is_empty() {
        git::run(repo_path, &["worktree", "add", "--detach", &dir_s])
    } else {
        git::run(
            repo_path,
            &["worktree", "add", "--detach", &dir_s, &branch_ref],
        )
    };
    let added = match added {
        Ok(out) if out.status_ok => out,
        _ => {
            let fallback = git::run(repo_path, &["worktree", "add", "--detach", &dir_s])
                .map_err(|e| format!("git worktree add failed: {e}"))?;
            if !fallback.status_ok {
                return Err(format!(
                    "git worktree add failed: {}",
                    tail(&fallback.stderr)
                ));
            }
            fallback
        }
    };
    let _ = added;

    let probe = git::run(worktree_dir, &["config", "--get", "remote.origin.url"]);
    if probe.map(|p| p.status_ok).unwrap_or(false) {
        let ext = git::run(repo_path, &["config", "extensions.worktreeConfig", "true"]);
        if ext.map(|e| e.status_ok).unwrap_or(false) {
            let override_write = git::run(
                worktree_dir,
                &["config", "--worktree", "remote.origin.url", ""],
            );
            if !override_write.map(|o| o.status_ok).unwrap_or(false) {
                eprintln!("worktree: push-prevention override failed (run proceeds)");
            }
        }
    }

    let head = git::git(worktree_dir, &["rev-parse", "HEAD"])?;
    let gitdir = git::git_common_dir(repo_path);
    Ok((head.trim().to_string(), gitdir))
}

pub fn remove_run_worktree(repo_path: &Path, worktree_dir: &Path) {
    if worktree_dir.as_os_str().is_empty() {
        return;
    }
    let dir_s = worktree_dir.to_string_lossy().to_string();
    let removed = git::run(repo_path, &["worktree", "remove", "--force", &dir_s])
        .map(|o| o.status_ok)
        .unwrap_or(false);
    if !removed && worktree_dir.exists() {
        let _ = std::fs::remove_dir_all(worktree_dir);
    }
    let _ = git::run(repo_path, &["worktree", "prune"]);
}

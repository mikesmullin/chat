use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Map, Value};

use crate::vcs::git;

fn registry_path(root: &Path) -> PathBuf {
    root.join(".angela").join("hosts.json")
}

pub fn import_legacy_registry(root: &Path) {
    let text = std::fs::read_to_string(registry_path(root)).unwrap_or_default();
    if text.trim().is_empty() {
        return;
    }
    let legacy: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let repos = legacy
        .get("repos")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let worktrees = legacy
        .get("worktrees")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if repos.is_empty() && worktrees.is_empty() {
        return;
    }
    let imported = crate::db::with(root, |c| {
        let existing = crate::vcs::store::list_repos(c).len();
        if existing > 0 {
            return 0usize;
        }
        let mut n = 0usize;
        for repo in &repos {
            let id = repo.get("id").and_then(Value::as_str).unwrap_or_default();
            let name = repo.get("name").and_then(Value::as_str).unwrap_or_default();
            let path = repo.get("path").and_then(Value::as_str).unwrap_or_default();
            if id.is_empty() || name.is_empty() || path.is_empty() {
                continue;
            }
            if crate::vcs::store::insert_repo(c, id, name, path, "", "main").is_ok() {
                n += 1;
            }
        }
        for wt in &worktrees {
            let mut row = wt.clone();
            if let Some(obj) = row.as_object_mut() {
                let repo_id = obj.get("repoId").and_then(Value::as_str).unwrap_or_default().to_string();
                if let Some(repo) = crate::vcs::store::repo_by_id(c, &repo_id) {
                    obj.insert("repoId".into(), json!(repo.get("id").and_then(Value::as_str).unwrap_or_default()));
                }
            }
            if crate::vcs::store::upsert_worktree(c, &row).is_ok() {
                n += 1;
            }
        }
        n
    })
    .unwrap_or(0);
    if imported > 0 {
        eprintln!("hostgit: imported {imported} legacy registry row(s) from hosts.json");
    }
}

fn repo_rows(root: &Path) -> Vec<Value> {
    crate::db::with(root, |c| crate::vcs::store::list_repos(c)).unwrap_or_default()
}

fn repo_row(root: &Path, id: &str) -> Option<Value> {
    crate::db::with(root, |c| crate::vcs::store::repo_by_id(c, id))
        .ok()
        .flatten()
}

fn worktree_rows(root: &Path) -> Vec<Value> {
    crate::db::with(root, |c| {
        let repos = crate::vcs::store::list_repos(c);
        crate::vcs::store::list_worktrees(c)
            .into_iter()
            .map(|mut w| {
                if let Some(obj) = w.as_object_mut() {
                    let repo_id = obj.get("repoId").and_then(Value::as_str).unwrap_or_default().to_string();
                    if let Some(repo) = repos.iter().find(|r| r.get("id").and_then(Value::as_str) == Some(repo_id.as_str())) {
                        obj.insert("repoName".into(), json!(repo.get("name").and_then(Value::as_str).unwrap_or_default()));
                        obj.insert("repoPath".into(), json!(repo.get("path").and_then(Value::as_str).unwrap_or_default()));
                    }
                }
                w
            })
            .collect()
    })
    .unwrap_or_default()
}

fn worktree_row(root: &Path, id: &str) -> Option<Value> {
    worktree_rows(root)
        .into_iter()
        .find(|w| w.get("id").and_then(Value::as_str) == Some(id))
}

fn put_worktree(root: &Path, w: &Value) -> Result<(), String> {
    crate::db::with(root, |c| crate::vcs::store::upsert_worktree(c, w))?
}

fn drop_worktree(root: &Path, id: &str) -> Result<(), String> {
    crate::db::with(root, |c| crate::vcs::store::delete_worktree(c, id))?
}

fn id_for(prefix: &str, path: &str) -> String {
    crate::vcs::id_for(prefix, path)
}

pub fn check_path(raw: &str) -> Result<PathBuf, String> {
    crate::vcs::check_path(raw).map_err(|e| e.message)
}

fn is_git_dir(p: &Path) -> bool {
    let g = p.join(".git");
    g.is_dir() || g.is_file()
}

pub fn is_safe_ref(s: &str) -> bool {
    git::is_safe_ref(s)
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    git::git(dir, args)
}

fn current_branch(dir: &Path) -> String {
    git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|s| s.trim().to_string())
        .map(|s| if s == "HEAD" { String::new() } else { s })
        .unwrap_or_default()
}

pub fn branches(dir: &Path) -> Vec<String> {
    git::branches(dir).into_iter().map(|(name, _)| name).collect()
}

pub fn add_repo(root: &Path, raw_path: &str) -> Result<Value, String> {
    let path = check_path(raw_path)?;
    if !is_git_dir(&path) {
        return Err(format!("not a git repository: {}", path.display()));
    }
    let path_s = path.to_string_lossy().to_string();
    let id = id_for("r", &path_s);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path_s.clone());

    crate::db::with(root, |c| {
        if crate::vcs::store::repo_by_id(c, &id).is_some() {
            let head = current_branch(&path);
            let select = if head.is_empty() {
                String::new()
            } else {
                format!("{id}:{head}")
            };
            return Ok(json!({ "id": id, "select": select, "selectKind": "branch" }));
        }
        crate::vcs::store::insert_repo(c, &id, &name, &path_s, "", "main")?;
        let head = current_branch(&path);
        let select = if head.is_empty() {
            String::new()
        } else {
            format!("{id}:{head}")
        };
        Ok(json!({ "id": id, "select": select, "selectKind": "branch" }))
    })?
}

pub fn remove_repo(root: &Path, id: &str) -> Result<Value, String> {
    let ids: Vec<String> = worktree_rows(root)
        .into_iter()
        .filter(|w| w.get("repoId").and_then(Value::as_str) == Some(id))
        .filter_map(|w| w.get("id").and_then(Value::as_str).map(str::to_string))
        .collect();
    for wid in ids {
        let _ = remove_worktree(root, &wid);
    }
    crate::db::with(root, |c| crate::vcs::store::delete_repo(c, id))??;
    Ok(json!({ "removed": id }))
}

pub fn add_branch(root: &Path, repo_id: &str, name: &str, from: &str) -> Result<Value, String> {
    let repo = repo_row(root, repo_id).ok_or("no such repo")?;
    let dir = PathBuf::from(repo.get("path").and_then(Value::as_str).unwrap_or_default());
    if !is_safe_ref(name) {
        return Err(format!("not a usable branch name: {name}"));
    }
    let base = from.trim();
    if !base.is_empty() && !is_safe_ref(base) {
        return Err(format!("not a usable start point: {base}"));
    }
    let mut args = vec!["branch", "--", name];
    if !base.is_empty() {
        args.push(base);
    }
    git(&dir, &args)?;
    Ok(json!({
        "branch": name,
        "select": format!("{repo_id}:{name}"),
        "selectKind": "branch",
    }))
}

fn worktree_dir(root: &Path, repo_name: &str, branch: &str) -> PathBuf {
    let safe: String = branch
        .chars()
        .map(|c| if c == '/' { '-' } else { c })
        .collect();
    root.join(".angela")
        .join("worktrees")
        .join(format!("{repo_name}-{safe}"))
}

pub fn add_worktree(
    root: &Path,
    repo_id: &str,
    branch: &str,
    create: bool,
) -> Result<Value, String> {
    let repo = repo_row(root, repo_id).ok_or("no such repo")?;
    let repo_path = repo.get("path").and_then(Value::as_str).unwrap_or_default().to_string();
    let repo_name = repo.get("name").and_then(Value::as_str).unwrap_or("repo").to_string();
    let dir = PathBuf::from(&repo_path);
    if !is_safe_ref(branch) {
        return Err(format!("not a usable branch name: {branch}"));
    }
    let target = worktree_dir(root, &repo_name, branch);
    if target.exists() {
        return Err(format!(
            "a worktree directory already exists at {}",
            target.display()
        ));
    }
    std::fs::create_dir_all(target.parent().unwrap_or(&target)).map_err(|e| e.to_string())?;
    let target_s = target.to_string_lossy().to_string();
    if target_s.contains(':') || target_s.contains(',') {
        return Err("the worktree path would contain ':' or ','".into());
    }

    let args: Vec<&str> = if create {
        vec!["worktree", "add", "-b", branch, &target_s]
    } else {
        vec!["worktree", "add", &target_s, branch]
    };
    git(&dir, &args)?;

    let id = id_for("w", &target_s);
    put_worktree(
        root,
        &json!({
            "id": id,
            "repoId": repo_id,
            "repoName": repo_name,
            "repoPath": repo_path,
            "path": target_s,
            "branch": branch,
            "sessionId": "",
            "sandboxId": "",
        }),
    )?;
    Ok(json!({
        "id": id,
        "path": target.to_string_lossy(),
        "select": id,
        "selectKind": "worktree",
    }))
}

pub fn remove_worktree(root: &Path, id: &str) -> Result<Value, String> {
    let Some(wt) = worktree_row(root, id) else {
        return Err("no such worktree".into());
    };
    let repo_id = wt.get("repoId").and_then(Value::as_str).unwrap_or_default();
    let path = wt.get("path").and_then(Value::as_str).unwrap_or_default().to_string();
    let mut note = String::new();
    if let Some(repo) = repo_row(root, repo_id) {
        let dir = PathBuf::from(repo.get("path").and_then(Value::as_str).unwrap_or_default());
        if let Err(e) = git(&dir, &["worktree", "remove", "--force", &path]) {
            note = e;
        }
    }
    drop_worktree(root, id)?;
    if !path.is_empty() {
        let _ = Command::new("git")
            .args(["config", "--global", "--unset-all", "safe.directory", &path])
            .output();
    }
    Ok(json!({ "removed": id, "note": note }))
}

fn default_worktree_parent(root: &Path) -> PathBuf {
    root.join(".angela").join("worktrees")
}

fn branch_prefix(branch: &str) -> String {
    let mut n = branch.len().min(23);
    while n > 0 && !branch.is_char_boundary(n) {
        n -= 1;
    }
    branch[..n]
        .chars()
        .map(|c| if c == '/' { '-' } else { c })
        .collect()
}

fn branch_hash(common_dir: &str, branch: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(common_dir.as_bytes());
    h.update([0]);
    h.update(branch.as_bytes());
    let mut hex = format!("{:x}", h.finalize());
    hex.truncate(16);
    hex
}

fn random_hex() -> String {
    use std::io::Read;
    let mut buf = [0u8; 4];
    let mut ok = false;
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        ok = f.read_exact(&mut buf).is_ok();
    }
    if !ok {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut h = sha2::Sha256::new();
        use sha2::Digest;
        h.update(nanos.to_le_bytes());
        h.update(std::process::id().to_le_bytes());
        let d = h.finalize();
        buf = [d[0], d[1], d[2], d[3]];
    }
    let mut s = String::with_capacity(8);
    for b in buf {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub fn expand_worktree_path(
    parent: &Path,
    repo_name: &str,
    branch: &str,
    common_dir: &str,
    pattern: &str,
) -> Result<String, String> {
    let hex = random_hex();
    let mut out = String::new();
    let mut rest = pattern;
    loop {
        let Some(start) = rest.find("{{") else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        let Some(end) = rest.find("}}") else {
            return Err(format!("unterminated placeholder in: {pattern}"));
        };
        let name = rest[..end].trim();
        rest = &rest[end + 2..];
        let value = match name {
            "parent" => parent.to_string_lossy().to_string(),
            "repo" => repo_name.to_string(),
            "prefix" => branch_prefix(branch),
            "hash" => branch_hash(common_dir, branch),
            "branch" => branch.to_string(),
            "hex" => hex.clone(),
            other => return Err(format!("unknown placeholder {{{{{other}}}}}")),
        };
        out.push_str(&value);
    }
    if out.is_empty() {
        return Err("the worktree path is empty".into());
    }
    if out.contains(':') || out.contains(',') {
        return Err(
            "the worktree path cannot contain ':' or ',' — a mount spec is delimited by both".into(),
        );
    }
    Ok(out)
}

fn expand_branch_pattern(pattern: &str) -> Result<String, String> {
    let hex = random_hex();
    let mut out = String::new();
    let mut rest = pattern;
    loop {
        let Some(start) = rest.find("{{") else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        let Some(end) = rest.find("}}") else {
            return Err(format!("unterminated placeholder in branch pattern: {pattern}"));
        };
        let name = rest[..end].trim();
        rest = &rest[end + 2..];
        match name {
            "hex" => out.push_str(&hex),
            other => return Err(format!("unknown placeholder {{{{{other}}}}} in branch pattern")),
        }
    }
    if out.is_empty() {
        return Err("the branch pattern is empty".into());
    }
    Ok(out)
}

fn git_common_dir(repo: &Path) -> Result<String, String> {
    git::git_common_dir(repo).ok_or_else(|| "git reported no common dir".to_string())
}

fn record_from_row(row: &Value) -> Value {
    json!({
        "repoId": row.get("repoId").cloned().unwrap_or(Value::Null),
        "repoName": row.get("repoName").cloned().unwrap_or(Value::Null),
        "branch": row.get("branch").cloned().unwrap_or(Value::Null),
        "path": row.get("path").cloned().unwrap_or(Value::Null),
        "branchPattern": row.get("branchPattern").cloned().unwrap_or(Value::Null),
        "pathPattern": row.get("pathPattern").cloned().unwrap_or(Value::Null),
        "provisioned": true,
    })
}

pub fn provision_session_worktree(
    root: &Path,
    session_id: &str,
    repo_id: &str,
    branch_pattern: &str,
    path_pattern: &str,
) -> Result<Value, String> {
    provision_session_worktree_full(root, session_id, repo_id, branch_pattern, path_pattern, None)
}

pub fn provision_session_worktree_on_branch(
    root: &Path,
    session_id: &str,
    repo_id: &str,
    branch: &str,
    path_pattern: &str,
) -> Result<Value, String> {
    provision_session_worktree_full(root, session_id, repo_id, branch, path_pattern, Some(branch))
}

fn provision_session_worktree_full(
    root: &Path,
    session_id: &str,
    repo_id: &str,
    branch_pattern: &str,
    path_pattern: &str,
    branch_override: Option<&str>,
) -> Result<Value, String> {
    let repo = repo_row(root, repo_id).ok_or("no such repo")?;
    let repo_path = PathBuf::from(repo.get("path").and_then(Value::as_str).unwrap_or_default());
    if !repo_path.is_dir() {
        return Err(format!(
            "repo is missing or is not a directory: {}",
            repo_path.display()
        ));
    }
    let repo_name = repo
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("repo")
        .to_string();

    let branch = match branch_override {
        Some(b) => b.to_string(),
        None => expand_branch_pattern(branch_pattern)?,
    };
    if !is_safe_ref(&branch) {
        return Err(format!("not a usable branch name: {branch}"));
    }

    let common_dir = git_common_dir(&repo_path)?;

    let parent = default_worktree_parent(root);
    let pattern = if path_pattern.trim().is_empty() {
        "{{parent}}/{{repo}}/{{prefix}}-{{hash}}"
    } else {
        path_pattern
    };
    let path_s = expand_worktree_path(&parent, &repo_name, &branch, &common_dir, pattern)?;
    if let Some(dir) = Path::new(&path_s).parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create the worktree parent dir: {e}"))?;
    }

    let target = PathBuf::from(&path_s);
    let row = worktree_rows(root)
        .into_iter()
        .find(|w| {
            w.get("path").and_then(Value::as_str) == Some(path_s.as_str())
                && w.get("sessionId").and_then(Value::as_str) == Some(session_id)
        })
        ;
    if let Some(row) = row {
        if target.is_dir() {
            return Ok(record_from_row(&row));
        }
    }
    if target.exists() {
        return Err(format!(
            "a directory already exists at {}",
            target.display()
        ));
    }

    let branch_exists = git(
        &repo_path,
        &[
            "rev-parse",
            "--verify",
            &format!("refs/heads/{branch}"),
        ],
    )
    .is_ok();
    let args: Vec<&str> = if branch_exists {
        vec!["worktree", "add", &path_s, &branch]
    } else {
        vec!["worktree", "add", "-b", &branch, &path_s]
    };
    if let Err(e) = git(&repo_path, &args) {
        if !e.contains("missing but already registered worktree") {
            return Err(e);
        }
        let _ = git(&repo_path, &["worktree", "prune"]);
        git(&repo_path, &args)?;
    }

    if let Some(mapped) = subuid_mapped_uid() {
        let acl = format!("u:{mapped}:rwX,u:{}:rwX", current_euid().unwrap_or(0));
        let _ = Command::new("setfacl").arg("-R").arg("-m").arg(&acl).arg(&path_s).output();
        let _ = Command::new("setfacl")
            .arg("-R")
            .arg("-d")
            .arg("-m")
            .arg(&acl)
            .arg(&path_s)
            .output();
        let _ = Command::new("sudo")
            .arg("-n")
            .arg("chown")
            .arg("-R")
            .arg(mapped.to_string())
            .arg(&path_s)
            .output();
    }
    let _ = Command::new("chmod")
        .arg("-R")
        .arg("a+rwX")
        .arg(&path_s)
        .output();
    let _ = Command::new("git")
        .args(["config", "--global", "--add", "safe.directory", &path_s])
        .output();

    let id = id_for("w", &path_s);
    put_worktree(
        root,
        &json!({
            "id": id,
            "repoId": repo_id,
            "repoName": repo_name,
            "repoPath": repo_path,
            "branch": branch,
            "path": path_s,
            "sessionId": session_id,
            "branchPattern": branch_pattern,
            "pathPattern": path_pattern,
            "sandboxId": "",
        }),
    )?;

    Ok(json!({
        "repoId": repo_id,
        "repoName": repo_name,
        "repoPath": repo_path,
        "branch": branch,
        "path": path_s,
        "branchPattern": branch_pattern,
        "pathPattern": path_pattern,
        "provisioned": true,
    }))
}

fn subuid_mapped_uid() -> Option<u32> {
    let euid = current_euid()?;
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
    let name = passwd.lines().find_map(|line| {
        let mut f = line.split(':');
        let user = f.next()?;
        let _pw = f.next()?;
        let uid: u32 = f.next()?.parse().ok()?;
        (uid == euid).then(|| user.to_string())
    })?;
    let subuid = std::fs::read_to_string("/etc/subuid").ok()?;
    let base: u64 = subuid.lines().find_map(|line| {
        let (user, rest) = line.split_once(':')?;
        let base: u64 = rest.split(':').next()?.parse().ok()?;
        (user == name).then_some(base)
    })?;
    u32::try_from(base + 999).ok()
}

fn current_euid() -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self").ok().map(|m| m.uid())
}

pub fn reap_session_worktree(root: &Path, session_id: &str) {
    if session_id.trim().is_empty() {
        return;
    }
    let ids: Vec<String> = worktree_rows(root)
        .into_iter()
        .filter(|w| w.get("sessionId").and_then(Value::as_str) == Some(session_id))
        .filter_map(|w| w.get("id").and_then(Value::as_str).map(str::to_string))
        .collect();
    for id in ids {
        let _ = remove_worktree(root, &id);
    }
}

pub fn reap_all_session_worktrees(root: &Path) {
    let ids: Vec<String> = worktree_rows(root)
        .into_iter()
        .filter(|w| {
            !w.get("sessionId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .is_empty()
        })
        .filter_map(|w| w.get("id").and_then(Value::as_str).map(str::to_string))
        .collect();
    for id in ids {
        let _ = remove_worktree(root, &id);
    }
}

pub fn session_properties_ready(
    root: &Path,
    meta: &Value,
    def: &angela::agent_loader::AgentDef,
) -> Result<(), String> {
    let id = meta.get("id").and_then(Value::as_str).unwrap_or("");
    let store = angela::conv_store::ConvStore::new(root).map_err(|e| e.to_string())?;
    let authored = store.load(id).unwrap_or(Value::Null);
    let mut resolved = meta.clone();
    let mut workspace_only = meta.clone();
    workspace_only["worktree"] = Value::Null;
    session_worktree_ready(root, &workspace_only, def)?;
    if authored.get("worktree").is_none_or(Value::is_null) {
        if let Some(wt) = meta.get("worktree").filter(|v| v.is_object()) {
            let repo = wt.get("repoId").and_then(Value::as_str).unwrap_or("");
            let branch = wt.get("branchPattern").and_then(Value::as_str).unwrap_or("");
            let path = wt.get("pathPattern").and_then(Value::as_str).unwrap_or("");
            if repo.is_empty() || branch.is_empty() {
                return Err("worktree requires a repo and branch pattern".into());
            }
            static PROVISION: std::sync::Mutex<()> = std::sync::Mutex::new(());
            let _guard = PROVISION.lock().map_err(|e| e.to_string())?;
            let cached = store.inherited_worktree(id).filter(|v| {
                v.get("repoId").and_then(Value::as_str) == Some(repo)
                    && v.get("branchPattern").and_then(Value::as_str) == Some(branch)
                    && v.get("pathPattern").and_then(Value::as_str).unwrap_or("") == path
            });
            let record = match cached {
                Some(record) => record,
                None => {
                    let record = provision_session_worktree(root, id, repo, branch, path)?;
                    store.set_inherited_worktree(id, &record)?;
                    record
                }
            };
            resolved["worktree"] = record;
        }
    }
    session_worktree_ready(root, &resolved, def)
}

pub fn session_worktree_ready(
    root: &Path,
    meta: &Value,
    def: &angela::agent_loader::AgentDef,
) -> Result<(), String> {
    let _ = def;
    let ws = meta
        .get("workspaceRoot")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if !ws.is_empty() {
        if !ws.starts_with('/') {
            return Err(format!("workspace root must be absolute: {ws}"));
        }
        if !Path::new(&ws).is_dir() {
            return Err(format!("workspace root is not a directory: {ws}"));
        }
    }

    let Some(record) = meta.get("worktree").filter(|w| !w.is_null()) else {
        return Ok(());
    };
    let Some(record) = record.as_object() else {
        return Err("worktree not provisioned".into());
    };
    let provisioned = record
        .get("provisioned")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !provisioned {
        return Err("worktree not provisioned".into());
    }
    let path = record
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if path.is_empty() {
        return Err("worktree has no path".into());
    }

    if !Path::new(&path).is_dir() {
        let repo_id = record
            .get("repoId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let branch = record
            .get("branch")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let path_pattern = record
            .get("pathPattern")
            .and_then(Value::as_str)
            .unwrap_or("");
        let session_id = meta.get("id").and_then(Value::as_str).unwrap_or_default();
        if repo_id.is_empty() || branch.is_empty() {
            return Err("worktree missing".into());
        }
        if let Err(e) =
            provision_session_worktree_on_branch(root, session_id, repo_id, branch, path_pattern)
        {
            return Err(format!("worktree missing: {e}"));
        }
        if !Path::new(&path).is_dir() {
            return Err("worktree missing".into());
        }
    }
    Ok(())
}

pub fn sweep(root: &Path) {
    let stale: Vec<String> = worktree_rows(root)
        .into_iter()
        .filter(|w| {
            !w.get("path")
                .and_then(Value::as_str)
                .map(|p| Path::new(p).is_dir())
                .unwrap_or(false)
        })
        .filter_map(|w| w.get("id").and_then(Value::as_str).map(str::to_string))
        .collect();
    for id in stale {
        let _ = drop_worktree(root, &id);
    }
}

pub fn branch_items(root: &Path) -> Value {
    let mut out = Vec::new();
    for repo in repo_rows(root) {
        let id = repo.get("id").and_then(Value::as_str).unwrap_or_default();
        let name = repo.get("name").and_then(Value::as_str).unwrap_or_default();
        let path = PathBuf::from(repo.get("path").and_then(Value::as_str).unwrap_or_default());
        if !path.is_dir() {
            continue;
        }
        for b in branches(&path) {
            out.push(json!({
                "id": format!("{id}:{b}"),
                "name": b,
                "hint": name,
                "icon": "git-branch",
            }));
        }
    }
    Value::Array(out)
}

pub fn repo_items(root: &Path) -> Value {
    Value::Array(
        repo_rows(root)
            .iter()
            .map(|r| {
                let path = r.get("path").and_then(Value::as_str).unwrap_or_default();
                json!({
                    "id": r.get("id").cloned().unwrap_or(Value::Null),
                    "name": r.get("name").cloned().unwrap_or(Value::Null),
                    "hint": current_branch(Path::new(path)),
                    "path": path,
                    "icon": "folder-open",
                })
            })
            .collect::<Vec<_>>(),
    )
}

pub fn worktree_items(root: &Path) -> Value {
    Value::Array(
        worktree_rows(root)
            .iter()
            .map(|w| {
                json!({
                    "id": w.get("id").cloned().unwrap_or(Value::Null),
                    "name": w.get("branch").cloned().unwrap_or(Value::Null),
                    "hint": w.get("repoName").cloned().unwrap_or(Value::Null),
                    "path": w.get("path").cloned().unwrap_or(Value::Null),
                    "icon": "git-fork",
                })
            })
            .collect::<Vec<_>>(),
    )
}

pub fn everything(root: &Path) -> Value {
    let mut m = Map::new();
    m.insert("repos".into(), repo_items(root));
    m.insert("repoBranches".into(), branch_items(root));
    m.insert("worktrees".into(), worktree_items(root));
    Value::Object(m)
}

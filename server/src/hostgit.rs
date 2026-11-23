//! Repos and worktrees on **this machine** — the two checkout-shaped things
//! the composer's last two pickers choose between.
//!
//! ## Why these live here and not in angela
//!
//! A conversation, its project and its transcript travel with the project
//! directory. "There is a clone of psicom/chat at `/workspace/psicom/chat`"
//! does not — it is true of this laptop and nowhere else. So the registry is
//! a file beside the database rather than a table in it, and nothing here
//! reaches angela.
//!
//! ## Why there is no scan
//!
//! A repo becomes selectable only when someone registers its absolute path.
//! Walking a configured root for `.git` directories would be friendlier and
//! is what most tools do; it also means a list that changes under you, and
//! every clone on the machine — including ones you would never mount into a
//! sandbox — showing up in a menu. Registration is one deliberate act per
//! repo and the list is then exactly what you put in it.
//!
//! ## Shelling out to git
//!
//! Every call is [`std::process::Command`] with separate arguments and no
//! shell, so a path or a branch name is an argument and can never be parsed
//! as a command. On top of that, [`is_safe_ref`] refuses anything that could
//! be read as an *option* — a branch called `--upload-pack=…` is a real
//! attack against tools that interpolate, and `-` is a legal first character
//! for a string that is not a legal branch.
//!
//! ## Worktrees and who cleans them up
//!
//! sbox-b creates sandboxes and knows nothing about worktrees, so its
//! `DELETE` cannot remove one. That makes reaping this module's job:
//! [`forget_for_sandbox`] runs when a sandbox is destroyed, and
//! [`sweep`] runs at boot to catch the ones a crash left behind. Without
//! both, every sandbox created from a worktree would leak a checkout.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Map, Value};

/// How long any git call may take before we give up on it. A clone is not
/// run here — these are all local metadata operations — so this is about a
/// wedged filesystem rather than a slow network.
const GIT_TIMEOUT_SECS: u64 = 30;

// ── the registry file ───────────────────────────────────────────────────

fn registry_path(root: &Path) -> PathBuf {
    root.join(".angela").join("hosts.json")
}

/// Read the registry, or an empty one. A corrupt file reads as empty rather
/// than fatal: it is a cache of paths the user can re-register, and refusing
/// to start the UI over it would be the worse failure.
pub fn load(root: &Path) -> Value {
    let text = std::fs::read_to_string(registry_path(root)).unwrap_or_default();
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if v.is_object() {
        v
    } else {
        json!({ "repos": [], "worktrees": [] })
    }
}

fn save(root: &Path, v: &Value) -> Result<(), String> {
    let p = registry_path(root);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
    // Write-then-rename: a half-written registry would lose every
    // registration, and this file is only ever read whole.
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
}

fn list_of<'a>(reg: &'a Value, key: &str) -> &'a [Value] {
    reg.get(key).and_then(Value::as_array).map(|v| &v[..]).unwrap_or(&[])
}

/// A short, stable id for a path. Content-derived so re-registering the same
/// clone is idempotent rather than producing a second entry pointing at the
/// same directory.
fn id_for(prefix: &str, path: &str) -> String {
    // FNV-1a: no dependency, and this is a naming scheme, not a hash anyone
    // relies on for integrity.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{prefix}{h:012x}")
}

// ── validation ──────────────────────────────────────────────────────────

/// A path we are willing to register and later hand to a container runtime.
///
/// `:` and `,` are refused because sbox-b's `--mount` takes a
/// comma-separated, colon-delimited string: a path containing either would
/// be split in the middle and mount something nobody asked for.
pub fn check_path(raw: &str) -> Result<PathBuf, String> {
    let t = raw.trim();
    if t.is_empty() {
        return Err("a repo needs an absolute path".into());
    }
    if !t.starts_with('/') {
        return Err(format!("path must be absolute: {t}"));
    }
    if t.contains(':') || t.contains(',') {
        return Err("path cannot contain ':' or ',' — a mount spec is delimited by both".into());
    }
    let p = PathBuf::from(t);
    let canon = p
        .canonicalize()
        .map_err(|_| format!("no such directory: {t}"))?;
    if !canon.is_dir() {
        return Err(format!("not a directory: {t}"));
    }
    // Re-check after canonicalising: a symlink can land somewhere with a
    // character the mount spec cannot carry.
    let s = canon.to_string_lossy().to_string();
    if s.contains(':') || s.contains(',') {
        return Err("resolved path contains ':' or ','".into());
    }
    Ok(canon)
}

/// Is this a git checkout? `.git` is a directory in a clone and a **file**
/// in a worktree, so both are accepted.
fn is_git_dir(p: &Path) -> bool {
    let g = p.join(".git");
    g.is_dir() || g.is_file()
}

/// A branch or ref name we will pass to git.
///
/// Deliberately stricter than git itself. The leading-dash rule is the one
/// that matters: git would read `-u` as an option, and a "branch name" is
/// the most natural place for an attacker-controlled string to arrive.
pub fn is_safe_ref(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty()
        && t.len() <= 200
        && !t.starts_with('-')
        && !t.starts_with('/')
        && !t.ends_with('/')
        && !t.contains("..")
        && !t.contains("//")
        && !t.contains('\\')
        && !t.contains('~')
        && !t.contains('^')
        && !t.contains(':')
        && !t.contains('?')
        && !t.contains('*')
        && !t.contains('[')
        && !t.chars().any(|c| c.is_whitespace() || c.is_control())
}

// ── running git ─────────────────────────────────────────────────────────

/// Run git in `dir`. Never through a shell: every argument is passed as one.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for a in args {
        cmd.arg(a);
    }
    // A prompt would block this thread until the timeout; failing is better
    // than hanging the request on a credential helper nobody can answer.
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.env("GIT_OPTIONAL_LOCKS", "0");
    let out = run_with_timeout(cmd)?;
    if out.status_ok {
        Ok(out.stdout)
    } else {
        let msg = out.stderr.lines().next().unwrap_or("git failed").to_string();
        Err(msg)
    }
}

struct Output {
    status_ok: bool,
    stdout: String,
    stderr: String,
}

/// `Command::output()` with a deadline.
///
/// `std` has no timeout for this, so the child is spawned, polled, and
/// killed if it outstays the limit. Without it a git call against a hung
/// network filesystem holds a connection thread for as long as the process
/// lives.
fn run_with_timeout(mut cmd: Command) -> Result<Output, String> {
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("could not run git: {e}"))?;
    let deadline = Instant::now() + Duration::from_secs(GIT_TIMEOUT_SECS);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("git timed out after {GIT_TIMEOUT_SECS}s"));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("could not read git output: {e}"))?;
    Ok(Output {
        status_ok: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    })
}

/// A checkout's local branches, in git's own order.
pub fn branches(dir: &Path) -> Vec<String> {
    git(
        dir,
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    )
    .map(|s| {
        s.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

/// What `HEAD` is on, or empty in a detached checkout.
fn current_branch(dir: &Path) -> String {
    git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|s| s.trim().to_string())
        .map(|s| if s == "HEAD" { String::new() } else { s })
        .unwrap_or_default()
}

// ── repos ───────────────────────────────────────────────────────────────

/// Register a clone by absolute path.
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

    let mut reg = load(root);
    let mut repos: Vec<Value> = list_of(&reg, "repos").to_vec();
    // Re-registering the same clone updates it rather than duplicating: the
    // id is derived from the path, so a second entry would be invisible in
    // the UI and impossible to remove independently.
    repos.retain(|r| r.get("id").and_then(Value::as_str) != Some(id.as_str()));
    repos.push(json!({ "id": id, "path": path_s, "name": name }));
    reg["repos"] = Value::Array(repos);
    reg.as_object_mut()
        .map(|m| m.entry("worktrees").or_insert_with(|| json!([])));
    save(root, &reg)?;
    // What the branch picker should land on. Registering a clone is only
    // ever a step towards choosing one of its branches, so the caller gets
    // the row to select rather than having to work it out from the id.
    let head = current_branch(&path);
    let select = if head.is_empty() {
        String::new()
    } else {
        format!("{id}:{head}")
    };
    Ok(json!({ "id": id, "select": select, "selectKind": "branch" }))
}

/// Forget a clone. Nothing on disk is touched — the repo was here before
/// chat knew about it and is not chat's to delete.
///
/// Its worktrees are forgotten too, since they are listed against it; the
/// directories themselves are removed properly via [`remove_worktree`]
/// before this point, or left for [`sweep`].
pub fn remove_repo(root: &Path, id: &str) -> Result<Value, String> {
    let mut reg = load(root);
    let repos: Vec<Value> = list_of(&reg, "repos")
        .iter()
        .filter(|r| r.get("id").and_then(Value::as_str) != Some(id))
        .cloned()
        .collect();
    let worktrees: Vec<Value> = list_of(&reg, "worktrees")
        .iter()
        .filter(|w| w.get("repoId").and_then(Value::as_str) != Some(id))
        .cloned()
        .collect();
    reg["repos"] = Value::Array(repos);
    reg["worktrees"] = Value::Array(worktrees);
    save(root, &reg)?;
    Ok(json!({ "removed": id }))
}

/// Make a branch in a registered clone. `from` defaults to the current HEAD.
pub fn add_branch(root: &Path, repo_id: &str, name: &str, from: &str) -> Result<Value, String> {
    let repo = find(root, "repos", repo_id).ok_or("no such repo")?;
    let dir = PathBuf::from(repo.get("path").and_then(Value::as_str).unwrap_or_default());
    if !is_safe_ref(name) {
        return Err(format!("not a usable branch name: {name}"));
    }
    let base = from.trim();
    if !base.is_empty() && !is_safe_ref(base) {
        return Err(format!("not a usable start point: {base}"));
    }
    // `--` so a name that survived validation still cannot be read as a
    // revision or a path. Belt and braces, and free.
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

fn find(root: &Path, key: &str, id: &str) -> Option<Value> {
    list_of(&load(root), key)
        .iter()
        .find(|r| r.get("id").and_then(Value::as_str) == Some(id))
        .cloned()
}

/// Where a worktree for this repo and branch should live.
///
/// Under `.angela/worktrees` beside the conversations, so a checkout chat
/// made is recognisable as chat's and a sweep knows where to look. The
/// branch's `/` becomes `-`: `feature/login` is one directory, not two.
fn worktree_dir(root: &Path, repo_name: &str, branch: &str) -> PathBuf {
    let safe: String = branch
        .chars()
        .map(|c| if c == '/' { '-' } else { c })
        .collect();
    root.join(".angela")
        .join("worktrees")
        .join(format!("{repo_name}-{safe}"))
}

// ── worktrees ───────────────────────────────────────────────────────────

/// `git worktree add` against a registered clone.
///
/// An existing branch is checked out; `create` makes a new one off the
/// clone's HEAD. Both are one git invocation — the `-b` form is how git
/// says "and make the branch while you are at it", which avoids a window
/// where the branch exists but the worktree does not.
pub fn add_worktree(
    root: &Path,
    repo_id: &str,
    branch: &str,
    create: bool,
) -> Result<Value, String> {
    let repo = find(root, "repos", repo_id).ok_or("no such repo")?;
    let repo_path = repo.get("path").and_then(Value::as_str).unwrap_or_default();
    let repo_name = repo.get("name").and_then(Value::as_str).unwrap_or("repo");
    let dir = PathBuf::from(repo_path);
    if !is_safe_ref(branch) {
        return Err(format!("not a usable branch name: {branch}"));
    }
    let target = worktree_dir(root, repo_name, branch);
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
    let mut reg = load(root);
    let mut list: Vec<Value> = list_of(&reg, "worktrees").to_vec();
    list.retain(|w| w.get("id").and_then(Value::as_str) != Some(id.as_str()));
    list.push(json!({
        "id": id,
        "repoId": repo_id,
        "repoName": repo_name,
        "path": target_s,
        "branch": branch,
        // Set when a sandbox is created from this worktree, so destroying
        // that sandbox can take the checkout with it.
        "sandboxId": "",
    }));
    reg["worktrees"] = Value::Array(list);
    save(root, &reg)?;
    Ok(json!({
        "id": id,
        "path": target.to_string_lossy(),
        "select": id,
        "selectKind": "worktree",
    }))
}

/// Remove a worktree: git first, then the registry.
///
/// `--force` because the point of removing it is that it is going away, and
/// a worktree whose sandbox just wrote to it will nearly always be dirty.
/// Losing uncommitted work in a throwaway checkout is the expected outcome;
/// the branch, which is where anything worth keeping was committed, stays.
pub fn remove_worktree(root: &Path, id: &str) -> Result<Value, String> {
    let Some(wt) = find(root, "worktrees", id) else {
        return Err("no such worktree".into());
    };
    let repo_id = wt.get("repoId").and_then(Value::as_str).unwrap_or_default();
    let path = wt.get("path").and_then(Value::as_str).unwrap_or_default();
    let mut note = String::new();
    if let Some(repo) = find(root, "repos", repo_id) {
        let dir = PathBuf::from(repo.get("path").and_then(Value::as_str).unwrap_or_default());
        if let Err(e) = git(&dir, &["worktree", "remove", "--force", path]) {
            // The registry entry still goes, or a worktree git has already
            // lost track of becomes permanently unremovable from the UI.
            note = e;
        }
    }
    let mut reg = load(root);
    let list: Vec<Value> = list_of(&reg, "worktrees")
        .iter()
        .filter(|w| w.get("id").and_then(Value::as_str) != Some(id))
        .cloned()
        .collect();
    reg["worktrees"] = Value::Array(list);
    save(root, &reg)?;
    Ok(json!({ "removed": id, "note": note }))
}

/// Note which sandbox a worktree was mounted into, so destroying that
/// sandbox can reap it.
pub fn claim_worktree(root: &Path, id: &str, sandbox: &str) {
    let mut reg = load(root);
    let list: Vec<Value> = list_of(&reg, "worktrees")
        .iter()
        .map(|w| {
            let mut w = w.clone();
            if w.get("id").and_then(Value::as_str) == Some(id) {
                w["sandboxId"] = json!(sandbox);
            }
            w
        })
        .collect();
    reg["worktrees"] = Value::Array(list);
    let _ = save(root, &reg);
}

/// Remove whatever worktree was created for this sandbox.
///
/// The half sbox-b cannot do: it owns the container and knows nothing about
/// the checkout bound into it, so a `DELETE` there would otherwise leave the
/// worktree behind forever.
pub fn forget_for_sandbox(root: &Path, sandbox: &str) {
    if sandbox.is_empty() {
        return;
    }
    let ids: Vec<String> = list_of(&load(root), "worktrees")
        .iter()
        .filter(|w| w.get("sandboxId").and_then(Value::as_str) == Some(sandbox))
        .filter_map(|w| w.get("id").and_then(Value::as_str).map(str::to_string))
        .collect();
    for id in ids {
        let _ = remove_worktree(root, &id);
    }
}

/// Drop registry entries whose directory is gone.
///
/// Run at boot. A crash between `git worktree add` and the registry write,
/// or a directory removed by hand, otherwise leaves a row the UI offers and
/// every mount of which fails.
pub fn sweep(root: &Path) {
    let reg = load(root);
    let before = list_of(&reg, "worktrees").len();
    let kept: Vec<Value> = list_of(&reg, "worktrees")
        .iter()
        .filter(|w| {
            w.get("path")
                .and_then(Value::as_str)
                .map(|p| Path::new(p).is_dir())
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    if kept.len() != before {
        let mut reg = reg;
        reg["worktrees"] = Value::Array(kept);
        let _ = save(root, &reg);
    }
}

// ── what the pickers read ───────────────────────────────────────────────

/// The branch picker's rows: every branch of every registered repo, named by
/// the branch and qualified by the repo it belongs to.
///
/// Flattened rather than grouped because the control shows one line, and two
/// repos both having `main` is the normal case — the hint is what tells them
/// apart.
pub fn branch_items(root: &Path) -> Value {
    let reg = load(root);
    let mut out = Vec::new();
    for repo in list_of(&reg, "repos") {
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
    let reg = load(root);
    Value::Array(
        list_of(&reg, "repos")
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
    let reg = load(root);
    Value::Array(
        list_of(&reg, "worktrees")
            .iter()
            .map(|w| {
                json!({
                    "id": w.get("id").cloned().unwrap_or(Value::Null),
                    "name": w.get("branch").cloned().unwrap_or(Value::Null),
                    "hint": w.get("repoName").cloned().unwrap_or(Value::Null),
                    "path": w.get("path").cloned().unwrap_or(Value::Null),
                    "icon": "tree-structure",
                })
            })
            .collect::<Vec<_>>(),
    )
}

/// The host path a sandbox should mount, given what the composer has
/// selected: the worktree if one is chosen, else the repo behind the branch.
///
/// Returns the path and the worktree id (empty when a plain repo was used),
/// because the caller has to know whether there is anything to reap.
pub fn mount_for(root: &Path, worktree_id: &str, branch_id: &str) -> Option<(String, String)> {
    if !worktree_id.is_empty() {
        if let Some(w) = find(root, "worktrees", worktree_id) {
            let p = w.get("path").and_then(Value::as_str).unwrap_or_default();
            if Path::new(p).is_dir() {
                return Some((p.to_string(), worktree_id.to_string()));
            }
        }
    }
    // `<repoId>:<branch>` — only the repo half names a directory. The branch
    // is whatever that clone currently has checked out; mounting a clone
    // does not switch it, which is exactly why worktrees exist.
    let repo_id = branch_id.split(':').next().unwrap_or_default();
    if repo_id.is_empty() {
        return None;
    }
    let repo = find(root, "repos", repo_id)?;
    let p = repo.get("path").and_then(Value::as_str).unwrap_or_default();
    if Path::new(p).is_dir() {
        return Some((p.to_string(), String::new()))
    }
    None
}

/// Everything the four pickers need, in one response.
pub fn everything(root: &Path) -> Value {
    let mut m = Map::new();
    m.insert("repos".into(), repo_items(root));
    m.insert("repoBranches".into(), branch_items(root));
    m.insert("worktrees".into(), worktree_items(root));
    Value::Object(m)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_branch_name_that_could_be_an_option_is_refused() {
        assert!(is_safe_ref("main"));
        assert!(is_safe_ref("feature/login"));
        // The one that matters: git would read this as a flag.
        assert!(!is_safe_ref("--upload-pack=touch /tmp/pwned"));
        assert!(!is_safe_ref("-b"));
        assert!(!is_safe_ref("has space"));
        assert!(!is_safe_ref("a..b"));
        assert!(!is_safe_ref(""));
        assert!(!is_safe_ref("x:y"));
    }

    #[test]
    fn a_path_that_would_break_a_mount_spec_is_refused() {
        assert!(check_path("relative/path").is_err());
        assert!(check_path("/no/such/dir/anywhere").is_err());
        assert!(check_path("/tmp/a:b").is_err());
        assert!(check_path("/tmp/a,b").is_err());
        assert!(check_path("  ").is_err());
        // A real directory that is fine.
        assert!(check_path("/tmp").is_ok());
    }

    #[test]
    fn ids_are_derived_so_registering_twice_is_idempotent() {
        assert_eq!(id_for("r", "/a/b"), id_for("r", "/a/b"));
        assert_ne!(id_for("r", "/a/b"), id_for("r", "/a/c"));
        assert_ne!(id_for("r", "/a/b"), id_for("w", "/a/b"));
    }

    #[test]
    fn a_branch_with_a_slash_becomes_one_directory() {
        let d = worktree_dir(Path::new("/root"), "chat", "feature/login");
        assert!(d.ends_with("chat-feature-login"));
    }
}

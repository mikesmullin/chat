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
//! A session owns its worktree. It is provisioned when the session is
//! saved and reaped at session delete — [`reap_session_worktree`] runs from
//! the deletion hook (and from the API's delete routes), and [`sweep`]
//! stays a boot-time pass that drops rows whose directory vanished.
//! sbox-b knows nothing about checkouts: a sandbox created from a worktree
//! never owns it, and destroying a sandbox never reaps the checkout. That
//! half is this module's.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

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
        // Git prints progress lines and the real failure to stderr; the
        // fatal line is the one that says what went wrong.
        let msg = out
            .stderr
            .lines()
            .map(str::trim)
            .find(|l| l.to_ascii_lowercase().starts_with("fatal"))
            .or_else(|| out.stderr.lines().next())
            .unwrap_or("git failed")
            .to_string();
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

/// The host uid the sandbox's `desktop` user maps to, or `None` when the
/// mapping cannot be determined.
///
/// Rootless containers run with a user namespace: their uid 0 is this host
/// user, and their uid N≥1 lands at `<this user's subuid base> + (N-1)` —
/// the subuid range starts at container uid 1, which is the one detail that
/// makes this off-by-one easy to get wrong (verified against the mapping:
/// guest-created files land on the host at base+999). The guest's `desktop`
/// is uid 1000, so base + 999. Files meant to be owned by the guest's
/// working user must be chowned to that out here — ownership is exactly
/// what the mapping rewrites. Read from `/etc/subuid`; a failure returns
/// `None` and the caller falls back to permissions alone.
fn subuid_mapped_uid() -> Option<u32> {
    let euid = current_euid()?;
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
    let name = passwd.lines().find_map(|line| {
        let mut f = line.split(':');
        let user = f.next()?;
        let _pw = f.next()?; // passwd(5): name:passwd:uid:…
        let uid: u32 = f.next()?.parse().ok()?;
        (uid == euid).then(|| user.to_string())
    })?;
    let subuid = std::fs::read_to_string("/etc/subuid").ok()?;
    let base: u64 = subuid.lines().find_map(|line| {
        let (user, rest) = line.split_once(':')?;
        let base: u64 = rest.split(':').next()?.parse().ok()?;
        (user == name).then_some(base)
    })?;
    // container uid 1000 (desktop) → base + (1000 - 1); see the doc.
    u32::try_from(base + 999).ok()
}

/// This process's effective uid (the owner of everything provisioned).
fn current_euid() -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self").ok().map(|m| m.uid())
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
    // The checkout trusted host-side git with a safe.directory entry (its
    // owner is the guest's mapped uid); the trust goes with the directory.
    if !path.is_empty() {
        let _ = Command::new("git")
            .args(["config", "--global", "--unset-all", "safe.directory", path])
            .output();
    }
    Ok(json!({ "removed": id, "note": note }))
}

// ── session worktrees: one checkout per session ────────────────────────
//
// A session owns its worktree. It is provisioned when the session is
// saved — the settings route resolves the config the client sent — and
// reaped at session delete. Sandboxes are disposable and never own a
// checkout: a container may mount one read-write, and a dead container
// never reaps it — the session does.

/// Where a session's checkout lives by default. Beside the registry, so
/// a checkout chat made is recognisable as chat's.
fn default_worktree_parent(root: &Path) -> PathBuf {
    root.join(".angela").join("worktrees")
}

/// A branch's directory prefix: first 23 BYTES, `/` folded to `-`
/// (`feature/login` is one directory, not two). Bytes, not characters —
/// the path is a byte string — so a multi-byte character is never split.
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

/// The name hash of a checkout: the first 16 lowercase hex chars of
/// sha256 over `<common_dir>\0<branch>`. Two repos can hold the same
/// branch name; the common dir is what keeps them apart.
fn branch_hash(common_dir: &str, branch: &str) -> String {
    let mut h = Sha256::new();
    h.update(common_dir.as_bytes());
    h.update([0]);
    h.update(branch.as_bytes());
    let mut hex = format!("{:x}", h.finalize());
    hex.truncate(16);
    hex
}

/// Eight random lowercase hex chars, one fresh draw per call.
fn random_hex() -> String {
    use std::io::Read;
    let mut buf = [0u8; 4];
    let mut ok = false;
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        ok = f.read_exact(&mut buf).is_ok();
    }
    if !ok {
        // No /dev/urandom (uncommon): fold the clock and the pid together.
        // Not cryptographic — just a fresh-enough separator.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut h = Sha256::new();
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

/// Expand a worktree path pattern into a path.
///
/// Placeholders: `{{parent}}`, `{{repo}}`, `{{prefix}}` (branch truncated
/// to its first 23 bytes, `/` folded to `-`), `{{hash}}` (sha256 of
/// `<common_dir>\0<branch>`, first 16 hex chars), `{{branch}}` and
/// `{{hex}}` (8 random lowercase hex chars, one fresh draw per call —
/// used wherever it appears, so a pattern that mentions it twice still
/// names one checkout). Unknown placeholders are an error, as are empty
/// results and paths that would break a mount spec (`:` and `,`).
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
            "the worktree path cannot contain ':' or ',' — a mount spec is delimited by both"
                .into(),
        );
    }
    Ok(out)
}

/// Expand a branch pattern. Only `{{hex}}` and literal text are accepted:
/// every other placeholder is derived from the branch itself, which is
/// what the pattern is computing.
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

/// The repo's git common dir, resolved to an absolute path. In a plain
/// clone it is the `.git` beside the checkout; in a worktree it is the
/// shared dir of the main checkout.
fn git_common_dir(repo: &Path) -> Result<String, String> {
    let raw = git(repo, &["rev-parse", "--git-common-dir"])?;
    let rel = raw.trim();
    if rel.is_empty() {
        return Err("git reported no common dir".into());
    }
    let p = if Path::new(rel).is_absolute() {
        PathBuf::from(rel)
    } else {
        repo.join(rel)
    };
    let canon = p
        .canonicalize()
        .map_err(|e| format!("cannot resolve the git common dir: {e}"))?;
    Ok(canon.to_string_lossy().to_string())
}

/// A resolved record from a registry worktree row — the idempotency half
/// of provisioning hands back the same shape a fresh provision would.
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

/// Provision a session's worktree: resolve the branch, lay the checkout
/// down, register it.
///
/// `branch_pattern` is a template over `{{hex}}` (one random draw, used
/// everywhere the placeholder appears) and literal text; `path_pattern`
/// defaults to `{{parent}}/{{repo}}/{{prefix}}-{{hash}}` with the parent
/// being `<root>/.angela/worktrees`. Returns the resolved record
/// `{repoId, repoName, branch, path, branchPattern, pathPattern,
/// provisioned: true}`.
///
/// Idempotent where it is safe to be: a registry row with the same path
/// and session that still has its directory wins, and no git runs. An
/// existing path anything else occupies is an error, never a clobber.
pub fn provision_session_worktree(
    root: &Path,
    session_id: &str,
    repo_id: &str,
    branch_pattern: &str,
    path_pattern: &str,
) -> Result<Value, String> {
    provision_session_worktree_full(root, session_id, repo_id, branch_pattern, path_pattern, None)
}

/// Re-provision on an exact, recorded branch — the restore half of
/// [`session_worktree_ready`]. No `{{hex}}` is drawn: the branch is fixed,
/// so the path (which hashes the branch) is too. The record's
/// `branchPattern` is the branch itself — with an explicit override there
/// is no other template to record.
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
    // (a) The repo must exist, and its clone must still be there.
    let repo = find(root, "repos", repo_id).ok_or("no such repo")?;
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

    // (b) Expand the branch pattern once — one random draw, so the branch
    // the record holds is the branch git gets. An override is the
    // recorded branch, which must also be safe to pass to git.
    let branch = match branch_override {
        Some(b) => b.to_string(),
        None => expand_branch_pattern(branch_pattern)?,
    };
    if !is_safe_ref(&branch) {
        return Err(format!("not a usable branch name: {branch}"));
    }

    // (c) The common dir names the checkout: the hash is sha256 over
    // `<common_dir>\0<branch>`, so two repos holding the same branch
    // name still land in different paths.
    let common_dir = git_common_dir(&repo_path)?;

    // (d) Expand the path pattern. The parent is always the registry's
    // own worktrees dir; the parent of the resolved path must be creatable
    // before anything is attempted.
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

    // (e) Idempotency / refusal. Same path + same session + the directory
    // is still there → the row wins, no git runs. Anything else occupying
    // the path → error, never a clobber.
    let target = PathBuf::from(&path_s);
    let row = list_of(&load(root), "worktrees")
        .iter()
        .find(|w| {
            w.get("path").and_then(Value::as_str) == Some(path_s.as_str())
                && w.get("sessionId").and_then(Value::as_str) == Some(session_id)
        })
        .cloned();
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

    // (f) Create the worktree, `-b` only when the branch does not exist
    // yet. The two forms take arguments in opposite orders.
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
        // A hand-deleted directory leaves a stale registry entry in git.
        // Prune it — the checkout is gone, so dropping the entry loses
        // nothing — and retry once.
        if !e.contains("missing but already registered worktree") {
            return Err(e);
        }
        let _ = git(&repo_path, &["worktree", "prune"]);
        git(&repo_path, &args)?;
    }

    // (g) The checkout must belong to the sandbox's working user. Rootless
    // containers map their uids through the host's subuid range — the
    // guest's `desktop` (uid 1000) is `<subuid base + 1000>` out here — so
    // a checkout owned by this host user shows up root-owned inside the
    // guest and the agent could not write a line. Give it to the mapped
    // owner (chown needs the sudo this host grants its user), keeping our
    // own access through named ACL entries set while we still own the
    // tree — and a default ACL so files the guest creates keep both
    // halves reachable. A failure degrades to permissions alone.
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
    // The checkout now belongs to the mapped guest uid, which host-side git
    // reads as "dubious ownership". Trust this path explicitly (and only
    // this path — the reap removes the entry with the checkout).
    let _ = Command::new("git")
        .args(["config", "--global", "--add", "safe.directory", &path_s])
        .output();

    // (h) Register the row. The id is path-derived, so re-registering an
    // existing path replaces its row rather than duplicating it. The
    // `repoPath` goes in the row and the record: a sandbox that mounts the
    // checkout also mounts `<repoPath>/.git` at its own absolute path, so
    // the worktree's absolute gitdir pointer resolves inside the guest.
    let id = id_for("w", &path_s);
    let mut reg = load(root);
    let mut list: Vec<Value> = list_of(&reg, "worktrees").to_vec();
    list.retain(|w| w.get("id").and_then(Value::as_str) != Some(id.as_str()));
    list.push(json!({
        "id": id,
        "repoId": repo_id,
        "repoName": repo_name,
        "repoPath": repo_path,
        "branch": branch,
        "path": path_s,
        "sessionId": session_id,
        "branchPattern": branch_pattern,
        "pathPattern": path_pattern,
        // The session owns the checkout; no sandbox claims it.
        "sandboxId": "",
    }));
    reg["worktrees"] = Value::Array(list);
    save(root, &reg)?;

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

/// Reap every worktree the session owns — best-effort, errors ignored: a
/// checkout git has already lost track of is still a row to remove.
pub fn reap_session_worktree(root: &Path, session_id: &str) {
    if session_id.trim().is_empty() {
        return;
    }
    let reg = load(root);
    let ids: Vec<String> = list_of(&reg, "worktrees")
        .iter()
        .filter(|w| w.get("sessionId").and_then(Value::as_str) == Some(session_id))
        .filter_map(|w| w.get("id").and_then(Value::as_str).map(str::to_string))
        .collect();
    for id in ids {
        let _ = remove_worktree(root, &id);
    }
}

/// Reap every registry worktree row that belongs to a session — the bulk
/// delete's half, since the session rows go with it.
pub fn reap_all_session_worktrees(root: &Path) {
    let reg = load(root);
    let ids: Vec<String> = list_of(&reg, "worktrees")
        .iter()
        .filter(|w| !w.get("sessionId").and_then(Value::as_str).unwrap_or_default().trim().is_empty())
        .filter_map(|w| w.get("id").and_then(Value::as_str).map(str::to_string))
        .collect();
    for id in ids {
        let _ = remove_worktree(root, &id);
    }
}

/// The gate's readiness answer for one session's meta: the workspace root
/// set, and a provisioned worktree whose directory is still there.
///
/// A vanished worktree is re-provisioned here — the recorded branch is
/// used verbatim — so a hand-deleted checkout is repaired rather than
/// parked on forever.
pub fn session_worktree_ready(root: &Path, meta: &Value) -> Result<(), String> {
    let ws = meta
        .get("workspaceRoot")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if ws.is_empty() {
        return Err("workspace root is not set".into());
    }
    if !ws.starts_with('/') {
        return Err(format!("workspace root must be absolute: {ws}"));
    }
    if !Path::new(&ws).is_dir() {
        return Err(format!("workspace root is not a directory: {ws}"));
    }

    let Some(record) = meta.get("worktree").and_then(Value::as_object) else {
        return Err("worktree not provisioned".into());
    };
    let provisioned = record.get("provisioned").and_then(Value::as_bool).unwrap_or(false);
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
        // The directory is gone — most often a hand deletion. Restore
        // the checkout onto the recorded branch before answering, so the
        // turn starts against a real path. No {{hex}} is drawn: the
        // branch is fixed, so the path is too.
        let repo_id = record.get("repoId").and_then(Value::as_str).unwrap_or_default();
        let branch = record.get("branch").and_then(Value::as_str).unwrap_or_default();
        let path_pattern = record.get("pathPattern").and_then(Value::as_str).unwrap_or("");
        let session_id = meta.get("id").and_then(Value::as_str).unwrap_or_default();
        if repo_id.is_empty() || branch.is_empty() {
            return Err("worktree missing".into());
        }
        // The recorded branch is used verbatim — no {{hex}} is drawn, so
        // the path is the same as the one recorded.
        if let Err(e) = provision_session_worktree_on_branch(
            root,
            session_id,
            repo_id,
            branch,
            path_pattern,
        ) {
            return Err(format!("worktree missing: {e}"));
        }
        if !Path::new(&path).is_dir() {
            return Err("worktree missing".into());
        }
    }
    Ok(())
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

    // ── session-worktree provisioning ────────────────────────────────

    fn temp_dir(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "chat-hostgit-{tag}-{}-{n}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// A git fixture: one commit on main. Returns the repo path and the
    /// registered repo id.
    fn fixture_repo(root: &Path) -> (PathBuf, String) {
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args(args)
                .env("GIT_TERMINAL_PROMPT", "0")
                .output()
                .expect("git");
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["init", "-b", "main"]);
        run(&[
            "-c", "user.email=test@example.com", "-c", "user.name=Test",
            "commit", "--allow-empty", "-m", "init",
        ]);
        let id = add_repo(root, repo.to_str().unwrap()).unwrap()["id"].as_str().unwrap().to_string();
        (repo, id)
    }

    #[test]
    fn the_checkout_hash_is_sha256_over_common_dir_nul_branch() {
        // Known vector for the crate wiring.
        let mut h = Sha256::new();
        h.update(b"abc");
        assert_eq!(
            format!("{:x}", h.finalize()),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // The composition actually used: sha256("/tmp/repo/.git\0feature/login").
        assert_eq!(branch_hash("/tmp/repo/.git", "feature/login"), "36a1b3b203293506");
    }

    #[test]
    fn the_path_pattern_expands_every_placeholder_and_refuses_bad_input() {
        let parent = PathBuf::from("/root/.angela/worktrees");
        let exact = expand_worktree_path(
            &parent, "repo", "feature/login", "/common/.git",
            "{{parent}}/{{repo}}/{{prefix}}-{{hash}}",
        )
        .unwrap();
        let prefix = format!("{}/repo/feature-login-", parent.display());
        assert!(exact.starts_with(&prefix));
        let hash = &exact[prefix.len()..];
        assert_eq!(hash.len(), 16);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
        // The hash is stable for a given common dir and branch.
        assert_eq!(
            expand_worktree_path(&parent, "repo", "feature/login", "/common/.git", "{{hash}}").unwrap(),
            hash
        );
        // {{hex}}: eight lowercase hex chars, one fresh draw per call.
        let hex = expand_worktree_path(&parent, "repo", "feature/login", "/common/.git", "{{hex}}").unwrap();
        assert_eq!(hex.len(), 8);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
        // {{branch}} is the full branch.
        assert_eq!(
            expand_worktree_path(&parent, "repo", "feature/login", "/common/.git", "{{branch}}").unwrap(),
            "feature/login"
        );
        // Unknown / unterminated placeholders.
        assert!(expand_worktree_path(&parent, "repo", "feature/login", "/common/.git", "{{nope}}").is_err());
        assert!(expand_worktree_path(&parent, "repo", "feature/login", "/common/.git", "{{parent").is_err());
        // Empty results.
        assert!(expand_worktree_path(&parent, "repo", "feature/login", "/common/.git", "").is_err());
        // Mount-spec characters are refused.
        assert!(expand_worktree_path(&parent, "repo", "a:b", "/common/.git", "{{branch}}").is_err());
        assert!(expand_worktree_path(&parent, "repo", "a,b", "/common/.git", "{{branch}}").is_err());
    }

    #[test]
    fn the_branch_pattern_expands_only_hex_and_literals() {
        let branch = expand_branch_pattern("feature/{{hex}}").unwrap();
        assert!(branch.starts_with("feature/"));
        assert_eq!(branch.len(), "feature/".len() + 8);
        assert!(!is_safe_ref(&branch) == false, "{branch} should be a safe ref");
        assert!(expand_branch_pattern("{{prefix}}").is_err());
        assert!(expand_branch_pattern("{{hex").is_err());
        assert!(expand_branch_pattern("").is_err());
    }

    #[test]
    fn a_worktree_is_provisioned_then_idempotently_reused_then_reaped() {
        let root = temp_dir("provision");
        let (_repo, repo_id) = fixture_repo(&root);
        let record = provision_session_worktree(&root, "session-1", &repo_id, "feature/worktree", "").unwrap();
        assert_eq!(record["provisioned"], true);
        assert_eq!(record["branch"], "feature/worktree");
        assert_eq!(record["repoId"], repo_id);
        let path = record["path"].as_str().unwrap().to_string();
        assert!(Path::new(&path).is_dir());
        // The default layout: the registry's worktrees dir, the repo,
        // the prefix-hash.
        assert!(path.starts_with(&default_worktree_parent(&root).to_string_lossy().to_string()));
        let reg = load(&root);
        let row = list_of(&reg, "worktrees").first().unwrap().clone();
        assert_eq!(row["sessionId"], "session-1");
        assert_eq!(row["sandboxId"], "");
        assert_eq!(row["path"], path);
        assert_eq!(row["branchPattern"], "feature/worktree");
        // The branch was created in the repo.
        assert!(git(&_repo, &["rev-parse", "--verify", "refs/heads/feature/worktree"]).is_ok());

        // Idempotent: the same config returns the row without git.
        let record2 = provision_session_worktree(&root, "session-1", &repo_id, "feature/worktree", "").unwrap();
        assert_eq!(record2["path"], record["path"]);
        assert_eq!(record2["branch"], record["branch"]);
        assert_eq!(record2["provisioned"], true);
        assert_eq!(list_of(&load(&root), "worktrees").len(), 1);

        // Reap: the checkout and its row go.
        reap_session_worktree(&root, "session-1");
        assert!(!Path::new(&path).exists());
        assert_eq!(list_of(&load(&root), "worktrees").len(), 0);
    }

    #[test]
    fn a_random_branch_pattern_records_the_branch_it_created() {
        let root = temp_dir("random");
        let (_repo, repo_id) = fixture_repo(&root);
        let record = provision_session_worktree(&root, "session-1", &repo_id, "feature/{{hex}}", "").unwrap();
        let branch = record["branch"].as_str().unwrap();
        assert!(branch.starts_with("feature/"), "{branch}");
        assert!(is_safe_ref(branch), "{branch}");
        // The branch the record holds is the branch git created.
        assert!(git(&_repo, &["rev-parse", "--verify", &format!("refs/heads/{branch}")]).is_ok());
        assert!(Path::new(record["path"].as_str().unwrap()).is_dir());
    }

    #[test]
    fn a_path_occupied_by_a_different_session_is_refused() {
        let root = temp_dir("collision");
        let (_repo, repo_id) = fixture_repo(&root);
        provision_session_worktree(&root, "session-1", &repo_id, "feature/worktree", "").unwrap();
        let err = provision_session_worktree(&root, "session-2", &repo_id, "feature/worktree", "").unwrap_err();
        assert!(err.contains("a directory already exists"), "{err}");
        // session-1's row is untouched.
        let reg = load(&root);
        assert_eq!(list_of(&reg, "worktrees").len(), 1);
    }

    #[test]
    fn reap_takes_only_the_session_own_rows() {
        let root = temp_dir("reap");
        let (_repo, repo_id) = fixture_repo(&root);
        let rec1 = provision_session_worktree(&root, "session-1", &repo_id, "feature/one", "").unwrap();
        let rec2 = provision_session_worktree(&root, "session-2", &repo_id, "feature/two", "").unwrap();
        reap_session_worktree(&root, "session-1");
        assert!(!Path::new(rec1["path"].as_str().unwrap()).exists());
        assert!(Path::new(rec2["path"].as_str().unwrap()).is_dir());
        let reg = load(&root);
        let rows = list_of(&reg, "worktrees");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["sessionId"], "session-2");
    }

    #[test]
    fn reap_all_takes_every_session_owned_row() {
        let root = temp_dir("reap-all");
        let (_repo, repo_id) = fixture_repo(&root);
        let rec1 = provision_session_worktree(&root, "session-1", &repo_id, "feature/one", "").unwrap();
        let _rec2 = provision_session_worktree(&root, "session-2", &repo_id, "feature/two", "").unwrap();
        // A hand-made row — no sessionId, not this module's to reap.
        add_branch(&root, &repo_id, "handmade", "main").unwrap();
        add_worktree(&root, &repo_id, "handmade", false).unwrap();
        reap_all_session_worktrees(&root);
        assert!(!Path::new(rec1["path"].as_str().unwrap()).exists());
        let reg = load(&root);
        let rows = list_of(&reg, "worktrees");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["branch"], "handmade");
        // The hand-made row carries no session — a worktree no session
        // owns is not this module's to reap.
        assert!(rows[0].get("sessionId").map(|v| v.is_string()).unwrap_or(false) == false
            || rows[0]["sessionId"].as_str().map(|s| s.is_empty()).unwrap_or(true));
    }

    #[test]
    fn the_gate_refuses_an_unconfigured_session() {
        let root = temp_dir("gate");
        let bad = json!({ "id": "session-1" });
        let err = session_worktree_ready(&root, &bad).unwrap_err();
        assert!(err.contains("workspace root"), "{err}");

        let bad_relative = json!({ "id": "session-1", "workspaceRoot": "relative/path" });
        let err = session_worktree_ready(&root, &bad_relative).unwrap_err();
        assert!(err.contains("workspace root"), "{err}");

        let bad_dir = json!({ "id": "session-1", "workspaceRoot": root.join("no-such") });
        let err = session_worktree_ready(&root, &bad_dir).unwrap_err();
        assert!(err.contains("workspace root"), "{err}");

        let bad_wt = json!({
            "id": "session-1",
            "workspaceRoot": root,
            "worktree": { "provisioned": false, "path": "/tmp/x" },
        });
        let err = session_worktree_ready(&root, &bad_wt).unwrap_err();
        assert!(err.contains("worktree not provisioned"), "{err}");

        let bad_path = json!({
            "id": "session-1",
            "workspaceRoot": root,
            "worktree": { "provisioned": true, "path": "" },
        });
        assert!(session_worktree_ready(&root, &bad_path).is_err());
    }

    #[test]
    fn the_gate_restores_a_deleted_worktree() {
        let root = temp_dir("restore");
        let (_repo, repo_id) = fixture_repo(&root);
        let record = provision_session_worktree(&root, "session-1", &repo_id, "feature/restore", "").unwrap();
        let path = record["path"].as_str().unwrap().to_string();
        std::fs::remove_dir_all(&path).unwrap();
        assert!(!Path::new(&path).exists());

        let meta = json!({
            "id": "session-1",
            "workspaceRoot": root,
            "worktree": record,
        });
        let r = session_worktree_ready(&root, &meta);
        assert!(r.is_ok(), "ready: {r:?}");
        assert!(Path::new(&path).is_dir());
        // The recorded branch is re-created in the repo.
        assert!(git(&_repo, &["rev-parse", "--verify", "refs/heads/feature/restore"]).is_ok());
        // And the registry row is back.
        assert_eq!(list_of(&load(&root), "worktrees").len(), 1);
    }
}

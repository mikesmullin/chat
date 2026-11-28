use std::path::Path;
use std::process::Command;

pub const GIT_TIMEOUT_SECS: u64 = 60;

pub struct Output {
    pub code: i32,
    pub status_ok: bool,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Output {
    pub fn stdout_str(&self) -> String {
        String::from_utf8_lossy(&self.stdout).to_string()
    }
}

pub fn run(dir: &Path, args: &[&str]) -> Result<Output, String> {
    run_env(dir, args, &[])
}

pub fn run_env(dir: &Path, args: &[&str], extra_env: &[(&str, &str)]) -> Result<Output, String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for a in args {
        cmd.arg(a);
    }
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.env("GIT_OPTIONAL_LOCKS", "0");
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    run_with_timeout(cmd)
}

pub fn run_with_timeout(cmd: Command) -> Result<Output, String> {
    run_command(cmd, GIT_TIMEOUT_SECS)
}

pub fn run_command(mut cmd: Command, timeout_secs: u64) -> Result<Output, String> {
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("could not run git: {e}"))?;
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("command timed out after {timeout_secs}s"));
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("could not read git output: {e}"))?;
    Ok(Output {
        code: out.status.code().unwrap_or(-1),
        status_ok: out.status.success(),
        stdout: out.stdout,
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    })
}

pub fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = run(dir, args)?;
    if out.status_ok {
        Ok(out.stdout_str())
    } else {
        Err(failure_message(&out.stderr))
    }
}

pub fn failure_message(stderr: &str) -> String {
    stderr
        .lines()
        .map(str::trim)
        .find(|l| l.to_ascii_lowercase().starts_with("fatal"))
        .or_else(|| stderr.lines().next())
        .unwrap_or("git failed")
        .to_string()
}

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

pub fn validate_branch_field(field: &str, value: &str) -> Result<(), crate::err::ApiError> {
    use crate::err::validation_field;
    if value.is_empty() || value.len() > 200 {
        return Err(validation_field(
            field,
            &format!("length_1_200"),
            format!("field '{field}' must be 1–200 characters"),
        ));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '-'))
    {
        return Err(validation_field(
            field,
            "pattern_mismatch",
            format!("field '{field}' is invalid: only letters, digits, '.', '_', '/', '-' allowed"),
        ));
    }
    if value.starts_with('/') || value.starts_with('-') {
        return Err(validation_field(
            field,
            "leading_slash_or_dash",
            format!("field '{field}' must not start with '/' or '-'"),
        ));
    }
    if value.contains("..") {
        return Err(validation_field(
            field,
            "double_dot",
            format!("field '{field}' must not contain '..'"),
        ));
    }
    if value.contains("//") {
        return Err(validation_field(
            field,
            "double_slash",
            format!("field '{field}' must not contain '//'"),
        ));
    }
    if value.ends_with('/') || value.ends_with(".lock") {
        return Err(validation_field(
            field,
            "bad_suffix",
            format!("field '{field}' must not end with '/' or '.lock'"),
        ));
    }
    Ok(())
}

pub fn resolve_branch_head(dir: &Path, branch: &str) -> Option<String> {
    let out = run(
        dir,
        &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")],
    )
    .ok()?;
    if out.status_ok {
        Some(out.stdout_str().trim().to_string())
    } else {
        None
    }
}

pub fn branches(dir: &Path) -> Vec<(String, String)> {
    let out = run(
        dir,
        &[
            "for-each-ref",
            "refs/heads",
            "--format=%(objectname)%09%(refname:short)",
        ],
    );
    match out {
        Ok(out) if out.status_ok => out
            .stdout_str()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| {
                let i = l.find('\t')?;
                Some((l[i + 1..].to_string(), l[..i].to_string()))
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub enum MergeProbe {
    Clean { tree: String },
    Conflicts { files: Vec<String> },
    Unknown,
}

pub fn merge_tree_probe(dir: &Path, target_sha: &str, source_sha: &str) -> MergeProbe {
    let out = match run(
        dir,
        &[
            "merge-tree",
            "--write-tree",
            "--name-only",
            target_sha,
            source_sha,
        ],
    ) {
        Ok(out) => out,
        Err(_) => return MergeProbe::Unknown,
    };
    let text = out.stdout_str();
    if out.code == 0 {
        let tree = text.lines().next().unwrap_or("").trim().to_string();
        if tree.is_empty() {
            return MergeProbe::Unknown;
        }
        return MergeProbe::Clean { tree };
    }
    if out.code == 1 {
        let mut files = Vec::new();
        for line in text.lines().skip(1) {
            if line.is_empty() {
                break;
            }
            files.push(line.trim().to_string());
        }
        return MergeProbe::Conflicts { files };
    }
    MergeProbe::Unknown
}

pub fn is_ancestor(dir: &Path, ancestor: &str, descendant: &str) -> bool {
    run(
        dir,
        &["merge-base", "--is-ancestor", ancestor, descendant],
    )
    .map(|o| o.status_ok)
    .unwrap_or(false)
}

pub fn git_common_dir(dir: &Path) -> Option<String> {
    let raw = git(dir, &["rev-parse", "--git-common-dir"]).ok()?;
    let rel = raw.trim();
    if rel.is_empty() {
        return None;
    }
    let p = if Path::new(rel).is_absolute() {
        std::path::PathBuf::from(rel)
    } else {
        dir.join(rel)
    };
    p.canonicalize()
        .ok()
        .map(|c| c.to_string_lossy().to_string())
}

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};

use serde_json::{json, Value};

use super::drivers;
use super::store;
use crate::err::ApiError;

struct Park {
    answered: Option<String>,
    stopped: bool,
}

fn parks() -> &'static Mutex<HashMap<i64, Arc<(Mutex<Park>, Condvar)>>> {
    static PARKS: std::sync::OnceLock<Mutex<HashMap<i64, Arc<(Mutex<Park>, Condvar)>>>> =
        std::sync::OnceLock::new();
    PARKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn aborts() -> &'static Mutex<HashMap<i64, Arc<Mutex<bool>>>> {
    static ABORTS: std::sync::OnceLock<Mutex<HashMap<i64, Arc<Mutex<bool>>>>> =
        std::sync::OnceLock::new();
    ABORTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn park_for(run_id: i64) -> Arc<(Mutex<Park>, Condvar)> {
    let mut map = parks().lock().unwrap();
    map.entry(run_id)
        .or_insert_with(|| Arc::new((Mutex::new(Park { answered: None, stopped: false }), Condvar::new())))
        .clone()
}

fn abort_flag(run_id: i64) -> Arc<Mutex<bool>> {
    let mut map = aborts().lock().unwrap();
    map.entry(run_id)
        .or_insert_with(|| Arc::new(Mutex::new(false)))
        .clone()
}

fn forget_run(run_id: i64) {
    parks().lock().unwrap().remove(&run_id);
    aborts().lock().unwrap().remove(&run_id);
}

fn journal(
    root: &Path,
    run_id: i64,
    stage: Option<&str>,
    kind: &str,
    level: &str,
    message: &str,
    data: &Value,
) {
    let _ = crate::db::with(root, |c| {
        store::journal(c, run_id, stage, kind, level, message, data);
        Ok::<(), String>(())
    });
}

fn set_stage(
    root: &Path,
    run_id: i64,
    name: &str,
    state: &str,
    sandbox_id: Option<&str>,
    session_id: Option<&str>,
    error: Option<&str>,
    started: bool,
    ended: bool,
) {
    let _ = crate::db::with(root, |c| {
        store::update_stage(c, run_id, name, state, sandbox_id, session_id, error, started, ended)
    });
}

fn set_run(
    root: &Path,
    run_id: i64,
    state: &str,
    error: Option<&str>,
    started: bool,
    ended: bool,
) {
    let _ = crate::db::with(root, |c| {
        store::set_run_state(c, run_id, state, error, started, ended)
    });
    crate::events::bump_jobs();
}

pub fn start(root: &Path) {
    finalize_boot(root);
    let root = root.to_path_buf();
    std::thread::spawn(move || loop {
        pump(&root);
        std::thread::sleep(std::time::Duration::from_secs(2));
    });
}

fn finalize_boot(root: &Path) {
    let active: Vec<Value> = crate::db::with(root, |c| {
        let runs = store::list_runs(c, None, "all", 200);
        runs.into_iter()
            .filter(|r| {
                store::active_run_states().contains(
                    &r.get("state").and_then(Value::as_str).unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    for run in active {
        let id = run.get("id").and_then(Value::as_i64).unwrap_or(0);
        journal(
            root,
            id,
            None,
            "note",
            "warn",
            "run interrupted by restart; finalized as aborted",
            &json!({}),
        );
        finalize_run(root, id, "aborted", Some("interrupted by restart"));
    }
}

fn pump(root: &Path) {
    let queued: Vec<Value> =
        crate::db::with(root, |c| store::list_runs(c, None, "queued", 4)).unwrap_or_default();
    for run in queued {
        let id = run.get("id").and_then(Value::as_i64).unwrap_or(0);
        let job_id = run.get("jobId").and_then(Value::as_i64).unwrap_or(0);
        let _ = crate::db::with(root, |c| super::queue::claim_next(c));
        let _ = job_id;
        set_run(root, id, "running", None, true, false);
        let root_owned = root.to_path_buf();
        std::thread::spawn(move || execute_run(&root_owned, id));
    }
}

fn env_of(run: &Value) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Some(map) = run.get("env").and_then(Value::as_object) {
        for (k, v) in map {
            if let Some(s) = v.as_str() {
                out.insert(k.clone(), s.to_string());
            }
        }
    }
    out
}

fn step_env(run_env: &BTreeMap<String, String>, stage: &Value) -> BTreeMap<String, String> {
    let mut out = run_env.clone();
    if let Some(map) = stage.get("env").and_then(Value::as_object) {
        for (k, v) in map {
            if let Some(s) = v.as_str() {
                out.insert(k.clone(), s.to_string());
            }
        }
    }
    out
}

fn render_tokens(text: &str, run: &Value) -> String {
    text.replace("<run_id>", &run.get("id").and_then(Value::as_i64).unwrap_or(0).to_string())
        .replace("<run_number>", &run.get("number").and_then(Value::as_i64).unwrap_or(0).to_string())
        .replace("<job>", run.get("jobName").and_then(Value::as_str).unwrap_or_default())
        .replace("<repo>", run.get("env").and_then(|e| e.get("REPO_NAME")).and_then(Value::as_str).unwrap_or_default())
        .replace("<branch>", run.get("env").and_then(|e| e.get("REPO_BRANCH")).and_then(Value::as_str).unwrap_or_default())
}

struct RunContext {
    root: std::path::PathBuf,
    run: Value,
    run_env: BTreeMap<String, String>,
    aborted: Arc<Mutex<bool>>,
    deadline: Option<std::time::Instant>,
    sandbox: Option<String>,
    worktree: Option<String>,
    session_ids: HashMap<String, String>,
    failed: bool,
}

fn execute_run(root: &Path, run_id: i64) {
    let Some(run) = crate::db::with(root, |c| store::run_by_id(c, run_id)).ok().flatten() else {
        return;
    };
    let job_name = run.get("jobName").and_then(Value::as_str).unwrap_or_default().to_string();
    let job = crate::db::with(root, |c| store::job_by_name(c, &job_name))
        .ok()
        .flatten()
        .unwrap_or(Value::Null);
    let definition = job.get("definition").cloned().unwrap_or(Value::Null);
    let run_env = env_of(&run);

    let repo_path = run_env.get("REPO_PATH").cloned();
    let branch = run_env.get("REPO_BRANCH").cloned().unwrap_or_default();
    let mut worktree = None;
    if let Some(repo_path) = &repo_path {
        if !repo_path.is_empty() && Path::new(repo_path).is_dir() {
            let dir = super::worktree::run_worktree_dir(root, run_id);
            match super::worktree::create_run_worktree(Path::new(repo_path), &branch, &dir) {
                Ok((head, _gitdir)) => {
                    journal(
                        root,
                        run_id,
                        None,
                        "note",
                        "info",
                        &format!("run worktree at {} ({}…)", dir.display(), &head[..7.min(head.len())]),
                        &json!({ "path": dir.to_string_lossy(), "commit": head }),
                    );
                    worktree = Some(dir.to_string_lossy().to_string());
                }
                Err(e) => {
                    journal(root, run_id, None, "error", "error", &e, &json!({}));
                }
            }
        }
    }

    let mut ctx = RunContext {
        root: root.to_path_buf(),
        run: run.clone(),
        run_env: run_env.clone(),
        aborted: abort_flag(run_id),
        deadline: None,
        sandbox: None,
        worktree: worktree.clone(),
        session_ids: HashMap::new(),
        failed: false,
    };

    let stages = definition.get("stages").and_then(Value::as_array).cloned().unwrap_or_default();
    crate::db::with(root, |c| {
        for (i, stage) in stages.iter().enumerate() {
            let name = stage.get("name").and_then(Value::as_str).unwrap_or_default();
            if stage.get("parallel").and_then(Value::as_array).is_some() {
                let _ = store::insert_stage(c, run_id, name, i as i64, None);
                for branch_stage in stage.get("parallel").and_then(Value::as_array).unwrap() {
                    let bname = branch_stage.get("name").and_then(Value::as_str).unwrap_or_default();
                    let _ = store::insert_stage(c, run_id, bname, i as i64, Some(name));
                }
            } else {
                let _ = store::insert_stage(c, run_id, name, i as i64, None);
            }
        }
        Ok::<(), String>(())
    })
    .ok();

    for stage in &stages {
        if ctx.failed || *ctx.aborted.lock().unwrap() {
            break;
        }
        if stage.get("parallel").and_then(Value::as_array).is_some() {
            walk_fork(&mut ctx, stage);
        } else {
            walk_stage(&mut ctx, stage, None);
        }
    }

    if *ctx.aborted.lock().unwrap() {
        finalize_run(root, run_id, "aborted", Some("stopped"));
    } else if ctx.failed {
        finalize_run(root, run_id, "failure", Some("a stage failed"));
    } else {
        set_run(root, run_id, "posting", None, false, false);
        let outcome = walk_posts(&mut ctx, &definition);
        if outcome {
            finalize_run(root, run_id, "success", None);
        } else {
            finalize_run(root, run_id, "failure", Some("post-condition failed"));
        }
    }

    if let Some(sandbox) = ctx.sandbox.take() {
        drivers::sandbox_destroy(root, &sandbox);
    }
    if let (Some(repo_path), Some(wt)) = (repo_path, worktree) {
        super::worktree::remove_run_worktree(Path::new(&repo_path), Path::new(&wt));
    }
    forget_run(run_id);
}

fn finalize_run(root: &Path, run_id: i64, state: &str, error: Option<&str>) {
    crate::db::with(root, |c| {
        for stage in store::stage_rows(c, run_id) {
            let sstate = stage.get("state").and_then(Value::as_str).unwrap_or_default();
            let name = stage.get("name").and_then(Value::as_str).unwrap_or_default();
            match sstate {
                "pending" => {
                    let _ = store::update_stage(c, run_id, name, "skipped", None, None, None, false, true);
                }
                "provisioning" | "running" | "blocked" => {
                    let _ = store::update_stage(c, run_id, name, "aborted", None, None, None, false, true);
                }
                _ => {}
            }
        }
        Ok::<(), String>(())
    })
    .ok();
    set_run(root, run_id, state, error, false, true);
}

fn walk_fork(ctx: &mut RunContext, fork: &Value) {
    let group = fork.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    let branches = fork.get("parallel").and_then(Value::as_array).cloned().unwrap_or_default();
    let root = ctx.root.clone();
    let run = ctx.run.clone();
    let run_env = ctx.run_env.clone();
    let aborted = Arc::clone(&ctx.aborted);
    let handles: Vec<_> = branches
        .into_iter()
        .map(|branch_stage| {
            let root = root.clone();
            let run = run.clone();
            let run_env = run_env.clone();
            let aborted = Arc::clone(&aborted);
            let group = group.clone();
            std::thread::spawn(move || {
                let mut branch_ctx = RunContext {
                    root,
                    run,
                    run_env,
                    aborted,
                    deadline: None,
                    sandbox: None,
                    worktree: None,
                    session_ids: HashMap::new(),
                    failed: false,
                };
                walk_stage(&mut branch_ctx, &branch_stage, Some(&group));
                (branch_ctx.failed, branch_ctx.sandbox, branch_ctx.session_ids)
            })
        })
        .collect();
    for handle in handles {
        if let Ok((failed, sandbox, sessions)) = handle.join() {
            if failed {
                ctx.failed = true;
            }
            if ctx.sandbox.is_none() {
                ctx.sandbox = sandbox;
            }
            for (k, v) in sessions {
                ctx.session_ids.insert(k, v);
            }
        }
    }
    let _ = group;
}

fn walk_stage(ctx: &mut RunContext, stage: &Value, group: Option<&str>) {
    let root = ctx.root.clone();
    let name = stage.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    journal(root.as_path(), run_id(ctx), Some(&name), "stage_start", "info", &format!("stage '{name}' started"), &json!({ "group": group }));
    set_stage(&root, run_id(ctx), &name, "running", None, None, None, true, false);

    let isolation = stage.get("isolation").and_then(Value::as_str).unwrap_or("sandbox");
    let budget = stage.get("budget").cloned().unwrap_or(Value::Null);
    let max_turns = budget.get("maxTurns").and_then(Value::as_i64);
    let wall = budget
        .get("wallClockMs")
        .and_then(Value::as_i64)
        .map(|ms| std::time::Instant::now() + std::time::Duration::from_millis(ms as u64));
    ctx.deadline = wall;

    let steps = stage.get("steps").and_then(Value::as_array).cloned().unwrap_or_default();
    for (i, step) in steps.iter().enumerate() {
        if ctx.failed || *ctx.aborted.lock().unwrap() {
            break;
        }
        if let Some(deadline) = ctx.deadline {
            if std::time::Instant::now() > deadline {
                ctx.failed = true;
                journal(root.as_path(), run_id(ctx), Some(&name), "error", "error", "stage budget exceeded", &json!({ "budget": budget }));
                break;
            }
        }
        let kind = step.as_object().and_then(|m| m.keys().next()).cloned().unwrap_or_default();
        journal(root.as_path(), run_id(ctx), Some(&name), "step_start", "info", &format!("{kind} step {}", i + 1), &json!({ "index": i, "kind": kind }));
        let ok = run_step(ctx, stage, &name, step, isolation, max_turns, i);
        journal(
            root.as_path(),
            run_id(ctx),
            Some(&name),
            "step_end",
            if ok { "info" } else { "error" },
            &format!("{kind} step {} {}", i + 1, if ok { "succeeded" } else { "failed" }),
            &json!({ "index": i, "kind": kind, "ok": ok }),
        );
        if !ok {
            ctx.failed = true;
        }
    }

    let state = if *ctx.aborted.lock().unwrap() {
        "aborted"
    } else if ctx.failed {
        "failure"
    } else {
        "success"
    };
    set_stage(&root, run_id(ctx), &name, state, None, None, None, false, true);
    journal(
        root.as_path(),
        run_id(ctx),
        Some(&name),
        "stage_end",
        "info",
        &format!("stage '{name}' {state}"),
        &json!({ "state": state }),
    );
}

fn run_id(ctx: &RunContext) -> i64 {
    run_id_of(&ctx.run)
}

fn run_id_of(run: &Value) -> i64 {
    run.get("id").and_then(Value::as_i64).unwrap_or(0)
}

fn ensure_sandbox(ctx: &mut RunContext, _stage: &Value, name: &str) -> bool {
    if ctx.sandbox.is_some() {
        return true;
    }
    set_stage(&ctx.root, run_id(ctx), name, "provisioning", None, None, None, false, false);
    let wt = ctx
        .worktree
        .clone()
        .or_else(|| ctx.run_env.get("REPO_PATH").cloned())
        .unwrap_or_default();
    let git_dir = crate::vcs::git::git_common_dir(Path::new(&wt)).unwrap_or_default();
    match drivers::sandbox_provision(&ctx.root, &wt, &git_dir) {
        Ok(sandbox) => {
            ctx.sandbox = Some(sandbox.container_id.clone());
            set_stage(
                &ctx.root,
                run_id(ctx),
                name,
                "running",
                Some(&sandbox.container_id),
                None,
                None,
                false,
                false,
            );
            true
        }
        Err(e) => {
            journal(ctx.root.as_path(), run_id(ctx), Some(name), "error", "error", &e, &json!({}));
            false
        }
    }
}

fn run_step(
    ctx: &mut RunContext,
    stage: &Value,
    name: &str,
    step: &Value,
    isolation: &str,
    max_turns: Option<i64>,
    index: usize,
) -> bool {
    let root = ctx.root.clone();
    let run = ctx.run.clone();
    let env = step_env(&ctx.run_env, stage);
    let obj = match step.as_object() {
        Some(o) => o,
        None => return false,
    };
    let (kind, value) = match obj.iter().next() {
        Some(kv) => kv,
        None => return false,
    };

    match kind.as_str() {
        "shell" => {
            let cmd = value.as_str().unwrap_or_default().to_string();
            let cwd = ctx
                .worktree
                .clone()
                .unwrap_or_else(|| crate::api::project_root().to_string_lossy().to_string());
            let outcome = if isolation == "host" {
                drivers::shell_host(Path::new(&cwd), &cmd, &env, drivers::STEP_TIMEOUT_SECS)
            } else {
                if !ensure_sandbox(ctx, stage, name) {
                    return false;
                }
                let sandbox = ctx.sandbox.clone().unwrap_or_default();
                drivers::sandbox_exec(&root, &sandbox, &cmd, &env)
            };
            match outcome {
                Ok(out) => {
                    journal(
                        root.as_path(),
                        run_id_of(&run),
                        Some(name),
                        "note",
                        if out.ok { "info" } else { "error" },
                        &format!("$ {}\n{}", clip_cmd(&cmd), out.output),
                        &json!({ "code": out.code }),
                    );
                    out.ok
                }
                Err(e) => {
                    journal(root.as_path(), run_id_of(&run), Some(name), "error", "error", &e, &json!({}));
                    false
                }
            }
        }
        "agent" => {
            let instruction = value.as_str().unwrap_or_default().to_string();
            let agentfile = stage.get("agentfile").cloned().unwrap_or(Value::Null);
            let session_key = format!("{name}#{index}");
            let session_id = match ctx.session_ids.get(&session_key) {
                Some(id) => id.clone(),
                None => {
                    let label = super::mod_label(&run);
                    let project = super::project_for(&root, &run);
                    let workspace = ctx.worktree.clone().unwrap_or_default();
                    match drivers::agent_session_for(
                        &root,
                        run_id_of(&run),
                        name,
                        &agentfile,
                        &project,
                        &workspace,
                        &format!("{label} · {name}"),
                    ) {
                        Ok(id) => {
                            set_stage(&root, run_id_of(&run), name, "running", None, Some(&id), None, false, false);
                            ctx.session_ids.insert(session_key.clone(), id.clone());
                            id
                        }
                        Err(e) => {
                            journal(root.as_path(), run_id_of(&run), Some(name), "error", "error", &e, &json!({}));
                            return false;
                        }
                    }
                }
            };
            let model = agentfile
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let agent_name = agentfile.get("name").and_then(Value::as_str).map(str::to_string);
            let result = drivers::agent_step(
                &root,
                run_id_of(&run),
                name,
                &session_id,
                agent_name.as_deref(),
                &model,
                &instruction,
                max_turns,
                ctx.deadline,
                Arc::clone(&ctx.aborted),
            );
            if !result.ok && !*ctx.aborted.lock().unwrap() {
                journal(
                    root.as_path(),
                    run_id_of(&run),
                    Some(name),
                    "error",
                    "error",
                    "agent step stopped before completing",
                    &json!({ "turns": result.turns }),
                );
            }
            result.ok
        }
        "input" => {
            let question = value
                .get("question")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            journal(
                root.as_path(),
                run_id_of(&run),
                Some(name),
                "qa",
                "info",
                &question,
                &json!({ "question": question }),
            );
            set_stage(&root, run_id_of(&run), name, "blocked", None, None, None, false, false);
            set_run(&root, run_id_of(&run), "blocked", None, false, false);
            let park = park_for(run_id_of(&run));
            let mut state = park.0.lock().unwrap();
            loop {
                if state.stopped {
                    return false;
                }
                if let Some(answer) = state.answered.take() {
                    journal(
                        root.as_path(),
                        run_id_of(&run),
                        Some(name),
                        "qa",
                        "info",
                        &answer,
                        &json!({ "question": question, "answer": answer, "responder": "operator" }),
                    );
                    drop(state);
                    set_stage(&root, run_id_of(&run), name, "running", None, None, None, false, false);
                    set_run(&root, run_id_of(&run), "running", None, false, false);
                    return true;
                }
                let (next, _timeout) = park
                    .1
                    .wait_timeout(state, std::time::Duration::from_secs(2))
                    .unwrap();
                state = next;
                if *ctx.aborted.lock().unwrap() {
                    state.stopped = true;
                }
            }
        }
        "pr" => {
            let repo = render_tokens(value.get("repo").and_then(Value::as_str).unwrap_or_default(), &run);
            let branch = render_tokens(value.get("branch").and_then(Value::as_str).unwrap_or_default(), &run);
            let title = render_tokens(value.get("title").and_then(Value::as_str).unwrap_or_default(), &run);
            let path = value
                .get("path")
                .and_then(Value::as_str)
                .map(|p| render_tokens(p, &run));
            let number = run.get("number").and_then(Value::as_i64).unwrap_or(0);
            let job_name = run.get("jobName").and_then(Value::as_str).unwrap_or_default();
            let mut description = String::from("Job-run artifact");
            if let Some(p) = &path {
                description = format!("Job-run artifact: {p}");
            }
            description.push_str(&format!("\n\npsicom-run: {job_name}#{number}"));
            let input = json!({
                "repo": repo,
                "sourceBranch": branch,
                "title": title,
                "description": description,
                "author": format!("job {job_name}#{number}"),
            });
            let created = crate::db::with(&root, |c| crate::vcs::engine::create_pr(c, &repo, &input))
                .map_err(crate::err::ApiError::from)
                .and_then(|r| r);
            match created {
                Ok(created) => {
                    let pr = created.get("pr").cloned().unwrap_or(Value::Null);
                    let artifact = json!({
                        "repo": repo,
                        "number": pr.get("number").cloned().unwrap_or(Value::Null),
                        "title": title,
                        "state": "open",
                    });
                    crate::db::with(&root, |c| {
                        let mut artifacts: Vec<Value> = crate::db::with(&root, |c2| {
                            store::stage_rows(c2, run_id_of(&run))
                                .into_iter()
                                .find(|s| s.get("name").and_then(Value::as_str) == Some(name))
                                .and_then(|s| s.get("artifacts").and_then(Value::as_array).cloned())
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();
                        artifacts.push(artifact.clone());
                        store::set_stage_artifacts(c, run_id_of(&run), name, &json!(artifacts));
                        Ok::<(), String>(())
                    })
                    .ok();
                    journal(
                        root.as_path(),
                        run_id_of(&run),
                        Some(name),
                        "artifact",
                        "info",
                        &format!("PR opened: {}#{}", repo, pr.get("number").and_then(Value::as_i64).unwrap_or(0)),
                        &artifact,
                    );
                    super::triggers::on_pr_opened(&root, &repo, &created);
                    true
                }
                Err(e) => {
                    journal(root.as_path(), run_id_of(&run), Some(name), "error", "error", &e.message, &json!({}));
                    false
                }
            }
        }
        "parallel" => false,
        _ => false,
    }
}

fn clip_cmd(cmd: &str) -> String {
    if cmd.len() > 200 {
        format!("{}…", &cmd[..200])
    } else {
        cmd.to_string()
    }
}

fn walk_posts(ctx: &mut RunContext, definition: &Value) -> bool {
    let post = definition.get("post").cloned().unwrap_or(Value::Null);
    let outcome = !ctx.failed;
    let key = if outcome { "success" } else { "failure" };
    let mut ok = true;
    for list_key in ["always", key] {
        let steps = post
            .get(list_key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for step in steps {
            let fake_stage = json!({ "env": {}, "steps": [] });
            if !run_step(ctx, &fake_stage, &format!("post:{list_key}"), &step, "host", None, 0) {
                ok = false;
            }
        }
    }
    ok && outcome
}

pub fn answer(root: &Path, run_id: i64, answer: &str) -> Result<Value, ApiError> {
    let run = crate::db::with(root, |c| store::run_by_id(c, run_id))
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("run_not_found", format!("run {run_id} not found")))?;
    let state = run.get("state").and_then(Value::as_str).unwrap_or_default();
    if state != "blocked" {
        return Err(ApiError::conflict(
            "invalid_state",
            format!("run {run_id} is '{state}'; only blocked runs take answers"),
        ));
    }
    let park = park_for(run_id);
    let mut guard = park.0.lock().unwrap();
    guard.answered = Some(answer.to_string());
    drop(guard);
    park.1.notify_all();
    Ok(json!({ "run": run }))
}

pub fn stop(root: &Path, run_id: i64, force: bool) -> Result<Value, ApiError> {
    let run = crate::db::with(root, |c| store::run_by_id(c, run_id))
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("run_not_found", format!("run {run_id} not found")))?;
    let state = run.get("state").and_then(Value::as_str).unwrap_or_default();
    if store::active_run_states().contains(&state) {
        *abort_flag(run_id).lock().unwrap() = true;
        let park = park_for(run_id);
        park.0.lock().unwrap().stopped = true;
        park.1.notify_all();
        for stage in crate::db::with(root, |c| store::stage_rows(c, run_id)).unwrap_or_default() {
            if let Some(session) = stage.get("sessionId").and_then(Value::as_str) {
                drivers::abort_session(root, session);
            }
        }
        set_run(root, run_id, "stopping", None, false, false);
        if force {
            finalize_run(root, run_id, "aborted", Some("stopped"));
        }
    }
    let fresh = crate::db::with(root, |c| store::run_by_id(c, run_id))
        .ok()
        .flatten()
        .unwrap_or(Value::Null);
    Ok(json!({ "run": fresh }))
}

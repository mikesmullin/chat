use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

use angela::http::{Request as ARequest, ResponseSink};
use angela::session_store::NewSession;
use serde_json::{json, Value};

pub const STEP_TIMEOUT_SECS: u64 = 30 * 60;

pub struct ShellOutcome {
    pub ok: bool,
    pub code: i32,
    pub output: String,
}

pub fn shell_host(
    cwd: &Path,
    cmd: &str,
    env: &std::collections::BTreeMap<String, String>,
    timeout_secs: u64,
) -> Result<ShellOutcome, String> {
    let mut command = Command::new("sh");
    command.arg("-c").arg(cmd);
    command.current_dir(cwd);
    for (k, v) in env {
        command.env(k, v);
    }
    command.env("GIT_TERMINAL_PROMPT", "0");
    let out = crate::vcs::git::run_command(command, timeout_secs)?;
    let mut text = out.stdout_str();
    if !out.stderr.trim().is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(out.stderr.trim());
    }
    Ok(ShellOutcome {
        ok: out.status_ok,
        code: out.code,
        output: clip(&text),
    })
}

fn clip(text: &str) -> String {
    if text.len() > 200_000 {
        text[text.len() - 200_000..].to_string()
    } else {
        text.to_string()
    }
}

pub fn sandbox_image(root: &Path) -> String {
    let value = crate::db::with(root, |c| {
        c.query_row(
            "SELECT value FROM settings WHERE key = 'jobs__sandbox_image'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_default()
    })
    .unwrap_or_default();
    let value = value.trim().to_string();
    if value.is_empty() {
        "alpine-desktop:spacesd".to_string()
    } else {
        value
    }
}

pub struct StageSandbox {
    pub container_id: String,
}

pub fn sandbox_provision(
    root: &Path,
    worktree_path: &str,
    repo_git_dir: &str,
) -> Result<StageSandbox, String> {
    let image = sandbox_image(root);
    let id = crate::sbox::create(root, &image, worktree_path, repo_git_dir)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    loop {
        match crate::sbox::state(&id) {
            Some(state) if state == "running" || state == "ready" => break,
            Some(state) if state == "error" || state == "dead" => {
                let _ = crate::sbox::destroy(root, &id);
                return Err(format!("sandbox {id} failed to start ({state})"));
            }
            _ => {}
        }
        if std::time::Instant::now() > deadline {
            let _ = crate::sbox::destroy(root, &id);
            return Err(format!("sandbox {id} did not reach a running state in time"));
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    let backend = crate::sbox::SboxBackend {
        root: root.to_path_buf(),
    };
    let _ = angela::sandbox::Backend::prepare_worktree(&backend, &id);
    Ok(StageSandbox { container_id: id })
}

pub fn sandbox_exec(
    root: &Path,
    container_id: &str,
    cmd: &str,
    env: &std::collections::BTreeMap<String, String>,
) -> Result<ShellOutcome, String> {
    let backend = crate::sbox::SboxBackend {
        root: root.to_path_buf(),
    };
    let guest = vec!["sh".to_string(), "-c".to_string(), cmd.to_string()];
    let env_pairs: Vec<(String, String)> = env.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    let argv = angela::sandbox::Backend::exec_argv(
        &backend,
        container_id,
        &guest,
        &env_pairs,
    );
    let mut command = Command::new(&argv[0]);
    command.args(&argv[1..]);
    let out = crate::vcs::git::run_command(command, STEP_TIMEOUT_SECS)?;
    Ok(ShellOutcome {
        ok: out.status_ok,
        code: out.code,
        output: clip(&{
            let mut t = out.stdout_str();
            if !out.stderr.trim().is_empty() {
                t.push('\n');
                t.push_str(out.stderr.trim());
            }
            t
        }),
    })
}

pub fn sandbox_destroy(root: &Path, container_id: &str) {
    let backend = crate::sbox::SboxBackend {
        root: root.to_path_buf(),
    };
    let _ = angela::sandbox::Backend::destroy(&backend, container_id);
}

struct JournalSink {
    run_id: i64,
    stage: String,
    turns: Arc<Mutex<i64>>,
    max_turns: Option<i64>,
    deadline: Option<std::time::Instant>,
    aborted: Arc<Mutex<bool>>,
}

impl ResponseSink for JournalSink {
    fn json(&mut self, _status: u16, body: &Value) {
        if body.get("ok").and_then(Value::as_bool) == Some(false) {
            let error = body
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("request failed")
                .to_string();
            journal_error(self.run_id, Some(&self.stage), &error);
        }
    }

    fn begin_ndjson(&mut self) {}

    fn row(&mut self, value: &Value) -> bool {
        let kind = value.get("type").and_then(Value::as_str).unwrap_or_default();
        if kind == "card" {
            let card = value.get("card").cloned().unwrap_or(Value::Null);
            let card_kind = card.get("kind").and_then(Value::as_str).unwrap_or_default();
            match card_kind {
                "tool_call" | "tool_result" => {
                    let message = card
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    journal(
                        self.run_id,
                        Some(&self.stage),
                        "tool",
                        "debug",
                        &clip(&message),
                        &json!({ "card": card_kind, "name": card.get("name").and_then(Value::as_str) }),
                    );
                }
                "assistant" | "reasoning" => {
                    let message = card
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    if card_kind == "assistant" {
                        let mut turns = self.turns.lock().unwrap();
                        *turns += 1;
                        if let Some(max) = self.max_turns {
                            if *turns >= max {
                                *self.aborted.lock().unwrap() = true;
                                return false;
                            }
                        }
                    }
                    journal(
                        self.run_id,
                        Some(&self.stage),
                        "llm",
                        "debug",
                        &clip(&message),
                        &json!({ "role": card_kind }),
                    );
                }
                "error" => {
                    let message = card
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or("agent error")
                        .to_string();
                    journal_error(self.run_id, Some(&self.stage), &message);
                }
                _ => {}
            }
        }
        if let Some(deadline) = self.deadline {
            if std::time::Instant::now() > deadline {
                *self.aborted.lock().unwrap() = true;
                return false;
            }
        }
        !*self.aborted.lock().unwrap()
    }

    fn end(&mut self) {}
}

fn journal(
    run_id: i64,
    stage: Option<&str>,
    kind: &str,
    level: &str,
    message: &str,
    data: &Value,
) {
    let root = crate::api::project_root();
    let _ = crate::db::with(&root, |c| {
        super::store::journal(c, run_id, stage, kind, level, message, data);
        Ok::<(), String>(())
    });
}

fn journal_error(run_id: i64, stage: Option<&str>, message: &str) {
    journal(run_id, stage, "error", "error", message, &json!({}));
}

pub struct AgentStepResult {
    pub ok: bool,
    pub turns: i64,
}

pub fn agent_session_for(
    root: &Path,
    run_id: i64,
    stage_name: &str,
    agentfile: &Value,
    project: &str,
    workspace_root: &str,
    label: &str,
) -> Result<String, String> {
    let server = crate::api::angela(0);
    let store = &server.state.store;

    let defined: Vec<String> = store.agents().list().into_iter().map(|a| a.name).collect();
    let want = agentfile
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let agent = if defined.iter().any(|n| n == &want) {
        Some(want)
    } else {
        defined.into_iter().next()
    };
    let model = agentfile
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let system_prompt = agentfile
        .get("systemPrompt")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let tools: Vec<String> = agentfile
        .get("tools")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|t| t.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let created = store
        .create(NewSession {
            agent: agent.clone(),
            model: if model.is_empty() {
                None
            } else {
                Some(model.clone())
            },
            title: Some(label.to_string()),
            mcp: tools,
            ..Default::default()
        })
        .map_err(|e| e.to_string())?;
    let session_id = created
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if session_id.is_empty() {
        return Err("could not create the run session".to_string());
    }

    let mut patch = serde_json::Map::new();
    patch.insert("title".into(), json!(label));
    if !project.trim().is_empty() {
        patch.insert("project".into(), json!(project));
    }
    if !workspace_root.trim().is_empty() {
        patch.insert("workspaceRoot".into(), json!(workspace_root));
    }
    if !system_prompt.trim().is_empty() {
        patch.insert("soulAppendage".into(), json!(system_prompt));
    }
    store.update_meta(&session_id, patch);

    if let Some(agent_name) = agent.clone() {
        let req = ARequest {
            method: "POST".to_string(),
            path: format!("/api/sessions/{session_id}/participants"),
            query: String::new(),
            headers: Default::default(),
            body: serde_json::to_vec(&json!({
                "kind": "agent",
                "agentName": agent_name,
                "handle": agent_name,
            }))
            .unwrap_or_default(),
        };
        let mut sink = QuietSink;
        server.dispatch(&req, &mut sink);
    }

    let _ = root;
    let _ = run_id;
    let _ = stage_name;
    Ok(session_id)
}

struct QuietSink;

impl ResponseSink for QuietSink {
    fn json(&mut self, _status: u16, _body: &Value) {}
    fn begin_ndjson(&mut self) {}
    fn row(&mut self, _value: &Value) -> bool {
        true
    }
    fn end(&mut self) {}
}

pub fn agent_step(
    root: &Path,
    run_id: i64,
    stage_name: &str,
    session_id: &str,
    agent_name: Option<&str>,
    model: &str,
    instruction: &str,
    max_turns: Option<i64>,
    deadline: Option<std::time::Instant>,
    aborted: Arc<Mutex<bool>>,
) -> AgentStepResult {
    let server = crate::api::angela(0);
    let turns = Arc::new(Mutex::new(0i64));
    let mut sink = JournalSink {
        run_id,
        stage: stage_name.to_string(),
        turns: Arc::clone(&turns),
        max_turns,
        deadline,
        aborted: Arc::clone(&aborted),
    };
    let req = ARequest {
        method: "POST".to_string(),
        path: "/api/chat".to_string(),
        query: String::new(),
        headers: Default::default(),
        body: serde_json::to_vec(&json!({
            "content": instruction,
            "sessionId": session_id,
            "tabId": format!("run-{run_id}"),
            "agent": agent_name,
            "model": if model.is_empty() { Value::Null } else { json!(model) },
        }))
        .unwrap_or_default(),
    };
    server.dispatch(&req, &mut sink);
    let _ = root;
    let stopped = *aborted.lock().unwrap();
    let done_turns = *turns.lock().unwrap();
    AgentStepResult {
        ok: !stopped,
        turns: done_turns,
    }
}

pub fn abort_session(root: &Path, session_id: &str) {
    let _ = root;
    let server = crate::api::angela(0);
    let req = ARequest {
        method: "POST".to_string(),
        path: "/api/abort".to_string(),
        query: String::new(),
        headers: Default::default(),
        body: serde_json::to_vec(&json!({ "sessionId": session_id })).unwrap_or_default(),
    };
    let mut sink = QuietSink;
    server.dispatch(&req, &mut sink);
}

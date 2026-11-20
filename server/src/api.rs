//! `/api/*` — angela, running **in this process**.
//!
//! This used to forward to a separate angela daemon over HTTP, which is what
//! you do when the agent half is written in another language. It is a Rust
//! crate now, so the chat app is one server: one process to start, one set of
//! logs, no port to coordinate, and no proxy hop to buffer a stream.
//!
//! The adapter below is the whole seam. angela's router writes through its
//! `ResponseSink`; this maps that onto the dev server's `Stream`, so the same
//! routes serve both here and from the standalone `angela-serve` binary.

use std::sync::Arc;

use angela::http::{Request as ARequest, ResponseSink};
use angela::serve::{AppState, Server as AngelaServer};
use rustex1_devserver::{Request, State, Stream};
use serde_json::Value;

/// Where sessions live. `ANGELA_ROOT` overrides; otherwise the directory the
/// server was started from, which is the chat project itself.
pub fn project_root() -> std::path::PathBuf {
    match std::env::var("ANGELA_ROOT") {
        Ok(v) if !v.trim().is_empty() => std::path::PathBuf::from(v.trim()),
        _ => std::env::current_dir().unwrap_or_else(|_| ".".into()),
    }
}

/// Built once: it owns the session store and the tokio runtime that turns
/// drive agl on. Rebuilding per request would spin up a runtime per call.
pub(crate) fn angela(port: u16) -> &'static AngelaServer {
    use std::sync::OnceLock;
    static SERVER: OnceLock<AngelaServer> = OnceLock::new();
    SERVER.get_or_init(|| {
        let root = project_root();
        // The reported port is this server's: embedded angela has no port of
        // its own, and `/api/health` saying 0 would just be a lie.
        let state = AppState::new(&root, port).expect("angela state");
        eprintln!(
            "angela embedded · conversations in {}/.angela/conversations.db",
            root.display()
        );
        AngelaServer::new(Arc::new(state))
    })
}

/// Bridges angela's `ResponseSink` onto the dev server's `Stream`.
struct DevSink<'s, 'a> {
    out: &'s mut Stream<'a>,
    streaming: bool,
    ended: bool,
    chat: bool,
    session_id: String,
    prompt: String,
    baseline: Vec<Value>,
    turn_id: Option<u64>,
    disconnected: bool,
    port: u16,
}

impl<'s, 'a> ResponseSink for DevSink<'s, 'a> {
    fn json(&mut self, status: u16, body: &Value) {
        let _ = self.out.json(status, body);
        self.ended = true;
    }

    fn begin_ndjson(&mut self) {
        // The dev server adds its own chunked framing, so only the content
        // type is ours to set. `no-cache` matters: a buffering intermediary
        // is exactly what makes a token stream arrive as one lump.
        let _ = self.out.start(
            200,
            &[
                ("content-type", "application/x-ndjson"),
                ("cache-control", "no-cache"),
            ],
        );
        self.streaming = true;
        if self.chat && !self.session_id.is_empty() {
            self.turn_id = Some(crate::events::begin_turn(&self.session_id, std::mem::take(&mut self.baseline), self.prompt.clone()));
        }
    }

    fn row(&mut self, value: &Value) -> bool {
        if self.chat {
            if self.turn_id.is_none() && value.get("type").and_then(Value::as_str) == Some("session") {
                if let Some(id) = value.get("sessionId").and_then(Value::as_str) {
                    self.session_id = id.to_string();
                    let mut baseline = angela::serve::reconstruct_messages(&angela(self.port).state.store, id);
                    if baseline.last().is_some_and(|v| v.get("kind").and_then(Value::as_str) == Some("user") && v.get("text").and_then(Value::as_str) == Some(self.prompt.as_str())) { baseline.pop(); }
                    self.turn_id = Some(crate::events::begin_turn(id, baseline, self.prompt.clone()));
                }
            }
            if let Some(id) = self.turn_id { crate::events::turn_row(id, value); }
        }
        if !self.disconnected {
            let mut line = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
            line.push(b'\n');
            self.disconnected = self.out.chunk(&line).is_err();
        }
        // A browser reload closes its POST socket; the turn still belongs to
        // the server and its SSE subscribers, so do not cancel inference.
        self.chat || !self.disconnected
    }

    fn end(&mut self) {
        if self.ended {
            return;
        }
        self.ended = true;
        if let Some(id) = self.turn_id { crate::events::end_turn(id); }
        if !self.disconnected { let _ = self.out.finish(); }
    }
}

/// Current model configuration, shared by the on-demand endpoint and event feed.
pub fn models() -> Value {
    let cfg = agl::config::read_user_config();
    let default = cfg.get("default_model").and_then(|v| v.to_scalar_string()).unwrap_or_default();
    let mut models = Vec::<String>::new();
    if let Some(providers) = cfg.get("context_windows").and_then(|v| v.as_map()) {
        for (provider, entries) in providers {
        if let Some(entries) = entries.as_map() {
            for name in entries.keys() {
            let id = format!("{provider}:{name}");
            if !models.contains(&id) { models.push(id); }
            }
        }
        }
    }
    if !default.is_empty() && !models.contains(&default) { models.push(default.clone()); }
    serde_json::json!({"default_model": default, "models": models})
}

/// Session metadata plus the timestamp of the newest event on any branch.
/// `updatedAt` also changes for settings edits, so it is not an event timestamp.
pub(crate) fn sessions(port: u16) -> Value {
    let mut data = angela(port).sessions_list_for_test();
    let path = project_root().join(".angela").join("conversations.db");
    if let Ok(conn) = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
        add_latest_events(&mut data, &conn);
    }
    data
}

fn add_latest_events(data: &mut Value, db: &rusqlite::Connection) {
    let Some(rows) = data.get_mut("sessions").and_then(Value::as_array_mut) else { return; };
    let Ok(mut query) = db.prepare_cached(
        "SELECT ts FROM nodes WHERE conversation_id = ?1 ORDER BY seq DESC LIMIT 1"
    ) else { return; };
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str) else { continue; };
        let time: Option<String> = rusqlite::OptionalExtension::optional(query.query_row([id], |r| r.get(0))).ok().flatten();
        if let (Some(time), Some(row)) = (time, row.as_object_mut()) {
            row.insert("lastEventAt".into(), Value::String(time));
        }
    }
}

/// The dev server's raw `/api/` handler.
pub fn handle_api(state: &Arc<State>, req: &Request, out: &mut Stream<'_>) {
    if req.path == "/api/sessions" && req.method == "GET" {
        let _ = out.json(200, &sessions(state.config.port));
        return;
    }
    if req.path == "/api/events" && req.method == "GET" {
        crate::events::stream(out);
        return;
    }
    if req.path == "/api/models" {
        let body = models();
        crate::events::publish_models(body.clone());
        let _ = out.json(200, &body);
        return;
    }
    // ── sandboxes, repos and worktrees ─────────────────────────────────
    // The other three pickers. `/api/targets` is one GET for all of them:
    // they are read together on every redraw, and three round-trips would
    // let the menus disagree about which checkout exists.
    if req.path == "/api/targets" && req.method == "GET" {
        let root = project_root();
        let mut body = crate::hostgit::everything(&root);
        let (sandboxes, up) = match crate::sbox::list(&root) {
            Ok(v) => (v, true),
            // An empty list and a dead service look identical in a menu, so
            // the flag travels with it and the picker can say which.
            Err(_) => (serde_json::json!([]), false),
        };
        if let Value::Object(m) = &mut body {
            m.insert("sandboxes".into(), sandboxes);
            m.insert("sboxUp".into(), serde_json::json!(up));
        }
        let _ = out.json(200, &body);
        return;
    }
    if req.path.starts_with("/api/targets/") && req.method == "POST" {
        let root = project_root();
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let f = |k: &str| {
            body.get(k)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let flag = |k: &str, dflt: bool| body.get(k).and_then(Value::as_bool).unwrap_or(dflt);
        let action = req.path.trim_start_matches("/api/targets/").trim_matches('/');
        let result: Result<Value, String> = match action {
            "repo-add" => crate::hostgit::add_repo(&root, &f("path")),
            "repo-remove" => crate::hostgit::remove_repo(&root, &f("id")),
            "branch-add" => {
                crate::hostgit::add_branch(&root, &f("repoId"), &f("name"), &f("from"))
            }
            "worktree-add" => {
                crate::hostgit::add_worktree(&root, &f("repoId"), &f("branch"), flag("create", false))
            }
            "worktree-remove" => crate::hostgit::remove_worktree(&root, &f("id")),
            "sandbox-add" => crate::sbox::create(
                &root,
                &f("kind"),
                &f("worktreeId"),
                &f("branchId"),
                flag("network", true),
                &f("name"),
            ),
            // Rename only the label (chat-owned); sbox-b's id never changes.
            "sandbox-rename" => {
                crate::hostgit::set_sandbox_name(
                    &root,
                    &f("id"),
                    &f("name"),
                )
                    .map(|_| serde_json::json!({}))
            }
            "sandbox-remove" => crate::sbox::destroy(&root, &f("id")),
            other => Err(format!("unknown target action: {other}")),
        };
        match result {
            // Same contract as `/api/projects`: every write answers with the
            // fresh listing, so the caller never has to ask twice.
            Ok(mut v) => {
                if let Value::Object(map) = &mut v {
                    map.insert("ok".into(), serde_json::json!(true));
                    if let Value::Object(all) = crate::hostgit::everything(&root) {
                        for (k, val) in all {
                            map.insert(k, val);
                        }
                    }
                    if let Ok(list) = crate::sbox::list(&root) {
                        map.insert("sandboxes".into(), list);
                    }
                }
                let _ = out.json(200, &v);
            }
            Err(e) => {
                let _ = out.json(400, &serde_json::json!({"ok": false, "error": e}));
            }
        }
        return;
    }

    // ── projects ───────────────────────────────────────────────────────
    // The folders conversations are filed under. Served here rather than
    // from angela's router because they are a chat-app concept: angela
    // stores the label and owns the operations, this decides what the UI
    // may ask for.
    if req.path.starts_with("/api/projects") {
        let root = project_root();
        let Ok(store) = angela::conv_store::ConvStore::new(root) else {
            let _ = out.json(500, &serde_json::json!({"error": "no conversation store"}));
            return;
        };
        if req.method == "GET" {
            let _ = out.json(200, &store.projects_detail());
            return;
        }
        if req.method != "POST" {
            let _ = out.json(405, &serde_json::json!({"error": "use GET or POST"}));
            return;
        }
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let field = |k: &str| {
            body.get(k)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        // The sub-path is the verb. `assign` is the one the sidebar's
        // drag-and-drop uses; the other three come from the inspector form.
        let action = req
            .path
            .trim_start_matches("/api/projects")
            .trim_matches('/')
            .to_string();
        let result: Result<Value, String> = match action.as_str() {
            "" | "create" => store.create_project(&field("name")).map(|n| {
                // Same contract as the target actions: the response names
                // the row the picker should land on.
                serde_json::json!({"name": n, "select": n, "selectKind": "project"})
            }),
            "rename" => store
                .rename_project(&field("from"), &field("to"))
                .map(|n| serde_json::json!({"name": n})),
            "delete" => store
                .delete_project(&field("name"))
                .map(|n| serde_json::json!({"unfiled": n})),
            "assign" => store
                .assign_project(&field("sessionId"), &field("project"))
                .map(|n| serde_json::json!({"project": n})),
            other => Err(format!("unknown project action: {other}")),
        };
        match result {
            // Every write answers with the new listing as well as the
            // result: the sidebar and the picker both redraw from it, and a
            // second round-trip to find out what just happened is a window
            // in which they disagree.
            Ok(mut v) => {
                if let Value::Object(map) = &mut v {
                    map.insert("ok".into(), serde_json::json!(true));
                    if let Some(Value::Array(list)) = store.projects_detail().get("projects") {
                        map.insert("projects".into(), Value::Array(list.clone()));
                    }
                }
                let _ = out.json(200, &v);
            }
            Err(e) => {
                let _ = out.json(400, &serde_json::json!({"ok": false, "error": e}));
            }
        }
        return;
    }

    if req.path == "/api/composer-settings" && req.method == "POST" {
        let payload: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let id = payload.get("sessionId").and_then(Value::as_str).unwrap_or("");
        let root = project_root();
        let result = angela::conv_store::ConvStore::new(root).ok().and_then(|store| {
            let old = store.load(id)?;
            let mut patch = serde_json::Map::new();
            for key in [
                "model",
                "allowlist",
                "allowlistMode",
                "allowlistSource",
                "project",
                // Where this conversation's work happens. `sandboxId` is the
                // one the turn loop reads to decide whose filesystem the
                // file and shell tools mean.
                "sandboxId",
                "worktreeId",
                "branchId",
            ] {
                if let Some(value) = payload.get(key) { patch.insert(key.into(), value.clone()); }
            }
            if let Some(reasoning) = payload.get("reasoningEffort") {
                let mut config = old.get("config").and_then(Value::as_object).cloned().unwrap_or_default();
                config.insert("reasoningEffort".into(), reasoning.clone());
                patch.insert("config".into(), Value::Object(config));
            }
            store.update_meta(id, patch)
        });
        let (status, body) = match result {
            Some(session) => (200, serde_json::json!({"ok": true, "session": session})),
            None => (404, serde_json::json!({"ok": false, "error": "session not found"})),
        };
        let _ = out.json(status, &body);
        return;
    }
    let areq = ARequest {
        method: req.method.clone(),
        path: req.path.clone(),
        query: req.query.clone(),
        headers: req
            .headers
            .iter()
            .map(|(k, v)| (k.to_ascii_lowercase(), v.clone()))
            .collect(),
        body: req.body.clone(),
    };
    let chat = req.method == "POST" && req.path == "/api/chat";
    let body: Value = if chat { serde_json::from_slice(&req.body).unwrap_or(Value::Null) } else { Value::Null };
    let session_id = body.get("sessionId").and_then(Value::as_str).unwrap_or_default().to_string();
    let prompt = body.get("content").and_then(Value::as_str).unwrap_or_default().to_string();
    let baseline = if chat && !session_id.is_empty() {
        angela::serve::reconstruct_messages(&angela(state.config.port).state.store, &session_id)
    } else { Vec::new() };
    let mut sink = DevSink {
        out, streaming: false, ended: false, chat, session_id, prompt, baseline,
        turn_id: None, disconnected: false, port: state.config.port,
    };
    angela(state.config.port).dispatch(&areq, &mut sink);
    sink.end();
}

#[cfg(test)]
mod latest_event_tests {
    use super::*;

    #[test]
    fn latest_event_is_taken_from_the_latest_sequence_not_metadata() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE nodes (conversation_id TEXT, seq INTEGER, ts TEXT);
            INSERT INTO nodes VALUES ('a', 1, '2020-01-01T00:00:00Z');
            INSERT INTO nodes VALUES ('a', 2, '2020-02-01T00:00:00Z');
            INSERT INTO nodes VALUES ('b', 1, '2020-03-01T00:00:00Z');").unwrap();
        let mut sessions = serde_json::json!({"sessions": [
            {"id":"a", "updatedAt":"2025-01-01T00:00:00Z"},
            {"id":"b"}, {"id":"empty"}
        ]});
        add_latest_events(&mut sessions, &db);
        assert_eq!(sessions["sessions"][0]["lastEventAt"], "2020-02-01T00:00:00Z");
        assert_eq!(sessions["sessions"][1]["lastEventAt"], "2020-03-01T00:00:00Z");
        assert!(sessions["sessions"][2].get("lastEventAt").is_none());
    }
}

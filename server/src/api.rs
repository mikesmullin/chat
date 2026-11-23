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
        let mut state = AppState::new(&root, port).expect("angela state");
        // The chat-control tools (`chat__sessions_prompt`) enqueue turns
        // through the host, so rows reach the live event feed exactly like
        // a typed prompt. Wired here because the bridging (the event feed)
        // is the host's, not angela's.
        state.prompt_sender = Some(Arc::new(|session: &str, content: String, agent: String, model: String| {
            crate::automations::submit_prompt(session, content, agent, model);
        }));
        // A roster change hits the event feed the moment it happens: the
        // person just invited sees the session appear in their sidebar in
        // real time (PLAN: an invite is an SSE event, not a 4s-later poll).
        state.seat_sender = Some(Arc::new(|_session: &str| {
            crate::events::bump_sessions();
        }));
        // Agent-owned sandboxes (PLAN55): angela owns the policy (one
        // container per session → agent, lazily launched), this module owns
        // the seam (the sbox-b client + worktree bookkeeping).
        angela::sandbox::set_backend(Box::new(crate::sbox::SboxBackend {
            root: root.clone(),
        }));
        eprintln!(
            "angela embedded · conversations in {}/.angela/conversations.db",
            root.display()
        );
        AngelaServer::new(Arc::new(state))
    })
}

/// Add chat-owned status events to angela's reconstructed transcript in
/// branch order. They are not model messages and are not sent as prompts.
fn chat_messages(store: &angela::conv_store::ConvStore, id: &str) -> Vec<Value> {
    let mut messages = angela::serve::reconstruct_messages(store, id);
    let path = store.path(id);
    let order: std::collections::HashMap<_, _> = path.iter().enumerate().map(|(i, node)| (node.id.as_str(), i)).collect();
    for (index, node) in path.iter().enumerate() {
        if node.kind != "chat_status" { continue; }
        let Some(text) = node.payload.get("text").and_then(Value::as_str) else { continue; };
        let row = serde_json::json!({"id": node.id, "eventId": node.id, "kind": "system", "text": text, "ts": node.ts, "commandNotice": true, "visible": node.visible});
        let position = messages.iter().position(|row| row.get("eventId").and_then(Value::as_str).and_then(|id| order.get(id)).is_some_and(|next| *next > index)).unwrap_or(messages.len());
        messages.insert(position, row);
    }
    for message in &mut messages { crate::assistant_text::normalize_assistant_messages(message); }
    messages
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
    assistant_filter: crate::assistant_text::StreamFilter,
    /// When set, a session this request creates is owned by this user
    /// (PLAN44: ownership is claimed at creation, never at first touch —
    /// a stranger's first message must not make them the owner).
    claim_owner: Option<i64>,
    /// The acting user, for listings the response must filter (PLAN44 C4):
    /// `None` is the dev/admin zone — unfiltered.
    viewer: Option<i64>,
}

impl DevSink<'_, '_> {
    fn publish_row(&mut self, value: &Value) {
        if self.chat {
            if let Some(id) = self.turn_id { crate::events::turn_row(id, value); }
        }
        // A new session announces itself as `{"type":"session","sessionId"}`.
        // Claim it for the creator exactly once.
        if let (Some(owner), Some(sid)) = (
            self.claim_owner,
            value.get("sessionId").and_then(Value::as_str),
        ) {
            if value.get("type").and_then(Value::as_str) == Some("session") && !sid.is_empty() {
                let _ = angela(self.port)
                    .state
                    .store
                    .identity()
                    .ensure_resource("session", sid, owner);
                self.claim_owner = None;
            }
        }
        if !self.disconnected {
            let mut line = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
            line.push(b'\n');
            self.disconnected = self.out.chunk(&line).is_err();
        }
    }
}

impl<'s, 'a> ResponseSink for DevSink<'s, 'a> {
    fn json(&mut self, status: u16, body: &Value) {
        let mut body = body.clone();
        // A JSON answer that names a fresh session id claims it too
        // (`/api/session/new`).
        if let (Some(owner), Some(sid)) = (
            self.claim_owner,
            body.get("id").and_then(Value::as_str),
        ) {
            let _ = angela(self.port)
                .state
                .store
                .identity()
                .ensure_resource("session", sid, owner);
            self.claim_owner = None;
        }
        if body.get("messages").and_then(Value::as_array).is_some() {
            if let Some(id) = body.pointer("/session/id").and_then(Value::as_str).map(str::to_owned) {
                body["messages"] = Value::Array(chat_messages(&angela(self.port).state.store, &id));
            }
        }
        // Agent listings ride this path (angela's /api/agents): only agents
        // shared with this viewer survive (PLAN44 C4).
        if let Some(uid) = self.viewer {
            if let Some(rows) = body.get_mut("agents").and_then(Value::as_array_mut) {
                let names: Vec<String> = rows
                    .iter()
                    .filter_map(|a| a.get("name").and_then(Value::as_str).map(str::to_string))
                    .collect();
                let keep: std::collections::HashSet<String> = angela(self.port)
                    .state
                    .store
                    .identity()
                    .readable(uid, "agent", &names)
                    .into_iter()
                    .collect();
                rows.retain(|a| {
                    a.get("name")
                        .and_then(Value::as_str)
                        .is_some_and(|n| keep.contains(n))
                });
            }
        }
        crate::assistant_text::normalize_assistant_messages(&mut body);
        let _ = self.out.json(status, &body);
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
                    let mut baseline = chat_messages(&angela(self.port).state.store, id);
                    if baseline.last().is_some_and(|v| v.get("kind").and_then(Value::as_str) == Some("user") && v.get("text").and_then(Value::as_str) == Some(self.prompt.as_str())) { baseline.pop(); }
                    self.turn_id = Some(crate::events::begin_turn(id, baseline, self.prompt.clone()));
                }
            }
        }
        let rows = if self.chat { self.assistant_filter.process(value) } else { vec![value.clone()] };
        for row in rows { self.publish_row(&row); }
        // A browser reload closes its POST socket; the turn still belongs to
        // the server and its SSE subscribers, so do not cancel inference.
        self.chat || !self.disconnected
    }

    fn end(&mut self) {
        if self.ended {
            return;
        }
        self.ended = true;
        if let Some(row) = self.assistant_filter.finish() { self.publish_row(&row); }
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

/// Session metadata plus the newest event's timestamp on any branch, and the
/// last generation's token counts. `updatedAt` also changes for settings
/// edits, so it is not an event timestamp; `totalTokens` is what the context
/// gauges (the composer's pie, the state symbol's ring) divide by the row's
/// `contextWindow`.
pub(crate) fn sessions(port: u16) -> Value {
    let mut data = angela(port).sessions_list_for_test();
    let path = project_root().join(".angela").join("conversations.db");
    if let Ok(conn) = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
        add_latest_events(&mut data, &conn);
        add_token_counts(&mut data, &conn);
        add_seat_counts(&mut data, &conn);
    }
    data
}

/// The row contract's `seats`: how many active seats the conversation has —
/// the roster fingerprint that makes an invite change this snapshot at all.
/// Without it, seating someone rewrites nothing here and no SSE fires: the
/// session would reach their sidebar only after an unrelated row edit.
fn add_seat_counts(data: &mut Value, db: &rusqlite::Connection) {
    let Some(rows) = data.get_mut("sessions").and_then(Value::as_array_mut) else { return; };
    let Ok(mut query) = db.prepare_cached(
        "SELECT COUNT(*) FROM participants WHERE conversation_id = ?1 AND active = 1"
    ) else { return; };
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str) else { continue; };
        let n: i64 = query.query_row([id], |r| r.get(0)).unwrap_or(0);
        if let Some(row) = row.as_object_mut() {
            row.insert("seats".into(), Value::from(n));
        }
    }
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

/// The row contract's `totalTokens`: the newest `gen_info` node's generation
/// total — the same number the `usage` rows stream and the full session meta
/// carries as `lastTotalTokens`. Derived here for the same reason as
/// `lastEventAt` above: the sessions list is a projection for the sidebar,
/// and this is what its state symbols fill their ring with.
fn add_token_counts(data: &mut Value, db: &rusqlite::Connection) {
    let Some(rows) = data.get_mut("sessions").and_then(Value::as_array_mut) else { return; };
    let Ok(mut query) = db.prepare_cached(
        "SELECT gen_total_tokens FROM nodes WHERE conversation_id = ?1 \
         AND kind = 'gen_info' AND gen_total_tokens IS NOT NULL \
         ORDER BY seq DESC LIMIT 1"
    ) else { return; };
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str) else { continue; };
        let total: Option<i64> = rusqlite::OptionalExtension::optional(query.query_row([id], |r| r.get(0))).ok().flatten();
        if let (Some(total), Some(row)) = (total, row.as_object_mut()) {
            row.insert("totalTokens".into(), Value::from(total));
        }
    }
}

/// The existence check and deletion are one SQLite statement, so events on
/// any branch protect the conversation even if the client has stale data.
fn delete_empty_session(db: &rusqlite::Connection, id: &str) -> rusqlite::Result<bool> {
    db.execute_batch("PRAGMA foreign_keys = ON")?;
    Ok(db.execute(
        "DELETE FROM conversation WHERE id = ?1 AND NOT EXISTS \
         (SELECT 1 FROM nodes WHERE conversation_id = ?1 AND NOT \
          (kind = 'session' AND COALESCE(json_extract(payload, '$.kind'), '') = 'created'))", [id]
    )? == 1)
}

/// The acting user's id, or 1 for the dev zone — the gate only lets
/// loopback through without a cookie, and loopback is the developer on
/// this machine (D12), acting with the admin's authority.
fn actor_id(req: &Request) -> i64 {
    crate::auth::current_user(req).map(|u| u.id).unwrap_or(1)
}

fn is_admin(req: &Request) -> bool {
    crate::auth::current_user(req).map(|u| u.is_admin()).unwrap_or(true)
}

/// PLAN44 C4 + the roster: a session is the viewer's to see or speak in when
/// the ACL says so **or when they are seated in it** — being invited into a
/// room (a named `user` seat, from a `@mention` or `/invite`) is what makes
/// it appear in their Projects tree and lets them talk in it. The dev zone
/// and the admin see everything.
pub(crate) fn session_open_to(uid: Option<i64>, sid: &str, privilege: &str) -> bool {
    let Some(uid) = uid else {
        return true;
    };
    let server = angela(0);
    let ident = server.state.store.identity();
    if ident.user_by_id(uid).map(|u| u.is_admin()).unwrap_or(false) {
        return true;
    }
    if ident.can(uid, "session", sid, privilege) {
        return true;
    }
    ident
        .user_by_id(uid)
        .map(|u| server.state.store.sessions_seating(&u.name).iter().any(|s| s == sid))
        .unwrap_or(false)
}

/// [`session_open_to`] for a whole listing: which of these sessions may
/// this viewer see?
pub(crate) fn visible_session_ids(uid: Option<i64>, names: &[String]) -> Vec<String> {
    let Some(uid) = uid else {
        return names.to_vec();
    };
    let server = angela(0);
    let ident = server.state.store.identity();
    if ident.user_by_id(uid).map(|u| u.is_admin()).unwrap_or(false) {
        return names.to_vec();
    }
    let allowed: std::collections::HashSet<String> =
        ident.readable(uid, "session", names).into_iter().collect();
    let seated: std::collections::HashSet<String> = ident
        .user_by_id(uid)
        .map(|u| server.state.store.sessions_seating(&u.name).into_iter().collect())
        .unwrap_or_default();
    names
        .iter()
        .filter(|n| allowed.contains(*n) || seated.contains(*n))
        .cloned()
        .collect()
}

/// PLAN55 listing gate: a sandbox holds its agent's work, so a row is
/// visible — and its container destroyable — only to the admin and to users
/// who may **edit** the agent inhabiting it. `None` viewer is the dev zone,
/// which acts as the admin. An uninhabited row has no agent to authorize
/// against: admin-only (fail closed).
fn sandbox_visible_with(
    ident: &angela::identity::Identity<'_>,
    viewer: Option<i64>,
    agent: &str,
) -> bool {
    let Some(uid) = viewer else {
        return true;
    };
    if ident.user_by_id(uid).map(|u| u.is_admin()).unwrap_or(false) {
        return true;
    }
    !agent.is_empty() && ident.can(uid, "agent", agent, "edit")
}

/// [`sandbox_visible_with`] for the one viewer a request or a feed stream
/// carries.
pub(crate) fn sandbox_visible(viewer: Option<i64>, agent: &str) -> bool {
    let ident = angela(0).state.store.identity();
    sandbox_visible_with(&ident, viewer, agent)
}

/// The sandboxes rows one viewer may see (the listing gate applied).
fn visible_sandboxes(viewer: Option<i64>, rows: Value) -> Value {
    let ident = angela(0).state.store.identity();
    let Value::Array(list) = rows else {
        return rows;
    };
    Value::Array(
        list.into_iter()
            .filter(|r| {
                sandbox_visible_with(&ident, viewer, r.get("agent").and_then(Value::as_str).unwrap_or_default())
            })
            .collect(),
    )
}

/// Layer 1 in one call: the dev zone and the admin hold everything.
fn actor_may(req: &Request, cap: &str) -> bool {
    match crate::auth::current_user(req) {
        Some(u) => angela(0).state.store.identity().has_capability(u.id, cap),
        None => true,
    }
}

/// The capability a mutating request needs (PLAN44 catalog, Layer 1).
/// Reads are per-resource (Layer 2) and checked where the resource is named.
fn required_capability(req: &Request) -> Option<&'static str> {
    if req.method == "GET" || req.method == "HEAD" {
        return None;
    }
    let p = req.path.as_str();
    if p == "/api/session/new" {
        return Some("session.create");
    }
    if p == "/api/agents" || p.starts_with("/api/agents/") {
        return Some("agent.create");
    }
    if p.starts_with("/api/automations/") {
        return Some("automation.create");
    }
    if p.starts_with("/api/sessions/") && p.ends_with("/participants") {
        return Some("session.invite");
    }
    if matches!(
        p,
        "/api/projects/project-add"
            | "/api/projects/repo-add"
            | "/api/projects/branch-add"
            | "/api/projects/worktree-add"
    ) {
        return Some("project.create");
    }
    None
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn query_str(req: &Request, key: &str) -> String {
    let prefix = format!("{key}=");
    for pair in req.query.split('&') {
        if let Some(v) = pair.strip_prefix(&prefix) {
            return percent_decode(v);
        }
    }
    String::new()
}

fn query_int(req: &Request, key: &str) -> Option<i64> {
    query_str(req, key).parse().ok()
}

/// The share dialog's data for one resource: `{grants, ownerId}`.
fn acl_rows(
    ident: &angela::identity::Identity<'_>,
    kind: &str,
    name: &str,
) -> (Vec<Value>, Option<i64>) {
    let (owner, rows) = ident.grants_on(kind, name);
    (rows, owner)
}

/// The dev server's raw `/api/` handler.
pub fn handle_api(state: &Arc<State>, req: &Request, out: &mut Stream<'_>) {
    // ---- identity, admin and ACL (PLAN44) -------------------------------
    //
    // The gate has already turned anonymous non-loopback traffic away; what
    // arrives here is either a logged-in user or the dev zone (loopback, no
    // cookie — the developer on this machine, D12). The dev zone passes
    // every check below as the admin would.
    if req.path == "/api/profile" && req.method == "POST" {
        // Per-user profile (PLAN44): the location is yours to set. The
        // name is *not* here — it is fixed at login, and only re-creating
        // the account changes it.
        let Some(u) = crate::auth::current_user(req) else {
            let _ = out.json(401, &serde_json::json!({"ok": false, "error": "login required"}));
            return;
        };
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let location = body.get("location").and_then(Value::as_str).unwrap_or("");
        let ident = angela(state.config.port).state.store.identity();
        let r = ident.set_location(u.id, location);
        let _ = out.json(
            if r.is_ok() { 200 } else { 400 },
            &serde_json::json!({"ok": r.is_ok(), "error": r.err().unwrap_or_default()}),
        );
        return;
    }
    if req.path == "/api/me" && req.method == "GET" {
        let ident = angela(state.config.port).state.store.identity();
        let body = match crate::auth::current_session(req) {
            Some((u, dev)) => {
                // A masked session keeps its switcher (D4): the chip must
                // know it is wearing one, and who stands behind it.
                let real = crate::auth::real_actor(&ident, &u, &dev);
                // The *effective* list: the admin holds everything
                // implicitly, so the UI can gate on membership alone.
                let caps: Vec<String> = if u.is_admin() {
                    angela::identity::CAPABILITIES.iter().map(|s| s.to_string()).collect()
                } else {
                    ident.capabilities(u.id)
                };
                serde_json::json!({
                    "user": { "id": u.id, "name": u.name, "displayName": u.display_name,
                              "location": u.location,
                              "admin": u.is_admin(), "disabled": u.disabled },
                    "capabilities": caps,
                    "masquerade": crate::auth::is_masked(&u, &dev),
                    "realName": real.name,
                    // Every user sees the perimeter's state (PLAN44 D12):
                    // the page glows while the door is open.
                    "shields": if crate::auth::instance().shields_down() { "down" } else { "up" },
                })
            }
            None => serde_json::json!({ "user": null }),
        };
        let _ = out.json(200, &body);
        return;
    }
    if req.path == "/api/admin/audit" && req.method == "GET" {
        if !is_admin(req) {
            let _ = out.json(403, &serde_json::json!({"ok": false, "error": "admin only"}));
            return;
        }
        let n = query_int(req, "n").filter(|n| *n > 0).unwrap_or(100).min(1000) as usize;
        let entries = angela(state.config.port).state.store.identity().audit_tail(n);
        let _ = out.json(200, &serde_json::json!({"ok": true, "audit": entries}));
        return;
    }
    if req.path == "/api/admin/perimeter" && req.method == "GET" {
        if !is_admin(req) {
            let _ = out.json(403, &serde_json::json!({"ok": false, "error": "admin only"}));
            return;
        }
        let p = crate::auth::instance();
        let _ = out.json(200, &serde_json::json!({
            "ok": true,
            "shields": if p.shields_down() { "down" } else { "up" },
            "bans": p.bans(),
            "kernelFirewall": p.firewall_ok(),
        }));
        return;
    }
    if req.path == "/api/admin/shields" && req.method == "POST" {
        if !is_admin(req) {
            let _ = out.json(403, &serde_json::json!({"ok": false, "error": "admin only"}));
            return;
        }
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let down = body.get("down").and_then(Value::as_bool).unwrap_or(false);
        crate::auth::instance().set_shields_down(down);
        let _ = out.json(200, &serde_json::json!({
            "ok": true,
            "shields": if down { "down" } else { "up" },
        }));
        return;
    }
    if req.path == "/api/admin/bans" {
        if !is_admin(req) {
            let _ = out.json(403, &serde_json::json!({"ok": false, "error": "admin only"}));
            return;
        }
        let p = crate::auth::instance();
        if req.method == "GET" {
            let _ = out.json(200, &serde_json::json!({"ok": true, "bans": p.bans()}));
        } else {
            let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
            let ip = body.get("ip").and_then(Value::as_str).unwrap_or("");
            p.unban(ip);
            let _ = out.json(200, &serde_json::json!({"ok": true, "bans": p.bans()}));
        }
        return;
    }
    if req.path == "/api/users" && req.method == "GET" {
        let ident = angela(state.config.port).state.store.identity();
        // Names and ids are public inside the house (the share dialog needs
        // them); capability lists are nobody's business but your own — and
        // the admin's.
        let admin_view = is_admin(req);
        let me = actor_id(req);
        let users: Vec<Value> = ident
            .list_users()
            .into_iter()
            .map(|u| {
                let mut row = serde_json::json!({
                    "id": u.id, "name": u.name, "displayName": u.display_name,
                    "admin": u.is_admin(), "disabled": u.disabled,
                });
                if admin_view || u.id == me {
                    row["capabilities"] = serde_json::json!(ident.capabilities(u.id));
                }
                row
            })
            .collect();
        let _ = out.json(200, &serde_json::json!({"ok": true, "users": users}));
        return;
    }
    if req.path == "/api/users/capabilities" && req.method == "POST" {
        if !is_admin(req) {
            let _ = out.json(403, &serde_json::json!({"ok": false, "error": "admin only"}));
            return;
        }
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let ident = angela(state.config.port).state.store.identity();
        let user_id = body.get("userId").and_then(Value::as_i64).unwrap_or(0);
        let cap = body.get("capability").and_then(Value::as_str).unwrap_or("");
        let grant = body.get("grant").and_then(Value::as_bool).unwrap_or(true);
        let r = if grant {
            ident.grant_capability(user_id, cap).map(|_| true)
        } else {
            ident.revoke_capability(user_id, cap);
            Ok(true)
        };
        let _ = out.json(
            if r.is_ok() { 200 } else { 400 },
            &serde_json::json!({"ok": r.is_ok(), "error": r.err().unwrap_or_default()}),
        );
        return;
    }
    if req.path == "/api/users/allowlist" && req.method == "POST" {
        if !is_admin(req) {
            let _ = out.json(403, &serde_json::json!({"ok": false, "error": "admin only"}));
            return;
        }
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let ident = angela(state.config.port).state.store.identity();
        let user_id = body.get("userId").and_then(Value::as_i64).unwrap_or(0);
        let kind = body.get("kind").and_then(Value::as_str).unwrap_or("");
        let patterns: Vec<String> = body
            .get("patterns")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let r = ident.set_allowlist(user_id, kind, &patterns);
        let _ = out.json(
            if r.is_ok() { 200 } else { 400 },
            &serde_json::json!({"ok": r.is_ok(), "error": r.err().unwrap_or_default()}),
        );
        return;
    }
    if req.path == "/api/devices" && req.method == "GET" {
        let ident = angela(state.config.port).state.store.identity();
        // A user lists their own devices. Peeking at anyone else's is
        // admin work — and "anyone else" includes the face an admin
        // masquerades as: the acting user decides, always.
        let target = if is_admin(req) {
            query_int(req, "userId").or_else(|| crate::auth::current_user(req).map(|u| u.id))
        } else {
            crate::auth::current_user(req).map(|u| u.id)
        };
        let devices: Vec<Value> = target
            .map(|uid| {
                ident
                    .list_devices(uid)
                    .into_iter()
                    // Masquerade acts are not the user's recognized
                    // devices: they are the admin's acts, recorded in the
                    // audit log, and nothing here can delete or block
                    // them — so they do not appear here. (The `ip` check
                    // retires rows from before the provenance mark.)
                    .filter(|d| {
                        !d.fingerprint.starts_with("masquerade:by:") && d.ip != "masquerade"
                    })
                    .map(|d| serde_json::json!({
                        "id": d.id, "fingerprint": d.fingerprint,
                        "userAgent": d.user_agent, "ip": d.ip, "createdAt": d.created_at,
                        "lastSeenAt": d.last_seen_at, "revoked": d.revoked,
                    }))
                    .collect()
            })
            .unwrap_or_default();
        let _ = out.json(200, &serde_json::json!({"ok": true, "devices": devices}));
        return;
    }
    if req.path == "/api/devices/revoke" && req.method == "POST" {
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let ident = angela(state.config.port).state.store.identity();
        let device_id = body.get("deviceId").and_then(Value::as_i64).unwrap_or(0);
        // Scope: a user revokes their own devices (user_id filter), the
        // admin any (0 = no filter).
        let scope = if is_admin(req) { 0 } else {
            crate::auth::current_user(req).map(|u| u.id).unwrap_or(-1)
        };
        let r = ident.revoke_device(scope, device_id);
        let _ = out.json(
            if r.is_ok() { 200 } else { 400 },
            &serde_json::json!({"ok": r.is_ok(), "error": r.err().unwrap_or_default()}),
        );
        return;
    }
    if req.path == "/api/acl" && req.method == "GET" {
        // The share dialog's data: the grants on one resource. Owners, the
        // admin, and anyone with `grant` may look.
        let ident = angela(state.config.port).state.store.identity();
        let (kind, name) = (query_str(req, "kind"), query_str(req, "name"));
        let actor_id = actor_id(req);
        if !ident.can(actor_id, &kind, &name, "grant") && !ident.can(actor_id, &kind, &name, "read") {
            let _ = out.json(403, &serde_json::json!({"ok": false, "error": "no such resource"}));
            return;
        }
        let (grants, owner) = acl_rows(&ident, &kind, &name);
        let _ = out.json(200, &serde_json::json!({
            "ok": true, "kind": kind, "name": name, "ownerId": owner, "grants": grants,
        }));
        return;
    }
    if req.path == "/api/acl/grant" && req.method == "POST" {
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let ident = angela(state.config.port).state.store.identity();
        let s = |k: &str| body.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        // The grantee may be an id or a login name — the share dialog
        // speaks names, the table stores ids.
        let grantee = body.get("grantee").and_then(Value::as_i64).unwrap_or(0);
        let grantee = if grantee > 0 {
            grantee
        } else {
            ident.user_by_name(&s("grantee")).map(|u| u.id).unwrap_or(0)
        };
        let with_grant = body.get("withGrant").and_then(Value::as_bool).unwrap_or(false);
        // A resource older than the ACL has no row yet. The admin sharing
        // it is what materialises one — owned by the admin, not by the
        // first person to touch it.
        if ident.resource(&s("kind"), &s("name")).is_none() && actor_id(req) == 1 {
            let _ = ident.ensure_resource(&s("kind"), &s("name"), 1);
        }
        let r = ident.grant(grantee, &s("kind"), &s("name"), &s("privilege"), actor_id(req), with_grant);
        if r.is_ok() && s("kind") == "session" && matches!(s("privilege").as_str(), "read" | "post" | "seat") {
            // Sharing a conversation seats the person in it (PLAN44 A4):
            // the roster says who is in the room, the grant is what lets
            // them in — one action, one truth.
            let name = s("grantee");
            let handle = name.to_lowercase().replace(' ', "-");
            let _ = angela(state.config.port).state.store.add_participant(
                &s("name"),
                "user",
                &handle,
                Some(&name),
                None,
                None,
                "",
                None,
                None,
            );
        }
        let _ = out.json(
            if r.is_ok() { 200 } else { 403 },
            &serde_json::json!({"ok": r.is_ok(), "error": r.err().unwrap_or_default()}),
        );
        return;
    }
    if req.path == "/api/acl/revoke" && req.method == "POST" {
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let ident = angela(state.config.port).state.store.identity();
        let s = |k: &str| body.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let grantee = body.get("grantee").and_then(Value::as_i64).unwrap_or(0);
        let grantee = if grantee > 0 {
            grantee
        } else {
            ident.user_by_name(&s("grantee")).map(|u| u.id).unwrap_or(0)
        };
        let r = ident.revoke_grant(actor_id(req), &s("kind"), &s("name"), grantee, &s("privilege"));
        let _ = out.json(
            if r.is_ok() { 200 } else { 403 },
            &serde_json::json!({"ok": r.is_ok(), "error": r.err().unwrap_or_default()}),
        );
        return;
    }

    // ---- media (PLAN44 A5): text and images only -----------------------
    if req.path == "/api/upload" && req.method == "POST" {
        // Raw bytes, name in the query, kept under `.angela/media`. The
        // extension allowlist is the whole content policy.
        let name = query_str(req, "name");
        let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
        if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp") || req.body.is_empty() {
            let _ = out.json(
                400,
                &serde_json::json!({"ok": false, "error": "images only (png/jpg/gif/webp)"}),
            );
            return;
        }
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let file = format!("{nanos:x}.{ext}");
        let dir = project_root().join(".angela").join("media");
        let _ = std::fs::create_dir_all(&dir);
        match std::fs::write(dir.join(&file), &req.body) {
            Ok(()) => {
                let _ = out.json(200, &serde_json::json!({"ok": true, "url": format!("/media/{file}")}));
            }
            Err(e) => {
                let _ = out.json(500, &serde_json::json!({"ok": false, "error": e.to_string()}));
            }
        }
        return;
    }
    if req.method == "GET" && req.path.starts_with("/media/") {
        let f = req.path.trim_start_matches("/media/");
        let path = project_root().join(".angela").join("media").join(f);
        // One directory, no separators: the name is ours, not the caller's.
        let safe = !f.is_empty() && !f.contains('/') && !f.contains('\\') && !f.contains("..");
        let bytes = if safe { std::fs::read(&path).ok() } else { None };
        match bytes {
            Some(data) => {
                let ctype = match f.rsplit('.').next().unwrap_or("") {
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "gif" => "image/gif",
                    "webp" => "image/webp",
                    _ => "application/octet-stream",
                };
                let _ = out.start(200, &[("content-type", ctype), ("cache-control", "public, max-age=31536000, immutable")]);
                let _ = out.chunk(&data);
            }
            None => {
                let _ = out.start(404, &[("content-type", "text/plain")]);
            }
        }
        return;
    }

    // ---- Layer 1: what this user may do at all --------------------------
    //
    // Capability checks on mutating routes, before angela sees anything.
    // Reads are per-resource (Layer 2) and checked where the resource is
    // named; creation verbs are global and checked here.
    if let Some(cap) = required_capability(req) {
        if !actor_may(req, cap) {
            let _ = out.json(403, &serde_json::json!({
                "ok": false,
                "error": format!("your account may not use `{cap}` (ask the admin)"),
            }));
            return;
        }
    }

    // ---- Layer 2: reads and posts on named resources --------------------
    if req.method == "POST" && req.path == "/api/chat" {
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        if let Some(sid) = body.get("sessionId").and_then(Value::as_str).filter(|s| !s.is_empty()) {
            if let Some(u) = crate::auth::current_user(req) {
                // Seated counts as speaking too — a person invited into a
                // room who cannot answer is not in the room.
                if !session_open_to(Some(u.id), sid, "post") {
                    let _ = out.json(403, &serde_json::json!({
                        "ok": false, "error": "you may not post in this conversation",
                    }));
                    return;
                }
            }
        }
    }
    if req.method == "POST"
        && req.path.starts_with("/api/sessions/")
        && req.path.ends_with("/participants")
    {
        if let Some(u) = crate::auth::current_user(req) {
            let id = req
                .path
                .trim_start_matches("/api/sessions/")
                .split('/')
                .next()
                .unwrap_or("");
            if !id.is_empty() && !session_open_to(Some(u.id), id, "seat") {
                let _ = out.json(403, &serde_json::json!({
                    "ok": false, "error": "you may not seat participants in this conversation",
                }));
                return;
            }
        }
    }
    if req.method == "GET" && req.path.starts_with("/api/sessions/") {
        if let Some(u) = crate::auth::current_user(req) {
            let id = req
                .path
                .trim_start_matches("/api/sessions/")
                .split('/')
                .next()
                .unwrap_or("");
            if !id.is_empty() && id != "new" && !session_open_to(Some(u.id), id, "read") {
                let _ = out.json(403, &serde_json::json!({
                    "ok": false, "error": "no such conversation",
                }));
                return;
            }
        }
    }

    if req.path == "/api/session/status" && req.method == "POST" {
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let id = body.get("sessionId").and_then(Value::as_str).unwrap_or("");
        let text = body.get("text").and_then(Value::as_str).unwrap_or("");
        let store = &angela(state.config.port).state.store;
        if id.is_empty() || text.trim().is_empty() || store.load(id).is_none() {
            let _ = out.json(400, &serde_json::json!({"ok": false, "error": "Invalid session or status"}));
            return;
        }
        match store.append_event(id, angela::session_store::NewEvent::new("chat_status", serde_json::json!({"text": text}))) {
            Ok(event) => { let _ = out.json(200, &serde_json::json!({"ok": true, "eventId": event.event_id, "ts": event.ts})); }
            Err(_) => { let _ = out.json(500, &serde_json::json!({"ok": false, "error": "Could not save status"})); }
        }
        return;
    }
    if req.path == "/api/session/delete-if-empty" && req.method == "POST" {
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let id = body.get("id").and_then(Value::as_str).unwrap_or_default();
        if id.is_empty() || id.contains("..") || id.contains('/') || id.contains('\\') {
            let _ = out.json(400, &serde_json::json!({"ok": false}));
            return;
        }
        let result = crate::events::if_session_idle(id, || {
            let path = project_root().join(".angela").join("conversations.db");
            let db = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
            db.busy_timeout(std::time::Duration::from_secs(2))?;
            delete_empty_session(&db, id)
        });
        match result {
            Some(Ok(true)) => {
                let server = angela(state.config.port);
                // Finish the ordinary deletion cleanup, including legacy files.
                // The PLAN55 sandbox cascade rides `store.delete` — a no-op on
                // this path by construction: a session with no events has never
                // had a turn, and a container only exists from a turn on.
                server.state.store.delete(id);
                server.state.tabs.forget_session(id);
                server.state.cancels.forget(id);
                server.state.approvals.forget(id);
                let _ = out.json(200, &serde_json::json!({"ok": true, "deleted": true}));
            }
            Some(Err(_)) => { let _ = out.json(500, &serde_json::json!({"ok": false})); }
            _ => { let _ = out.json(200, &serde_json::json!({"ok": true, "deleted": false})); }
        }
        return;
    }
    if req.path == "/api/sessions" && req.method == "GET" {
        let mut body = sessions(state.config.port);
        // "Unshared resources are invisible" (PLAN44 C4): a real,
        // non-admin user sees only conversations they may read — or are
        // seated in (the roster is what makes a room appear in their tree).
        // The dev zone and the admin see everything.
        if let Some(u) = crate::auth::current_user(req) {
            if !u.is_admin() {
                if let Some(rows) = body.get_mut("sessions").and_then(Value::as_array_mut) {
                    let names: Vec<String> = rows
                        .iter()
                        .filter_map(|s| s.get("id").and_then(Value::as_str).map(str::to_string))
                        .collect();
                    let keep: std::collections::HashSet<String> =
                        visible_session_ids(Some(u.id), &names).into_iter().collect();
                    rows.retain(|s| {
                        s.get("id")
                            .and_then(Value::as_str)
                            .is_some_and(|id| keep.contains(id))
                    });
                }
            }
        }
        let _ = out.json(200, &body);
        return;
    }
    if req.path == "/api/events" && req.method == "GET" {
        // The feed is shared; the *window* on it is per-user (PLAN44 C4).
        crate::events::stream(out, crate::auth::current_user(req).map(|u| u.id));
        return;
    }
    if req.path == "/api/models" {
        let body = models();
        crate::events::publish_models(body.clone());
        let _ = out.json(200, &body);
        return;
    }
    // ── sandboxes, repos and worktrees ─────────────────────────────────
    // The other pickers, plus the sandboxes menu's data. `/api/targets` is
    // one GET for all of them: they are read together on every redraw, and
    // separate round-trips would let the menus disagree about what exists.
    //
    // Sandboxes are agent-owned now (PLAN55 D2): the rows come from angela's
    // sandbox policy — each container with the (session, agent) it serves —
    // plus a per-image live rollup (D6, D13: there is no per-container
    // session count; it is always 1).
    if req.path == "/api/targets" && req.method == "GET" {
        let root = project_root();
        let mut body = crate::hostgit::everything(&root);
        let store = &angela(state.config.port).state.store;
        let (sandboxes, up) = match angela::sandbox::list_for_menu(store) {
            Ok(rows) => (rows, true),
            // An empty list and a dead service look identical in a menu, so
            // the flag travels with it and the menu can say which.
            Err(_) => (serde_json::json!([]), false),
        };
        if let Value::Object(m) = &mut body {
            // PLAN55 listing gate: only admin-visible rows travel per viewer.
            let viewer = crate::auth::current_user(req).map(|u| u.id);
            m.insert("sandboxes".into(), visible_sandboxes(viewer, sandboxes));
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
        // PLAN55 listing gate: destroying a sandbox is at least as privileged
        // as seeing it — admin, or edit on the inhabiting agent.
        if action == "sandbox-remove" {
            let viewer = crate::auth::current_user(req).map(|u| u.id);
            let id = f("id");
            let agent = angela(state.config.port)
                .state
                .store
                .sandbox_rows()
                .into_iter()
                .find(|(_, _, cid)| *cid == id)
                .map(|(_, agent, _)| agent)
                .unwrap_or_default();
            if !sandbox_visible(viewer, &agent) {
                let _ = out.json(
                    403,
                    &serde_json::json!({"ok": false, "error": "you may not destroy this sandbox"}),
                );
                return;
            }
        }
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
            // Sandboxes have no hand-driven create/rename (PLAN55 D2/D4/D10):
            // an agent's container is launched at its first turn and dies with
            // its session. Destroy stays, for the Sandbox form's Delete.
            "sandbox-remove" => crate::sbox::destroy(&root, &f("id")).map(|_| serde_json::json!({})),
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
                    let store = &angela(state.config.port).state.store;
                    if let Ok(rows) = angela::sandbox::list_for_menu(store) {
                        let viewer = crate::auth::current_user(req).map(|u| u.id);
                        map.insert("sandboxes".into(), visible_sandboxes(viewer, rows));
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

    // ── automations ──────────────────────────────────────────────────
    // Named prompts on a cron schedule. Chat-app concept (angela knows
    // sessions and turns, not schedules), so the registry and the firing
    // live here; firing itself reuses angela's dispatch, so a run is a
    // real turn in every consumer's eyes.
    if req.path == "/api/automations" && req.method == "GET" {
        let mut body = crate::automations::list(&project_root());
        // Only automations explicitly shared to this user (PLAN44 C4).
        if let Some(u) = crate::auth::current_user(req) {
            if !u.is_admin() {
                let ident = angela(state.config.port).state.store.identity();
                if let Some(rows) = body.get_mut("automations").and_then(Value::as_array_mut) {
                    let names: Vec<String> = rows
                        .iter()
                        .filter_map(|a| a.get("name").and_then(Value::as_str).map(str::to_string))
                        .collect();
                    let keep: std::collections::HashSet<String> =
                        ident.readable(u.id, "automation", &names).into_iter().collect();
                    rows.retain(|a| {
                        a.get("name")
                            .and_then(Value::as_str)
                            .is_some_and(|n| keep.contains(n))
                    });
                }
            }
        }
        let _ = out.json(200, &body);
        return;
    }
    if req.path.starts_with("/api/automations/") && req.method == "POST" {
        let root = project_root();
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let f = |k: &str| {
            body.get(k)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let action = req.path.trim_start_matches("/api/automations/").trim_matches('/');
        let result: Result<Value, String> = match action {
            "automation-save" => crate::automations::upsert(&root, &body).map(|id| serde_json::json!({ "id": id })),
            "automation-remove" => crate::automations::remove(&root, &f("id")).map(|_| serde_json::json!({})),
            "automation-toggle" => crate::automations::set_enabled(
                &root,
                &f("id"),
                body.get("enabled").and_then(Value::as_bool).unwrap_or(true),
            )
            .map(|_| serde_json::json!({})),
            other => Err(format!("unknown automation action: {other}")),
        };
        match result {
            // Same contract as targets: every write answers with the fresh
            // listing, so the caller never has to ask twice.
            Ok(mut v) => {
                if let Value::Object(map) = &mut v {
                    map.insert("ok".into(), serde_json::json!(true));
                    if let Value::Object(list) = crate::automations::list(&root) {
                        for (k, val) in list {
                            map.insert(k, val);
                        }
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
            let mut body = store.projects_detail();
            // A group is visible iff at least one of its sessions is
            // (PLAN44): groups are organizational, never grants — group
            // visibility *derives* from what is shared inside it.
            if let Some(u) = crate::auth::current_user(req) {
                if !u.is_admin() {
                    let all: Vec<Value> = angela(state.config.port).state.store.list_meta();
                    let names: Vec<String> = all
                        .iter()
                        .filter_map(|s| s.get("id").and_then(Value::as_str).map(str::to_string))
                        .collect();
                    let keep: std::collections::HashSet<String> =
                        visible_session_ids(Some(u.id), &names).into_iter().collect();
                    let mut counts: std::collections::HashMap<String, i64> =
                        std::collections::HashMap::new();
                    for s in &all {
                        if s.get("id").and_then(Value::as_str).is_some_and(|id| keep.contains(id)) {
                            let p = s.get("project").and_then(Value::as_str).unwrap_or("").to_string();
                            *counts.entry(p).or_default() += 1;
                        }
                    }
                    if let Some(rows) = body.get_mut("projects").and_then(Value::as_array_mut) {
                        rows.retain(|p| {
                            counts
                                .contains_key(p.get("id").and_then(Value::as_str).unwrap_or(""))
                        });
                        for p in rows.iter_mut() {
                            let key = p.get("id").and_then(Value::as_str).unwrap_or("");
                            if let Some(n) = counts.get(key) {
                                p["count"] = serde_json::json!(n);
                            }
                        }
                    }
                }
            }
            let _ = out.json(200, &body);
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
                // Where this conversation's work happens. There is no
                // session sandbox any more (PLAN55 D2): an agent's container
                // is per (session → agent), owned by the agents plane — so
                // `sandboxId` is deliberately not a composer setting.
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
    let mut areq = ARequest {
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
    let mut body: Value = if chat { serde_json::from_slice(&req.body).unwrap_or(Value::Null) } else { Value::Null };
    // Who said it comes from the session, never from the client body
    // (PLAN44 A6): a multi-human room's transcript is only truthful if the
    // sender is the authenticated one. The dev zone renders as "You".
    if chat {
        if let Some(u) = crate::auth::current_user(req) {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("from".into(), Value::String(u.name));
            }
            areq.body = serde_json::to_vec(&body).unwrap_or_default();
        }
    }
    let session_id = body.get("sessionId").and_then(Value::as_str).unwrap_or_default().to_string();
    let prompt = body.get("content").and_then(Value::as_str).unwrap_or_default().to_string();
    let baseline = if chat && !session_id.is_empty() {
        chat_messages(&angela(state.config.port).state.store, &session_id)
    } else { Vec::new() };
    let mut sink = DevSink {
        out, streaming: false, ended: false, chat, session_id, prompt, baseline,
        turn_id: None, disconnected: false, port: state.config.port,
        assistant_filter: crate::assistant_text::StreamFilter::default(),
        // A session made by this request belongs to its maker (PLAN44).
        claim_owner: if req.path == "/api/session/new"
            || (chat && body.get("sessionId").and_then(Value::as_str).unwrap_or("").is_empty())
        {
            Some(actor_id(req))
        } else {
            None
        },
        viewer: crate::auth::current_user(req).map(|u| u.id),
    };
    angela(state.config.port).dispatch(&areq, &mut sink);
    sink.end();
}

#[cfg(test)]
mod latest_event_tests {
    use super::*;

    #[test]
    fn roster_statuses_survive_reconstruction_in_branch_order() {
        use angela::session_store::{NewEvent, NewSession};
        let store = angela::conv_store::ConvStore::memory("/tmp/chat-status-test").unwrap();
        let id = store.create(NewSession::default()).unwrap()["id"].as_str().unwrap().to_string();
        let status = store.append_event(&id, NewEvent::new("chat_status", serde_json::json!({"text":"Invited @agent"}))).unwrap();
        store.append_event(&id, NewEvent::new("context_window", serde_json::json!({"role":"user", "content":"hi"}))).unwrap();
        store.append_event(&id, NewEvent::new("context_window", serde_json::json!({"role":"assistant", "content":"hello"}))).unwrap();
        store.append_event(&id, NewEvent::new("chat_status", serde_json::json!({"text":"Removed @agent"}))).unwrap();
        let rows = chat_messages(&store, &id);
        assert_eq!(rows.iter().map(|r| r["text"].as_str().unwrap_or("")).collect::<Vec<_>>(), ["Invited @agent", "hi", "hello", "Removed @agent"]);
        assert_eq!(rows[0]["eventId"], status.event_id);
        assert_eq!(rows[0]["ts"], status.ts);
        assert_eq!(rows[0]["commandNotice"], true);
    }

    #[test]
    fn only_sessions_without_events_are_deleted() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE conversation (id TEXT PRIMARY KEY); \
            CREATE TABLE nodes (conversation_id TEXT REFERENCES conversation(id) ON DELETE CASCADE, seq INTEGER, kind TEXT, payload TEXT); \
            INSERT INTO conversation VALUES ('empty'), ('with-events'); \
            INSERT INTO nodes VALUES ('empty', 1, 'session', '{\"kind\":\"created\"}'), \
            ('with-events', 1, 'session', '{}');").unwrap();
        assert!(delete_empty_session(&db, "empty").unwrap());
        assert!(!delete_empty_session(&db, "with-events").unwrap());
        assert!(!delete_empty_session(&db, "missing").unwrap());
        assert_eq!(db.query_row("SELECT COUNT(*) FROM conversation", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
    }

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

    #[test]
    fn token_counts_are_taken_from_the_newest_generation() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE nodes (conversation_id TEXT, seq INTEGER, kind TEXT, gen_total_tokens INTEGER);
            INSERT INTO nodes VALUES ('a', 1, 'gen_info', 1200);
            INSERT INTO nodes VALUES ('a', 2, 'gen_info', 4800);
            INSERT INTO nodes VALUES ('a', 3, 'tool_result', NULL);
            INSERT INTO nodes VALUES ('b', 1, 'session', NULL);").unwrap();
        let mut sessions = serde_json::json!({"sessions": [{"id":"a"}, {"id":"b"}]});
        add_token_counts(&mut sessions, &db);
        assert_eq!(sessions["sessions"][0]["totalTokens"], 4800);
        assert!(sessions["sessions"][1].get("totalTokens").is_none());
    }
}

#[cfg(test)]
mod sandbox_gate_tests {
    use super::*;

    /// PLAN55 listing gate: a sandbox holds its agent's work, so only the
    /// admin and whoever may edit the inhabiting agent see its row — and an
    /// uninhabited instance, having no agent to authorize against, is
    /// admin-only.
    #[test]
    fn a_sandbox_is_visible_only_to_whoever_may_edit_its_agent() {
        let store = angela::conv_store::ConvStore::memory("/tmp/sandbox-gate-test").unwrap();
        let ident = store.identity();
        // The first user is the admin; the second is nobody in particular.
        let bob = ident.create_user("Bob", "Bob").unwrap();
        let alice = ident.create_user("alice", "Alice").unwrap();
        ident.ensure_resource("agent", "video", bob.id).unwrap();
        ident
            .grant(alice.id, "agent", "video", "edit", bob.id, false)
            .unwrap();

        // The dev zone (no viewer) and the admin see everything.
        assert!(sandbox_visible_with(&ident, None, ""));
        assert!(sandbox_visible_with(&ident, Some(bob.id), "video"));
        assert!(sandbox_visible_with(&ident, Some(bob.id), ""));

        // Alice may edit `video`, so its sandbox is hers to see — and nothing
        // else's is.
        assert!(sandbox_visible_with(&ident, Some(alice.id), "video"));
        assert!(!sandbox_visible_with(&ident, Some(alice.id), "chewy"));
        assert!(!sandbox_visible_with(&ident, Some(alice.id), ""));
    }
}

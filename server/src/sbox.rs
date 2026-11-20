//! Talking to **sbox-b**, the host daemon that owns sandboxes.
//!
//! ## Chat is a second front door, not a new API
//!
//! sbox-b's sandbox lifecycle was already HTTP — `POST`/`GET`/`DELETE` on
//! `/api/orchestration/sandboxes` — and its MCP tools (`sandbox_create`,
//! `sandbox_list`, `sandbox_destroy`) are a thin facade over the same
//! functions, validator included. So nothing here re-implements a lifecycle:
//! this module forwards, and the decisions stay where they already were.
//!
//! What chat adds is the half sbox-b has no concept of — the **worktree**.
//! sbox-b takes host bind mounts and knows nothing about where one came
//! from, so [`create`] asks [`crate::hostgit`] for the path and tells it
//! which sandbox claimed it, and [`destroy`] reaps it afterwards.
//!
//! ## Why a hand-rolled client
//!
//! One loopback JSON call, no TLS, no redirects, no auth. Pulling in an HTTP
//! client stack for that would add a dependency tree — and, with `reqwest`,
//! a second async runtime to reason about inside a handler that is
//! deliberately synchronous. sbox-b's own internal client is the same shape
//! for the same reason.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};

/// Where sbox-b listens. Overridable because the port is sbox-b's choice,
/// not ours, and a second instance on a dev box is a normal thing to want.
fn base() -> (String, u16) {
    let host = std::env::var("SBOX_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("SBOX_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000u16);
    (host, port)
}

/// Creating a sandbox boots a container; listing one does not.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const READ_TIMEOUT: Duration = Duration::from_secs(180);

/// One request/response against sbox-b.
///
/// `Connection: close` so the body ends at EOF and there is no chunked
/// decoding to get wrong — this talks to one service, once per call.
fn request(method: &str, path: &str, body: Option<&Value>) -> Result<Value, String> {
    let (host, port) = base();
    let addr = format!("{host}:{port}");
    let sock = addr
        .parse()
        .map_err(|_| format!("bad sbox address: {addr}"))
        .and_then(|a| {
            TcpStream::connect_timeout(&a, CONNECT_TIMEOUT)
                .map_err(|e| format!("sbox is not reachable at {addr}: {e}"))
        })?;
    sock.set_read_timeout(Some(READ_TIMEOUT)).ok();
    sock.set_write_timeout(Some(CONNECT_TIMEOUT)).ok();
    let mut sock = sock;

    let payload = body.map(|b| b.to_string()).unwrap_or_default();
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nAccept: application/json\r\n"
    );
    if body.is_some() {
        head.push_str("Content-Type: application/json\r\n");
        head.push_str(&format!("Content-Length: {}\r\n", payload.len()));
    }
    head.push_str("\r\n");
    sock.write_all(head.as_bytes()).map_err(|e| e.to_string())?;
    if !payload.is_empty() {
        sock.write_all(payload.as_bytes()).map_err(|e| e.to_string())?;
    }
    sock.flush().ok();

    let mut raw = Vec::new();
    sock.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&raw).to_string();
    let Some(split) = text.find("\r\n\r\n") else {
        return Err("sbox sent a malformed response".into());
    };
    let (headers, rest) = text.split_at(split);
    let status: u16 = headers
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let body_text = rest.trim_start_matches("\r\n\r\n").trim();
    let parsed: Value = serde_json::from_str(body_text).unwrap_or(Value::Null);

    if (200..300).contains(&status) {
        return Ok(parsed);
    }
    // sbox-b answers errors as `{code, message}`; surface its wording rather
    // than inventing our own, because it is the half that knows why.
    let msg = parsed
        .get("message")
        .or_else(|| parsed.get("error"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("sbox returned HTTP {status}"));
    Err(msg)
}

/// The sandbox picker's rows.
///
/// The id is sbox-b's; the label is chat's. sbox-b has no idea a human reads
/// a name, so its own id is the fallback — but the moment chat names a box, the
/// menu, the trigger and every other view agree on the friendlier label. The
/// state is the hint, because a stopped sandbox is still selectable and you
/// should know.
pub fn list(root: &Path) -> Result<Value, String> {
    let v = request("GET", "/api/orchestration/sandboxes", None)?;
    let rows = v
        .get("sandboxes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(Value::Array(
        rows.iter()
            .map(|s| {
                let id = s.get("id").and_then(Value::as_str).unwrap_or_default();
                let state = s.get("state").and_then(Value::as_str).unwrap_or("unknown");
                let name = crate::hostgit::sandbox_name(root, &id)
                    .unwrap_or_else(|| id.to_string());
                json!({
                    "id": id,
                    "name": name,
                    "hint": state,
                    "icon": "desktop",
                    "state": state,
                })
            })
            .collect::<Vec<_>>(),
    ))
}

/// Create a sandbox, mounting whatever checkout the composer has selected.
///
/// `kind` picks the image row (`headed` gives a desktop, `headless` does
/// not). The mount is read-write: the point of attaching a worktree is that
/// the agent works in it.
///
/// Creation is **async** on sbox-b's side — it answers 202 and boots in the
/// background — so this returns as soon as the record exists. The picker
/// shows the state, and a sandbox that fails to boot is still there to
/// inspect rather than silently absent.
pub fn create(
    root: &Path,
    kind: &str,
    worktree_id: &str,
    branch_id: &str,
    network: bool,
    name: &str,
) -> Result<Value, String> {
    let kind = match kind {
        "headed" | "headless" => kind,
        _ => "headless",
    };
    let mut payload = json!({ "image": kind, "network": network });

    let mount = crate::hostgit::mount_for(root, worktree_id, branch_id);
    if let Some((path, _)) = &mount {
        payload["volumes"] = json!([{
            "type": "bind",
            "path": path,
            // One fixed guest path, so an agent's instructions do not have
            // to vary with which repo happens to be mounted. The constant
            // is angela's because the other half of the bargain is there:
            // the `## Environment` section tells the agent to work here.
            "mount": angela::harness::SANDBOX_MOUNT,
            "mode": "rw",
        }]);
    }

    let created = request("POST", "/api/orchestration/sandboxes", Some(&payload))?;
    let id = created
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    // The label is chat's: sbox-b names nothing, and a box that was made on
    // purpose should be findable by what you called it, not a hex id. The
    // write is best-effort — the box already exists in sbox-b, and a label
    // that fails to store degrades to the id rather than failing the create.
    if !name.trim().is_empty() {
        let _ = crate::hostgit::set_sandbox_name(root, &id, name);
    }

    // Tie the checkout to the box, so destroying the box removes it. Only a
    // worktree is claimed — a plain repo was here first and is not ours.
    if let Some((_, wt)) = &mount {
        if !wt.is_empty() && !id.is_empty() {
            crate::hostgit::claim_worktree(root, wt, &id);
        }
    }
    Ok(json!({
        "id": id,
        "state": created.get("state").cloned().unwrap_or(Value::Null),
        // Creating a sandbox is how you start working in one, so the picker
        // lands on it rather than leaving the row saying "No sandbox" next
        // to a box that was made on purpose.
        "select": id,
        "selectKind": "sandbox",
    }))
}

/// Destroy a sandbox, then reap the worktree it was holding.
///
/// In that order: if sbox-b refuses, the checkout is still mounted into a
/// live container and removing it would pull the floor out from under a
/// running agent.
pub fn destroy(root: &Path, id: &str) -> Result<Value, String> {
    if id.trim().is_empty() {
        return Err("which sandbox?".into());
    }
    let path = format!("/api/orchestration/sandboxes/{id}");
    let out = request("DELETE", &path, None)?;
    crate::hostgit::forget_for_sandbox(root, id);
    crate::hostgit::forget_sandbox_name(root, id);
    Ok(out)
}

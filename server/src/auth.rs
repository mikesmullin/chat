//! The perimeter (PLAN44 D12): session cookies, the login form, Shields
//! Up/Down, and the ban list — one gate in front of every route.
//!
//! The rules, verbatim from the decisions:
//!
//! * **Shields Up (default):** any request from a non-loopback IPv4 without
//!   a valid session cookie gets its connection **hung up** and that IP is
//!   **instantly, permanently banned** (kernel-dropped by `firewall`).
//! * **Shields Down:** the only anonymous surface is `GET /login` and
//!   `POST /login` (the form is self-contained — zero assets). Anything
//!   else anonymous is refused empty, but not banned.
//! * **Bans persist across shield state** until an admin lifts them.
//! * **Loopback is the dev zone** (D12: "exempt from bans and shield
//!   rules"): allowed through with no cookie. A trusted socket on the
//!   machine itself is the one case where friction costs more than it buys;
//!   the household never arrives over loopback.
//!
//! Identity comes from angela's `identity` half: the cookie is a device
//! token (stored hashed), and login is **username-only** — an unknown name
//! creates the account (the out-of-band conversation with the admin is the
//! 2FA, D3/D12).

use std::collections::BTreeSet;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rustex1_devserver::{Gate, GateReply, GateVerdict, Request};
use serde_json::json;

use crate::firewall::Firewall;

pub const COOKIE: &str = "chat_session";

/// The one perimeter, reachable from the API layer (shields/bans admin
/// routes). Installed by `main` before the server starts.
static PERIMETER: std::sync::OnceLock<std::sync::Arc<Perimeter>> = std::sync::OnceLock::new();

pub fn init(p: std::sync::Arc<Perimeter>) {
    let _ = PERIMETER.set(p);
}

pub fn instance() -> &'static std::sync::Arc<Perimeter> {
    PERIMETER.get().expect("auth::init before the first request")
}
/// The login form: one self-contained document — inline CSS and JS, no
/// assets — because it is the only thing an anonymous client may fetch, and
/// every asset it pulled would be one more exception in the gate.
const LOGIN_FORM: &str = include_str!("login.html");

#[derive(Default)]
struct PerimeterState {
    /// false = Shields Up (the default when the file is missing).
    shields_down: bool,
    bans: BTreeSet<String>,
}

/// The app's source of truth for shields and bans; the firewall is kept in
/// step with it, never the other way round.
pub struct Perimeter {
    root: PathBuf,
    state: Mutex<PerimeterState>,
    firewall: Mutex<Option<Firewall>>,
}

impl Perimeter {
    /// Load the persisted state and bring the kernel rules up to date.
    pub fn load(root: impl AsRef<Path>) -> Perimeter {
        let root = root.as_ref().to_path_buf();
        let text = std::fs::read_to_string(path_of(&root)).unwrap_or_default();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
        let state = PerimeterState {
            shields_down: v.get("shields").and_then(|s| s.as_str()) == Some("down"),
            bans: v
                .get("bans")
                .and_then(|b| b.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        };
        let fw = Firewall::sync(&state.bans.iter().cloned().collect::<Vec<_>>());
        Perimeter {
            root,
            state: Mutex::new(state),
            firewall: Mutex::new(Some(fw)),
        }
    }

    fn save(&self) {
        let state = self.state.lock().unwrap();
        let v = json!({
            "shields": if state.shields_down { "down" } else { "up" },
            "bans": state.bans.iter().collect::<Vec<_>>(),
        });
        drop(state);
        let p = path_of(&self.root);
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let text = serde_json::to_string_pretty(&v).unwrap_or_default();
        let tmp = p.with_extension("json.tmp");
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, &p);
        }
    }

    pub fn shields_down(&self) -> bool {
        self.state.lock().unwrap().shields_down
    }

    pub fn set_shields_down(&self, down: bool) {
        self.state.lock().unwrap().shields_down = down;
        self.save();
    }

    pub fn bans(&self) -> Vec<String> {
        self.state.lock().unwrap().bans.iter().cloned().collect()
    }

    /// Whether bans are kernel-enforced (nftables) or app-only.
    pub fn firewall_ok(&self) -> bool {
        self.firewall
            .lock()
            .unwrap()
            .as_ref()
            .map(|f| f.ok())
            .unwrap_or(false)
    }

    pub fn is_banned(&self, ip: &str) -> bool {
        self.state.lock().unwrap().bans.contains(ip)
    }

    /// Ban permanently. Loopback is exempt (dev access) and the ban list is
    /// the truth — the firewall merely enforces it.
    pub fn ban(&self, ip: &str) {
        let Ok(addr) = ip.parse::<IpAddr>() else {
            return;
        };
        if addr.is_loopback() {
            return;
        }
        let mut state = self.state.lock().unwrap();
        if !state.bans.insert(ip.to_string()) {
            return;
        }
        drop(state);
        self.save();
        if let Some(fw) = self.firewall.lock().unwrap().as_ref() {
            fw.ban(ip);
        }
    }

    pub fn unban(&self, ip: &str) {
        self.state.lock().unwrap().bans.remove(ip);
        self.save();
        if let Some(fw) = self.firewall.lock().unwrap().as_ref() {
            fw.unban(ip);
        }
    }

    /// The gate the devserver consults on every request.
    pub fn gate(self: &std::sync::Arc<Self>) -> std::sync::Arc<dyn Gate> {
        std::sync::Arc::new(GateImpl {
            perimeter: std::sync::Arc::clone(self),
        })
    }
}

fn path_of(root: &Path) -> PathBuf {
    root.join(".angela").join("perimeter.json")
}

struct GateImpl {
    perimeter: std::sync::Arc<Perimeter>,
}

/// Is this socket the machine talking to itself?
fn is_loopback(peer: Option<IpAddr>) -> bool {
    peer.map(|p| p.is_loopback()).unwrap_or(false)
}

/// The `chat_session` cookie value, if the request carries one.
pub fn cookie_of(req: &Request) -> Option<String> {
    for (k, v) in &req.headers {
        if k == "cookie" {
            for part in v.split(';') {
                let part = part.trim();
                if let Some(tok) = part.strip_prefix(&format!("{COOKIE}=")) {
                    if !tok.is_empty() {
                        return Some(tok.to_string());
                    }
                }
            }
        }
    }
    None
}

/// The logged-in user, re-resolved on demand (cheap: one indexed lookup).
/// `None` = anonymous. The gate has already turned anonymous non-loopback
/// traffic away by the time a handler calls this.
pub fn current_user(req: &Request) -> Option<angela::identity::User> {
    current_session(req).map(|(u, _)| u)
}

/// The session behind a request: who it acts as, and the device row that
/// says how it got there.
pub fn current_session(req: &Request) -> Option<(angela::identity::User, angela::identity::Device)> {
    let token = cookie_of(req)?;
    crate::api::angela(0)
        .state
        .store
        .identity()
        .device_by_token(&token)
}

/// The real actor behind a session. A masquerade device's fingerprint
/// names its backer (`masquerade:by:<admin>`): the face the session wears
/// may be anyone's, but the authority behind it is the admin's (PLAN44 D4 —
/// the switcher survives the mask; admin *operations* do not).
pub fn real_actor(
    ident: &angela::identity::Identity<'_>,
    acting: &angela::identity::User,
    device: &angela::identity::Device,
) -> angela::identity::User {
    device
        .fingerprint
        .strip_prefix("masquerade:by:")
        .and_then(|name| ident.user_by_name(name))
        .unwrap_or_else(|| acting.clone())
}

/// Is this session wearing another account's face? The backer named in the
/// device's fingerprint says who stands behind it; the mask is on when the
/// face is not the backer (switching back to yourself turns it off, even
/// though the device keeps its provenance mark).
pub fn is_masked(
    acting: &angela::identity::User,
    device: &angela::identity::Device,
) -> bool {
    device
        .fingerprint
        .strip_prefix("masquerade:by:")
        .is_some_and(|backer| !acting.name.eq_ignore_ascii_case(backer))
}

impl Gate for GateImpl {
    /// Before the websocket branch: a banned IP gets nothing at all.
    fn early(&self, _req: &Request, peer: Option<IpAddr>) -> bool {
        if is_loopback(peer) {
            return true;
        }
        match peer {
            Some(ip) => !self.perimeter.is_banned(&ip.to_string()),
            // No peer address is not a licence; refuse rather than guess.
            None => false,
        }
    }

    /// After the body: the full decision.
    fn handle(&self, req: &Request, peer: Option<IpAddr>) -> GateVerdict {
        if !self.early(req, peer) {
            return GateVerdict::HangUp;
        }

        let token = cookie_of(req);
        let logged_in = token
            .as_ref()
            .is_some_and(|t| crate::api::angela(0).state.store.identity().device_by_token(t).is_some());

        // The gate's own endpoints work in every zone: a dev machine
        // bootstraps accounts from localhost too, and logout only ever ends
        // the caller's own session.
        match (req.method.as_str(), req.path.as_str(), logged_in) {
            ("POST", "/logout", true) => return logout(token.as_ref().unwrap()),
            ("POST", "/masquerade", true) => return masquerade(req, token.as_ref().unwrap(), peer),
            ("GET", "/login", false) => {
                return GateVerdict::Reply(GateReply {
                    status: 200,
                    headers: vec![],
                    content_type: "text/html; charset=utf-8",
                    body: LOGIN_FORM.as_bytes().to_vec(),
                })
            }
            ("POST", "/login", false) => return login(req, peer),
            _ => {}
        }

        // A cookieless visit to the *app* is a login, not a ghost identity:
        // loopback document requests go to the form like everyone else
        // (the "Dev" chip is gone). Raw traffic — the API, assets, curl
        // without `Accept: text/html` — still trusts loopback for local
        // tooling (D12's dev access lives there, see `api::actor_id`).
        if is_loopback(peer) && !logged_in {
            let document = req.method == "GET"
                && header(req, "accept").contains("text/html")
                && req.path != "/login";
            if document {
                return GateVerdict::Reply(GateReply {
                    status: 302,
                    headers: vec![("location".into(), "/login".into())],
                    content_type: "text/plain",
                    body: b"login required".to_vec(),
                });
            }
            return GateVerdict::Allow;
        }
        if logged_in {
            return GateVerdict::Allow;
        }

        let ip = peer.map(|p| p.to_string()).unwrap_or_default();

        // Anonymous from here on. Shields Up bans and hangs up — every
        // request, including this one.
        if !self.perimeter.shields_down() {
            self.perimeter.ban(&ip);
            return GateVerdict::HangUp;
        }

        // Shields Down: nothing else is anonymous-reachable. Empty, not
        // banned.
        GateVerdict::Reply(GateReply {
            status: 404,
            headers: vec![],
            content_type: "text/plain",
            body: b"".to_vec(),
        })
    }
}

/// Username-only login (D3): an unknown name creates the account; the first
/// account ever is the admin purely by being id 1.
fn login(req: &Request, peer: Option<IpAddr>) -> GateVerdict {
    let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap_or(serde_json::Value::Null);
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("").trim();
    if name.is_empty() {
        return GateVerdict::Reply(GateReply {
            status: 400,
            headers: vec![],
            content_type: "application/json",
            body: br#"{"ok":false,"error":"a name is required"}"#.to_vec(),
        });
    }
    let fingerprint = body.get("fingerprint").and_then(|v| v.as_str()).unwrap_or("");
    let ident = crate::api::angela(0).state.store.identity();
    let user = match ident.user_by_name(name) {
        Some(u) if u.disabled => {
            return GateVerdict::Reply(GateReply {
                status: 403,
                headers: vec![],
                content_type: "application/json",
                body: br#"{"ok":false,"error":"that account is disabled"}"#.to_vec(),
            });
        }
        Some(u) => u,
        None => match ident.create_user(name, name) {
            Ok(u) => u,
            Err(e) => {
                return GateVerdict::Reply(GateReply {
                    status: 400,
                    headers: vec![],
                    content_type: "application/json",
                    body: serde_json::to_vec(&json!({ "ok": false, "error": e }))
                        .unwrap_or_default(),
                });
            }
        },
    };
    let ua = header(req, "user-agent");
    let ip = peer.map(|p| p.to_string()).unwrap_or_default();
    match ident.issue_device(user.id, fingerprint, &ua, &ip) {
        Ok((token, _dev)) => {
            ident.audit(&user.name, "", "login", &format!("{ip} {fingerprint}"));
            let cookie = format!(
                "{COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=31536000"
            );
            GateVerdict::Reply(GateReply {
                status: 200,
                headers: vec![("set-cookie".into(), cookie)],
                content_type: "application/json",
                body: serde_json::to_vec(&json!({
                    "ok": true,
                    "user": { "id": user.id, "name": user.name, "admin": user.is_admin() },
                }))
                .unwrap_or_default(),
            })
        }
        Err(e) => GateVerdict::Reply(GateReply {
            status: 500,
            headers: vec![],
            content_type: "application/json",
            body: serde_json::to_vec(&json!({ "ok": false, "error": e })).unwrap_or_default(),
        }),
    }
}

fn header(req: &Request, name: &str) -> String {
    req.headers
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

/// The admin's switch-user (PLAN44 D4): act as another account. The new
/// cookie is that user's, but the device record names the hand that took
/// it — `masquerade:by:<admin>` — so the audit trail reads "alice (by
/// Bob)" exactly as decided. Admin only; everyone else gets 403.
fn masquerade(req: &Request, token: &str, peer: Option<IpAddr>) -> GateVerdict {
    let ident = crate::api::angela(0).state.store.identity();
    let denied = |msg: &str| {
        GateVerdict::Reply(GateReply {
            status: 403,
            headers: vec![],
            content_type: "application/json",
            body: serde_json::to_vec(&json!({ "ok": false, "error": msg })).unwrap_or_default(),
        })
    };
    let Some((acting, dev)) = ident.device_by_token(token) else {
        return denied("unknown session");
    };
    // Authorization is the *backer's*, not the mask's: a session already
    // wearing Alice's face may still switch users as long as Bob stands
    // behind it (PLAN44 D4). Admin *operations* elsewhere still read the
    // acting face, which is exactly "switch back to do admin things".
    let real = real_actor(&ident, &acting, &dev);
    if !real.is_admin() {
        return denied("only the admin may switch users");
    }
    let body: serde_json::Value =
        serde_json::from_slice(&req.body).unwrap_or(serde_json::Value::Null);
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("").trim();
    let Some(target) = ident.user_by_name(name) else {
        return denied("no such user");
    };
    if target.disabled {
        return denied("that account is disabled");
    }
    let ua = header(req, "user-agent");
    // Switching back to the real account ends the mask: the new device is
    // an ordinary one, so `is_masquerade` reads false again.
    // Every device this route makes wears the same provenance mark —
    // including switching back to yourself — so the devices list can tell
    // "a login" from "an admin act" in one place. Whether a mask is *worn*
    // is a different question (acting ≠ backer), answered at /api/me.
    let fingerprint = format!("masquerade:by:{}", real.name);
    let via = if acting.name == real.name || target.name == real.name {
        String::new()
    } else {
        format!(" via {}", acting.name)
    };
    let ip = peer.map(|p| p.to_string()).unwrap_or_else(|| "masquerade".into());
    match ident.issue_device(target.id, &fingerprint, &ua, &ip) {
        Ok((new_token, _)) => {
            // The record names both faces (PLAN44 D4): "alice (by Bob)".
            ident.audit(
                &real.name,
                &target.name,
                "masquerade",
                &format!("{fingerprint}{via}"),
            );
            GateVerdict::Reply(GateReply {
                status: 200,
                headers: vec![(
                    "set-cookie".into(),
                    format!("{COOKIE}={new_token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=31536000"),
                )],
                content_type: "application/json",
                body: serde_json::to_vec(&json!({
                    "ok": true,
                    "user": { "id": target.id, "name": target.name, "admin": target.is_admin() },
                }))
                .unwrap_or_default(),
            })
        }
        Err(e) => denied(&e),
    }
}

/// End the caller's session: the device row is revoked (the cookie stops
/// validating server-side at once) and the browser is told to drop it.
fn logout(token: &str) -> GateVerdict {
    let ident = crate::api::angela(0).state.store.identity();
    if let Some((u, d)) = ident.device_by_token(token) {
        let _ = ident.revoke_device(u.id, d.id);
        ident.audit(&u.name, "", "logout", &format!("device {}", d.id));
    }
    GateVerdict::Reply(GateReply {
        status: 200,
        headers: vec![(
            "set-cookie".into(),
            format!("{COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0"),
        )],
        content_type: "application/json",
        body: br#"{"ok":true}"#.to_vec(),
    })
}

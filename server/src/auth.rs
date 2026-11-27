use std::collections::BTreeSet;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rustex1_devserver::{Gate, GateReply, GateVerdict, Request};
use serde_json::json;

use crate::firewall::Firewall;

pub const COOKIE: &str = "chat_session";

static PERIMETER: std::sync::OnceLock<std::sync::Arc<Perimeter>> = std::sync::OnceLock::new();

pub fn init(p: std::sync::Arc<Perimeter>) {
    let _ = PERIMETER.set(p);
}

pub fn instance() -> &'static std::sync::Arc<Perimeter> {
    PERIMETER.get().expect("auth::init before the first request")
}
const LOGIN_FORM: &str = include_str!("login.html");

#[derive(Default)]
struct PerimeterState {
    shields_down: bool,
    bans: BTreeSet<String>,
}

pub struct Perimeter {
    root: PathBuf,
    state: Mutex<PerimeterState>,
    firewall: Mutex<Option<Firewall>>,
}

impl Perimeter {
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

fn is_loopback(peer: Option<IpAddr>) -> bool {
    peer.map(|p| p.is_loopback()).unwrap_or(false)
}

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

pub fn current_user(req: &Request) -> Option<angela::identity::User> {
    current_session(req).map(|(u, _)| u)
}

pub fn current_session(req: &Request) -> Option<(angela::identity::User, angela::identity::Device)> {
    let token = cookie_of(req)?;
    crate::api::angela(0)
        .state
        .store
        .identity()
        .device_by_token(&token)
}

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
    fn early(&self, _req: &Request, peer: Option<IpAddr>) -> bool {
        if is_loopback(peer) {
            return true;
        }
        match peer {
            Some(ip) => !self.perimeter.is_banned(&ip.to_string()),
            None => false,
        }
    }

    fn handle(&self, req: &Request, peer: Option<IpAddr>) -> GateVerdict {
        if !self.early(req, peer) {
            return GateVerdict::HangUp;
        }

        let token = cookie_of(req);
        let logged_in = token
            .as_ref()
            .is_some_and(|t| crate::api::angela(0).state.store.identity().device_by_token(t).is_some());

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

        if !self.perimeter.shields_down() {
            self.perimeter.ban(&ip);
            return GateVerdict::HangUp;
        }

        GateVerdict::Reply(GateReply {
            status: 404,
            headers: vec![],
            content_type: "text/plain",
            body: b"".to_vec(),
        })
    }
}

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
    let fingerprint = format!("masquerade:by:{}", real.name);
    let via = if acting.name == real.name || target.name == real.name {
        String::new()
    } else {
        format!(" via {}", acting.name)
    };
    let ip = peer.map(|p| p.to_string()).unwrap_or_else(|| "masquerade".into());
    match ident.issue_device(target.id, &fingerprint, &ua, &ip) {
        Ok((new_token, _)) => {
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

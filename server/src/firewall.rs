use std::net::IpAddr;
use std::process::Command;

const TABLE: &str = "chat";
const SET: &str = "banned";

fn nft(args: &[&str]) -> bool {
    Command::new("nft")
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn scaffold() -> bool {
    let t = nft(&["add", "table", "inet", TABLE]);
    let s = nft(&[
        "add", "set", "inet", TABLE, SET, "{", "type", "ipv4_addr;", "}",
    ]);
    let c = nft(&[
        "add", "chain", "inet", TABLE, "input", "{",
        "type", "filter", "hook", "input", "priority", "-10;", "policy", "accept;", "}",
    ]);
    let r = nft(&[
        "add", "rule", "inet", TABLE, "input",
        "tcp", "dport", "3020", "ip", "saddr", &format!("@{SET}"), "drop",
    ]);
    t || s || c || r || nft(&["list", "table", "inet", TABLE])
}

pub struct Firewall {
    ok: bool,
}

impl Firewall {
    pub fn sync(bans: &[String]) -> Firewall {
        let ok = scaffold() && {
            let _ = nft(&["flush", "set", "inet", TABLE, SET]);
            true
        };
        let fw = Firewall { ok };
        for ip in bans {
            fw.ban(ip);
        }
        if !fw.ok {
            eprintln!(
                "firewall: nftables unavailable — bans are enforced by the app only \
                 (run as root, or grant nft via sudoers, for kernel-level drops)"
            );
        }
        fw
    }

    pub fn ok(&self) -> bool {
        self.ok
    }

    pub fn ban(&self, ip: &str) -> bool {
        if !self.ok {
            return false;
        }
        let Ok(addr) = ip.parse::<IpAddr>() else {
            return false;
        };
        if addr.is_loopback() || !addr.is_ipv4() {
            return false;
        }
        nft(&[
            "add", "element", "inet", TABLE, SET, "{", ip, "}",
        ])
    }

    pub fn unban(&self, ip: &str) {
        if !self.ok {
            return;
        }
        let _ = nft(&[
            "delete", "element", "inet", TABLE, SET, "{", ip, "}",
        ]);
    }
}

//! The firewall half of the perimeter (PLAN44 D12): banned IPv4s are not
//! just refused by the app, they are **dropped at the kernel** for
//! `tcp/3020`, so a banned scanner cannot even hold a socket open.
//!
//! Arch Linux, so nftables. One table, one set, one rule — the app owns the
//! ban *list* (it is the source of truth, see `auth::Perimeter`), and this
//! module keeps the set in step with it: on every ban, on every unban, and
//! once at startup in case the rules were flushed underneath us.
//!
//! ```nft
//! table inet chat {
//!   set banned { type ipv4_addr; }
//!   chain input { type filter hook input priority -10; policy accept;
//!                 tcp dport 3020 ip saddr @banned drop; }
//! }
//! ```
//!
//! Needs privileges (run as root, or a narrow sudoers rule for exactly
//! these nft invocations). When the commands fail — no nft, no permission —
//! the app degrades to hanging connections up itself: still correct, just
//! not kernel-enforced. `ok()` reports which world we are in, once, loudly.

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

/// Bring the table/set/rule into existence. Idempotent-ish: a second call
/// fails the `add table` (exists) but the set/rule adds also fail
/// harmlessly, and `sync` re-adds what matters.
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
    /// Prepare the rules and load `bans` into them. Never fails the boot:
    /// a world without kernel enforcement is reported, not fatal.
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

    /// Kernel-drop `ip`. Loopback is never banned (dev access) and IPv6 is
    /// out of scope — the household arrives on IPv4.
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

    /// Lift the drop. A missing element is not an error worth reporting.
    pub fn unban(&self, ip: &str) {
        if !self.ok {
            return;
        }
        let _ = nft(&[
            "delete", "element", "inet", TABLE, SET, "{", ip, "}",
        ]);
    }
}

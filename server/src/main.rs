//! chat server — the frontend web app shell, served by the rustex1 dev
//! server (static + HMR + wasm rebuild + self-restart).
//!
//! Owns NO agent logic — every `/api/*` call forwards to angela
//! (default http://127.0.0.1:8763, env ANGELA_ORIGIN), the thin
//! MCP-only harness over agl. The SPA is a pure data-plane client:
//! agents, sessions, chat NDJSON, approvals, allowlist, context-window.

mod proxy;

use rustex1_devserver::{args_to_config, Config, Server};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = std::env::current_dir().expect("current dir");
    let (config, rebuild) = args_to_config(&args, Config::discover(&root));

    // One forward, same streamed machinery as sbox-a: /api/ rides to angela
    // verbatim (the SPA's data plane). The browser never dials angela
    // directly, so the harness keeps its loopback trust model.
    let mut server = Server::new(config).raw("/api/", proxy::handle_api);
    if rebuild {
        server = server.rebuild_on_start();
    }
    server.run();
}

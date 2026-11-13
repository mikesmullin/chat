//! chat server — the frontend web app shell, served by the rustex1 dev
//! server (static + HMR + wasm rebuild + self-restart).
//!
//! angela runs **inside this process** (`mod api`): sessions, the chat NDJSON
//! stream and the agl turn loop are all served from here. The chat app is one
//! server — there is no second daemon to start and no port to coordinate.
//! Sessions live under `<cwd>/.angela/sessions`, or `ANGELA_ROOT` if set.

mod api;

use rustex1_devserver::{args_to_config, Config, Server};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = std::env::current_dir().expect("current dir");
    let (config, rebuild) = args_to_config(&args, Config::discover(&root));

    // /api/ is answered in-process by the embedded angela. The NDJSON chat
    // stream is written frame by frame, so a turn reaches the browser as the
    // model produces it.
    let mut server = Server::new(config).raw("/api/", api::handle_api);
    if rebuild {
        server = server.rebuild_on_start();
    }
    server.run();
}

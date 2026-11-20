//! chat server — the frontend web app shell, served by the rustex1 dev
//! server (static + HMR + wasm rebuild + self-restart).
//!
//! angela runs **inside this process** (`mod api`): sessions, the chat NDJSON
//! stream and the agl turn loop are all served from here. The chat app is one
//! server — there is no second daemon to start and no port to coordinate.
//! Sessions live under `<cwd>/.angela/sessions`, or `ANGELA_ROOT` if set.

mod api;
mod events;
mod hostgit;
mod sbox;

use rustex1_devserver::{args_to_config, Config, Server};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = std::env::current_dir().expect("current dir");
    let (config, rebuild) = args_to_config(&args, Config::discover(&root));

    // Drop worktree registrations whose directory is gone. A crash between
    // `git worktree add` and the registry write, or a checkout removed by
    // hand, otherwise leaves a row the picker offers and every mount of
    // which fails.
    hostgit::sweep(&api::project_root());

    // /api/ is answered in-process by the embedded angela. The NDJSON chat
    // stream is written frame by frame, so a turn reaches the browser as the
    // model produces it.
    let mut server = Server::new(config).raw("/api/", api::handle_api).on_start(events::start);
    if rebuild {
        server = server.rebuild_on_start();
    }
    server.run();
}

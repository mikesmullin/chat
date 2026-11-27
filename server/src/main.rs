mod api;
mod assistant_text;
mod auth;
mod automations;
mod events;
mod firewall;
mod hostgit;
mod sbox;

use rustex1_devserver::{args_to_config, Config, Server};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = std::env::current_dir().expect("current dir");
    let (config, rebuild) = args_to_config(&args, Config::discover(&root));

    hostgit::sweep(&api::project_root());

    let _ = api::angela(config.port);
    automations::recover_queue();
    let perimeter = std::sync::Arc::new(auth::Perimeter::load(&root));
    auth::init(std::sync::Arc::clone(&perimeter));
    let mut server = Server::new(config)
        .gate(perimeter.gate())
        .raw("/api/", api::handle_api)
        .raw("/media/", api::handle_api)
        .on_start(|state| {
            events::start(state);
            automations::start();
        });
    if rebuild {
        server = server.rebuild_on_start();
    }
    server.run();
}

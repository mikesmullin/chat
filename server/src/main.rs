mod api;
mod assistant_text;
mod auth;
mod automations;
mod db;
mod err;
mod events;
mod firewall;
mod hostgit;
mod jobs;
mod sbox;
mod vcs;

use rustex1_devserver::{args_to_config, Config, Server};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = std::env::current_dir().expect("current dir");
    let (config, rebuild) = args_to_config(&args, Config::discover(&root));

    angela::chat_tools::set_extra_tools(std::sync::Arc::new(|agent: &mut agl::agent::Agent, session: &str| {
        crate::vcs::agent_tool::register(agent, session);
    }));

    if let Err(e) = db::migrate(&api::project_root()) {
        eprintln!("chat schema migration failed: {e}");
        std::process::exit(1);
    }
    hostgit::import_legacy_registry(&api::project_root());

    hostgit::sweep(&api::project_root());
    jobs::scheduler::ensure_scheduled(&api::project_root());
    jobs::scheduler::start(&api::project_root());
    jobs::orchestrator::start(&api::project_root());

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

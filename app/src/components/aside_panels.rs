use rustex1_kit::{js, mjs};

fn close_open(store: &str) {
    if mjs::has_store(store) {
        let s = mjs::store_ref(store);
        js::set_bool(&s, "open", false);
    }
}

fn close_field(store: &str, field: &str) {
    if mjs::has_store(store) {
        let s = mjs::store_ref(store);
        js::set_bool(&s, field, false);
    }
}

pub fn close() {
    close_open("agents");
    close_open("settings");
    close_open("tform");
    close_open("sessionDraft");
    close_open("groupDraft");
    close_field("ident", "panelOpen");
    close_field("vcs", "repoPanelOpen");
    close_field("vcs", "prOpen");
    close_field("vcs", "prFormOpen");
    close_field("jobs", "runOpen");
    close_field("jobs", "jobOpen");
    close_field("jobs", "jobFormOpen");
}

use std::cell::Cell;

use wasm_bindgen::JsValue;

use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::api;
use crate::helpers;

thread_local! {
    static REMOVE_ARM: Cell<u32> = const { Cell::new(0) };
}

pub const STORE: &str = "vcs";
pub const EXPR: &str = "$store.vcs";

fn redraw() {
    mjs::redraw();
}

fn notice(s: &JsValue, text: &str) {
    js::set_str(s, "notice", text);
    crate::components::validation_info::note("vcs-notice", "vcs", text);
}

pub fn load_repos(_this: JsValue, _args: js_sys::Array) -> JsValue {
    api::vcs_get("repos", "", |v| {
        let s = mjs::store_ref(STORE);
        js::set(&s, "repos", &js::get(&v, "repos"));
        js::set(&s, "worktrees", &js::get(&v, "worktrees"));
        redraw();
    });
    JsValue::UNDEFINED
}

fn load_prs(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let state = js::get_str(&mjs::store_ref(STORE), "listState");
    api::vcs_get("prs", &format!("state={state}&limit=100"), |v| {
        let s = mjs::store_ref(STORE);
        js::set(&s, "prs", &js::get(&v, "prs"));
        redraw();
    });
    JsValue::UNDEFINED
}

fn set_list_state(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let state = js::arg_str(&args, 0);
    js::set_str(&s, "listState", &state);
    load_prs(JsValue::UNDEFINED, js_sys::Array::new());
    JsValue::UNDEFINED
}

fn open_pr(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    crate::components::aside_panels::close();
    let repo = js::arg_str(&args, 0);
    let number = js::arg_str(&args, 1);
    js::set_str(&s, "prRepo", &repo);
    js::set_str(&s, "prNumber", &number);
    js::set_bool(&s, "prOpen", true);
    js::set(&s, "pr", &js::obj());
    js::set(&s, "status", &js::obj());
    js::set_str(&s, "diffText", "");
    js::set_bool(&s, "diffLoading", true);
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
    let repo_q = repo.clone();
    let number_q = number.clone();
    api::vcs_get("pr", &format!("repo={repo}&number={number}"), move |v| {
        let s = mjs::store_ref(STORE);
        let pr = js::get(&v, "pr");
        if js::get_str(&s, "prRepo") == repo_q && js::get_str(&s, "prNumber") == number_q {
            js::set(&s, "pr", &pr);
            let desc = js::get_str(&pr, "description");
            js::set_str(&s, "descriptionHtml", &helpers::markdown_to_html(&if desc.trim().is_empty() { "_No description._".to_string() } else { desc }));
            redraw();
        }
    });
    let repo_q = repo.clone();
    let number_q = number.clone();
    api::vcs_get("pr-status", &format!("repo={repo}&number={number}"), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get_str(&s, "prRepo") == repo_q && js::get_str(&s, "prNumber") == number_q {
            js::set(&s, "status", &js::get(&v, "status"));
            js::set(&s, "checks", &js::get(&v, "checks"));
            redraw();
        }
    });
    let repo_q = repo;
    let number_q = number;
    api::vcs_text("pr-diff", &format!("repo={repo_q}&number={number_q}"), move |v| {
        let s = mjs::store_ref(STORE);
        let text = v.as_string().unwrap_or_default();
        if js::get_str(&s, "prRepo") == repo_q && js::get_str(&s, "prNumber") == number_q {
            js::set_str(&s, "diffText", &text);
            js::set(&s, "diffFiles", &crate::components::diff_view::parse_json(&text));
            js::set_bool(&s, "diffLoading", false);
            redraw();
        }
    });
    redraw();
    JsValue::UNDEFINED
}

fn close_pr(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "prOpen", false);
    js::set(&s, "pr", &js::obj());
    redraw();
    JsValue::UNDEFINED
}

fn refresh_pr(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let repo = js::get_str(&s, "prRepo");
    let number = js::get_str(&s, "prNumber");
    let args2 = js_sys::Array::new();
    args2.push(&JsValue::from_str(&repo));
    args2.push(&JsValue::from_str(&number));
    open_pr(JsValue::UNDEFINED, args2);
    let _ = args;
    JsValue::UNDEFINED
}

fn set_diff_mode(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_str(&s, "diffMode", &js::arg_str(&args, 0));
    redraw();
    JsValue::UNDEFINED
}

fn open_pr_form(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    crate::components::aside_panels::close();
    let repo = if args.length() > 0 { js::arg_str(&args, 0) } else { String::new() };
    js::set_bool(&s, "prFormOpen", true);
    js::set_str(&s, "formError", "");
    if !repo.is_empty() {
        js::set_str(&s, "formRepo", &repo);
        load_branches_of(&repo);
    }
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
    redraw();
    JsValue::UNDEFINED
}

fn load_branches_of(repo: &str) {
    let repo_owned = repo.to_string();
    api::vcs_get("branches", &format!("repo={repo_owned}"), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get_str(&s, "formRepo") != repo_owned {
            return;
        }
        let branches = js_sys::Array::from(&js::get(&v, "branches"));
        let source = js_sys::Array::new();
        let target = js_sys::Array::new();
        for b in branches.iter() {
            let name = js::get_str(&b, "name");
            let row = js::obj();
            js::set_str(&row, "name", &name);
            js::set_bool(&row, "isDefault", js::get_bool(&b, "isDefault"));
            source.push(&row);
            if js::get_bool(&b, "isDefault") {
                js::set_str(&mjs::store_ref(STORE), "formTarget", &name);
            }
            target.push(&row);
        }
        js::set(&s, "formSourceBranches", &source);
        js::set(&s, "formTargetBranches", &target);
        redraw();
    });
}

fn pick_form_repo(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let repo = js::arg_str(&args, 0);
    js::set_str(&s, "formRepo", &repo);
    js::set_str(&s, "formSource", "");
    load_branches_of(&repo);
    redraw();
    JsValue::UNDEFINED
}

fn pick_form_branch(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let which = js::arg_str(&args, 0);
    let name = js::arg_str(&args, 1);
    if which == "source" {
        js::set_str(&s, "formSource", &name);
    } else {
        js::set_str(&s, "formTarget", &name);
    }
    redraw();
    JsValue::UNDEFINED
}

fn form_input(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let field = js::arg_str(&args, 0);
    let value = js::arg_str(&args, 1);
    js::set_str(&s, &field, &value);
    JsValue::UNDEFINED
}

fn create_pr(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let repo = js::get_str(&s, "formRepo");
    let source = js::get_str(&s, "formSource");
    let target = js::get_str(&s, "formTarget");
    let title = js::get_str(&s, "formTitle");
    let description = js::get_str(&s, "formDescription");
    let attachments = js::get_str(&s, "formAttachments");
    if repo.is_empty() || source.is_empty() || title.trim().is_empty() {
        notice(&s, "a pull request needs a repo, a source branch and a title");
        redraw();
        return JsValue::UNDEFINED;
    }
    let payload = js::obj();
    js::set_str(&payload, "repo", &repo);
    js::set_str(&payload, "sourceBranch", &source);
    js::set_str(&payload, "targetBranch", &target);
    js::set_str(&payload, "title", &title);
    js::set_str(&payload, "description", &description);
    let list = js_sys::Array::new();
    for line in attachments.lines() {
        let path = line.trim();
        if path.is_empty() {
            continue;
        }
        let item = js::obj();
        js::set_str(&item, "path", path);
        list.push(&item);
    }
    if list.length() > 0 {
        js::set(&payload, "attachments", &list);
    }
    api::vcs_post("pr-create", &payload.into(), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get(&v, "ok").as_bool() == Some(false) {
            let error = js::get_str(&v, "error");
            js::set_str(&s, "formError", &if error.is_empty() { "could not open the pull request".to_string() } else { error });
            redraw();
            return;
        }
        let pr = js::get(&v, "pr");
        let repo = js::get_str(&pr, "repo");
        let number = js::get_str(&pr, "number");
        js::set_bool(&s, "prFormOpen", false);
        load_prs(JsValue::UNDEFINED, js_sys::Array::new());
        let args = js_sys::Array::new();
        args.push(&JsValue::from_str(&repo));
        args.push(&JsValue::from_str(&number));
        open_pr(JsValue::UNDEFINED, args);
    });
    redraw();
    JsValue::UNDEFINED
}

fn submit_review(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let repo = js::get_str(&s, "prRepo");
    let number = js::get_str(&s, "prNumber");
    let decision = js::get_str(&s, "reviewDecision");
    let comment = js::get_str(&s, "reviewComment");
    if decision.is_empty() {
        notice(&s, "pick a decision first");
        redraw();
        return JsValue::UNDEFINED;
    }
    let payload = js::obj();
    js::set_str(&payload, "repo", &repo);
    js::set_str(&payload, "number", &number);
    js::set_str(&payload, "decision", &decision);
    js::set_str(&payload, "comment", &comment);
    api::vcs_post("pr-review", &payload.into(), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get(&v, "ok").as_bool() == Some(false) {
            notice(&s, &js::get_str(&v, "error"));
            redraw();
            return;
        }
        js::set_str(&s, "reviewComment", "");
        load_prs(JsValue::UNDEFINED, js_sys::Array::new());
        let args = js_sys::Array::new();
        args.push(&JsValue::from_str(&js::get_str(&s, "prRepo")));
        args.push(&JsValue::from_str(&js::get_str(&s, "prNumber")));
        open_pr(JsValue::UNDEFINED, args);
    });
    redraw();
    JsValue::UNDEFINED
}

fn set_review_decision(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_str(&s, "reviewDecision", &js::arg_str(&args, 0));
    redraw();
    JsValue::UNDEFINED
}

fn merge_pr(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let repo = js::get_str(&s, "prRepo");
    let number = js::get_str(&s, "prNumber");
    let payload = js::obj();
    js::set_str(&payload, "repo", &repo);
    js::set_str(&payload, "number", &number);
    api::vcs_post("pr-merge", &payload.into(), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get(&v, "ok").as_bool() == Some(false) {
            notice(&s, &js::get_str(&v, "error"));
            redraw();
            return;
        }
        load_prs(JsValue::UNDEFINED, js_sys::Array::new());
        let args = js_sys::Array::new();
        args.push(&JsValue::from_str(&js::get_str(&s, "prRepo")));
        args.push(&JsValue::from_str(&js::get_str(&s, "prNumber")));
        open_pr(JsValue::UNDEFINED, args);
    });
    redraw();
    JsValue::UNDEFINED
}

fn cleanup_pr(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let delete_branch = js::arg_str(&args, 0) == "branch";
    let payload = js::obj();
    js::set_str(&payload, "repo", &js::get_str(&s, "prRepo"));
    js::set_str(&payload, "number", &js::get_str(&s, "prNumber"));
    js::set_bool(&payload, "deleteBranch", delete_branch);
    js::set_bool(&payload, "reapWorktree", true);
    api::vcs_post("pr-cleanup", &payload.into(), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get(&v, "ok").as_bool() == Some(false) {
            notice(&s, &js::get_str(&v, "error"));
        } else {
            notice(&s, "cleaned up");
        }
        load_repos(JsValue::UNDEFINED, js_sys::Array::new());
        let args = js_sys::Array::new();
        args.push(&JsValue::from_str(&js::get_str(&s, "prRepo")));
        args.push(&JsValue::from_str(&js::get_str(&s, "prNumber")));
        open_pr(JsValue::UNDEFINED, args);
    });
    redraw();
    JsValue::UNDEFINED
}

fn delete_pr(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let repo = js::get_str(&s, "prRepo");
    let number = js::get_str(&s, "prNumber");
    if js::get_str(&s, "deleteArmed") != number {
        js::set_str(&s, "deleteArmed", &number);
        notice(&s, "press delete again to remove this pull request and its attachments");
        redraw();
        return JsValue::UNDEFINED;
    }
    js::set_str(&s, "deleteArmed", "");
    api::vcs_delete("pr-delete", &format!("repo={repo}&number={number}"), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get(&v, "ok").as_bool() == Some(false) {
            notice(&s, &js::get_str(&v, "error"));
        }
        js::set_bool(&s, "prOpen", false);
        load_prs(JsValue::UNDEFINED, js_sys::Array::new());
    });
    redraw();
    JsValue::UNDEFINED
}

fn add_attachment(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let doc = js::get(&js::global(), "document");
    let input = js::call(&doc, "querySelector", &[JsValue::from_str("#home .pr-attach-input")]);
    if !js::is_missing(&input) {
        js::call(&input, "click", &[]);
    }
    JsValue::UNDEFINED
}

fn attachment_chosen(_this: JsValue, args: js_sys::Array) -> JsValue {
    let ev = js::arg(&args, 0);
    let target = js::get(&ev, "target");
    let files = js::get(&target, "files");
    let file = js_sys::Array::from(&files).get(0);
    if file.is_null() || file.is_undefined() {
        return JsValue::UNDEFINED;
    }
    let s = mjs::store_ref(STORE);
    let repo = js::get_str(&s, "prRepo");
    let number = js::get_str(&s, "prNumber");
    if repo.is_empty() || number.is_empty() {
        notice(&s, "open a pull request before attaching files");
        redraw();
        return JsValue::UNDEFINED;
    }
    let name = js::get_str(&file, "name");
    let url = format!("/api/vcs/pr-attach?repo={repo}&number={number}&name={name}");
    let done = wasm_bindgen::closure::Closure::once_into_js(move |resp: JsValue| {
        let parsed = wasm_bindgen::closure::Closure::once_into_js(move |_v: JsValue| {
            let args = js_sys::Array::new();
            args.push(&JsValue::from_str(&repo));
            args.push(&JsValue::from_str(&number));
            open_pr(JsValue::UNDEFINED, args);
        });
        let body = js::call(&resp, "json", &[]);
        js::call(&body, "then", &[parsed]);
    });
    let opts = js::obj();
    js::set_str(&opts, "method", "POST");
    js::set(&opts, "body", &file);
    let fetch = js::call(&js::global(), "fetch", &[JsValue::from_str(&url), opts.into()]);
    js::call(&fetch, "then", &[done]);
    JsValue::UNDEFINED
}

fn repo_remove(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let name = js::get_str(&s, "repoOpen");
    if js::get_str(&s, "removeArmed") != name {
        js::set_str(&s, "removeArmed", &name);
        redraw();
        let gen = REMOVE_ARM.with(|g| {
            let next = g.get() + 1;
            g.set(next);
            next
        });
        let name_owned = name.clone();
        rustex1_kit::lifecycle::after(3000.0, move || {
            let current = REMOVE_ARM.with(|g| g.get());
            if current != gen {
                return;
            }
            let s = mjs::store_ref(STORE);
            if js::get_str(&s, "removeArmed") != name_owned {
                return;
            }
            js::set_str(&s, "removeArmed", "");
            redraw();
        });
        return JsValue::UNDEFINED;
    }
    js::set_str(&s, "removeArmed", "");
    let mut repo_id = String::new();
    for repo in js_sys::Array::from(&js::get(&s, "repos")).iter() {
        if js::get_str(&repo, "name") == name {
            repo_id = js::get_str(&repo, "id");
            break;
        }
    }
    let payload = js::obj();
    js::set_str(&payload, "id", &repo_id);
    api::vcs_post("repo-remove", &payload.into(), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get(&v, "ok").as_bool() == Some(false) {
            js::set_str(&s, "repoError", &js::get_str(&v, "error"));
            redraw();
            return;
        }
        js::set_bool(&s, "repoPanelOpen", false);
        notice(&s, "untracked — the repo on disk is untouched");
        load_repos(JsValue::UNDEFINED, js_sys::Array::new());
    });
    redraw();
    JsValue::UNDEFINED
}

fn prs_for_repo(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let repo = js::arg_str(&args, 0);
    let show_all = js::get_str(&s, "listState") == "all";
    let out = js_sys::Array::new();
    for pr in js_sys::Array::from(&js::get(&s, "prs")).iter() {
        if js::get_str(&pr, "repo") != repo {
            continue;
        }
        let state = js::get_str(&pr, "state");
        if !show_all && (state == "merged" || state == "rejected") {
            continue;
        }
        out.push(&pr);
    }
    out.into()
}

fn repo_names(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let out = js_sys::Array::new();
    for repo in js_sys::Array::from(&js::get(&s, "repos")).iter() {
        out.push(&js::get(&repo, "name"));
    }
    out.into()
}

fn repo_label(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let name = js::arg_str(&args, 0);
    for repo in js_sys::Array::from(&js::get(&s, "repos")).iter() {
        if js::get_str(&repo, "name") == name {
            return js::get(&repo, "name");
        }
    }
    JsValue::from_str(&name)
}

fn open_repo(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    crate::components::aside_panels::close();
    let name = js::arg_str(&args, 0);
    js::set_str(&s, "repoOpen", &name);
    js::set_bool(&s, "repoPanelOpen", true);
    js::set_str(&s, "removeArmed", "");
    for repo in js_sys::Array::from(&js::get(&s, "repos")).iter() {
        if js::get_str(&repo, "name") == name {
            js::set_str(&s, "repoDesc", &js::get_str(&repo, "description"));
            js::set_str(&s, "repoBranch", &js::get_str(&repo, "defaultBranch"));
            break;
        }
    }
    js::set_str(&s, "repoError", "");
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
    redraw();
    JsValue::UNDEFINED
}

fn close_repo(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "repoPanelOpen", false);
    js::set_str(&s, "removeArmed", "");
    redraw();
    JsValue::UNDEFINED
}

fn repo_save(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let name = js::get_str(&s, "repoOpen");
    let mut repo_id = String::new();
    for repo in js_sys::Array::from(&js::get(&s, "repos")).iter() {
        if js::get_str(&repo, "name") == name {
            repo_id = js::get_str(&repo, "id");
            break;
        }
    }
    if repo_id.is_empty() {
        return JsValue::UNDEFINED;
    }
    let patch = js::obj();
    js::set_str(&patch, "description", &js::get_str(&s, "repoDesc"));
    js::set_str(&patch, "defaultBranch", &js::get_str(&s, "repoBranch"));
    let payload = js::obj();
    js::set_str(&payload, "id", &repo_id);
    js::set(&payload, "patch", &patch);
    api::vcs_post("repo-update", &payload.into(), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get(&v, "ok").as_bool() == Some(false) {
            js::set_str(&s, "repoError", &js::get_str(&v, "error"));
            redraw();
            return;
        }
        notice(&s, "saved");
        load_repos(JsValue::UNDEFINED, js_sys::Array::new());
    });
    redraw();
    JsValue::UNDEFINED
}

fn worktrees_for_repo(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let repo_id = js::arg_str(&args, 0);
    let out = js_sys::Array::new();
    for wt in js_sys::Array::from(&js::get(&s, "worktrees")).iter() {
        if js::get_str(&wt, "repoId") == repo_id {
            out.push(&wt);
        }
    }
    out.into()
}

fn checks_ok(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    for check in js_sys::Array::from(&js::get(&s, "checks")).iter() {
        let state = js::get_str(&check, "state");
        if state != "success" && state != "aborted" {
            return JsValue::FALSE;
        }
    }
    JsValue::TRUE
}

fn checks_pending(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    for check in js_sys::Array::from(&js::get(&s, "checks")).iter() {
        let state = js::get_str(&check, "state");
        if state != "success" && state != "aborted" && state != "failure" {
            return JsValue::TRUE;
        }
    }
    JsValue::FALSE
}

fn ready_to_merge(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let status = js::get(&s, "status");
    let approved = js::get_bool(&status, "approved");
    let mergeable = js::get_str(&status, "mergeable");
    let state = js::get_str(&status, "state");
    if state == "merged" || state == "rejected" {
        return JsValue::FALSE;
    }
    JsValue::from_bool(approved && mergeable == "clean" && checks_ok(JsValue::UNDEFINED, js_sys::Array::new()).as_bool().unwrap_or(false))
}

fn merge_reason(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let status = js::get(&s, "status");
    let state = js::get_str(&status, "state");
    if state == "merged" {
        return JsValue::from_str("merged");
    }
    if state == "rejected" {
        return JsValue::from_str("rejected");
    }
    if !js::get_bool(&status, "approved") {
        let approvals = js::get(&status, "approvals").as_f64().unwrap_or(0.0);
        let need = js::get(&status, "minApprovals").as_f64().unwrap_or(1.0);
        return JsValue::from_str(&format!("{approvals:.0} of {need:.0} approvals"));
    }
    if checks_pending(JsValue::UNDEFINED, js_sys::Array::new()).as_bool().unwrap_or(false) {
        return JsValue::from_str("checks still running");
    }
    if !checks_ok(JsValue::UNDEFINED, js_sys::Array::new()).as_bool().unwrap_or(false) {
        return JsValue::from_str("checks failed");
    }
    if js::get_str(&status, "mergeable") == "conflict" {
        return JsValue::from_str("conflicts with the target branch");
    }
    if js::get_str(&status, "mergeable").is_empty() {
        return JsValue::from_str("mergeability unknown");
    }
    JsValue::from_str("")
}

pub fn apply_topic(v: JsValue) {
    let s = mjs::store_ref(STORE);
    let repos = js::get(&v, "repos");
    let prs = js::get(&v, "prs");
    if !js_sys::Array::is_array(&repos) {
        return;
    }
    if js::get_str(&s, "prRepo").is_empty() {
        let list = js_sys::Array::from(&prs);
        if list.length() > 0 {
            let first = list.get(0);
            let _ = first;
        }
    }
    js::set(&s, "repos", &repos);
    js::set(&s, "worktrees", &js::get(&v, "worktrees"));
    js::set(&s, "prs", &prs);
    redraw();
}

pub fn register() {
    let mut d = Def::new().named("store vcs");
    d.val("repos", &js_sys::Array::new().into())
        .val("worktrees", &js_sys::Array::new().into())
        .val("prs", &js_sys::Array::new().into())
        .val("checks", &js_sys::Array::new().into())
        .val("pr", &js::obj().into())
        .val("status", &js::obj().into())
        .val("diffFiles", &js_sys::Array::new().into())
        .val("formSourceBranches", &js_sys::Array::new().into())
        .val("formTargetBranches", &js_sys::Array::new().into())
        .str("listState", "active")
        .str("prRepo", "")
        .str("prNumber", "")
        .str("diffText", "")
        .str("diffMode", "code")
        .str("descriptionHtml", "")
        .str("reviewDecision", "comment")
        .str("reviewComment", "")
        .str("formRepo", "")
        .str("formSource", "")
        .str("formTarget", "")
        .str("formTitle", "")
        .str("formDescription", "")
        .str("formAttachments", "")
        .str("formError", "")
        .str("repoOpen", "")
        .str("repoDesc", "")
        .str("repoBranch", "")
        .str("repoError", "")
        .str("removeArmed", "")
        .bool("repoPanelOpen", false)
        .str("deleteArmed", "")
        .str("notice", "")
        .bool("prOpen", false)
        .bool("prFormOpen", false)
        .bool("diffLoading", false)
        .method("loadRepos", load_repos)
        .method("loadPrs", load_prs)
        .method("setListState", set_list_state)
        .method("openPr", open_pr)
        .method("closePr", close_pr)
        .method("refreshPr", refresh_pr)
        .method("setDiffMode", set_diff_mode)
        .method("openPrForm", open_pr_form)
        .method("openRepo", open_repo)
        .method("closeRepo", close_repo)
        .method("repoSave", repo_save)
        .method("pickFormRepo", pick_form_repo)
        .method("pickFormBranch", pick_form_branch)
        .method("formInput", form_input)
        .method("createPr", create_pr)
        .method("submitReview", submit_review)
        .method("setReviewDecision", set_review_decision)
        .method("mergePr", merge_pr)
        .method("cleanupPr", cleanup_pr)
        .method("deletePr", delete_pr)
        .method("addAttachment", add_attachment)
        .method("attachmentChosen", attachment_chosen)
        .method("repoRemove", repo_remove)
        .method("prsForRepo", prs_for_repo)
        .method("repoNames", repo_names)
        .method("repoLabel", repo_label)
        .method("worktreesForRepo", worktrees_for_repo)
        .method("checksOk", checks_ok)
        .method("checksPending", checks_pending)
        .method("readyToMerge", ready_to_merge)
        .method("mergeReason", merge_reason);
    mjs::store(STORE, &d.finish());
}

use wasm_bindgen::JsValue;

use rustex1_kit::mjs::Def;
use rustex1_kit::{js, mjs};

use crate::api;

pub const STORE: &str = "jobs";
pub const EXPR: &str = "$store.jobs";

fn redraw() {
    mjs::redraw();
}

fn notice(s: &JsValue, text: &str) {
    crate::components::validation_info::note("jobs-notice", "jobs", text);
    js::set_str(s, "notice", text);
}

fn load_jobs(_this: JsValue, _args: js_sys::Array) -> JsValue {
    api::jobs_get("jobs", "", |v| {
        let s = mjs::store_ref(STORE);
        js::set(&s, "jobs", &js::get(&v, "jobs"));
        redraw();
    });
    JsValue::UNDEFINED
}

fn load_runs(_this: JsValue, _args: js_sys::Array) -> JsValue {
    api::jobs_get("runs", "limit=60", |v| {
        let s = mjs::store_ref(STORE);
        js::set(&s, "runs", &js::get(&v, "runs"));
        redraw();
    });
    JsValue::UNDEFINED
}

fn runs_for_job(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let name = js::arg_str(&args, 0);
    let out = js_sys::Array::new();
    for run in js_sys::Array::from(&js::get(&s, "runs")).iter() {
        if js::get_str(&run, "jobName") == name {
            out.push(&run);
        }
    }
    out.into()
}

fn jobs_for_basedir(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let basedir = js::arg_str(&args, 0);
    let out = js_sys::Array::new();
    for job in js_sys::Array::from(&js::get(&s, "jobs")).iter() {
        if js::get_str(&job, "basedir") == basedir {
            out.push(&job);
        }
    }
    out.into()
}

fn basedirs(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let out = js_sys::Array::new();
    let mut seen: Vec<String> = Vec::new();
    for job in js_sys::Array::from(&js::get(&s, "jobs")).iter() {
        let basedir = js::get_str(&job, "basedir");
        if !seen.contains(&basedir) {
            seen.push(basedir.clone());
            out.push(&JsValue::from_str(&basedir));
        }
    }
    out.into()
}

fn runs_for_basedir(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let basedir = js::arg_str(&args, 0);
    let out = js_sys::Array::new();
    for run in js_sys::Array::from(&js::get(&s, "runs")).iter() {
        let label = js::get_str(&run, "label");
        if label.starts_with(&format!("{basedir}:")) {
            out.push(&run);
        }
    }
    out.into()
}

fn open_run(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    crate::components::aside_panels::close();
    let id = js::arg_str(&args, 0);
    js::set_str(&s, "runId", &id);
    js::set_bool(&s, "runOpen", true);
    js::set(&s, "run", &js::obj());
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
    refresh_run();
    redraw();
    JsValue::UNDEFINED
}

fn refresh_run() {
    let s = mjs::store_ref(STORE);
    let id = js::get_str(&s, "runId");
    if id.is_empty() {
        return;
    }
    api::jobs_get("run", &format!("id={id}&journal=60"), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get_str(&s, "runId") != id {
            return;
        }
        js::set(&s, "run", &js::get(&v, "run"));
        let tail = js::get(&v, "journalTail");
        let list = js_sys::Array::from(&tail);
        let mut rows = Vec::new();
        for entry in list.iter() {
            let row = entry.clone();
            let data = js::get(&entry, "data");
            let message = js::get_str(&entry, "message");
            js::set_str(&row, "messageHtml", &crate::helpers::markdown_to_html(&message));
            js::set(&row, "data", &data);
            rows.push(row);
        }
        let arr = js_sys::Array::new();
        for r in rows.iter().rev() {
            arr.push(r);
        }
        js::set(&s, "journal", &arr);
        redraw();
    });
}

fn poll_run(_this: JsValue, _args: js_sys::Array) -> JsValue {
    refresh_run();
    JsValue::UNDEFINED
}

fn close_run(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::set_bool(&s, "runOpen", false);
    redraw();
    JsValue::UNDEFINED
}

fn open_job(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    crate::components::aside_panels::close();
    let name = js::arg_str(&args, 0);
    js::set_str(&s, "jobName", &name);
    js::set_bool(&s, "jobOpen", true);
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
    redraw();
    JsValue::UNDEFINED
}

fn open_job_form(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    crate::components::aside_panels::close();
    js::set_bool(&s, "jobFormOpen", true);
    js::set_str(&s, "formError", "");
    js::set_bool(&mjs::store_ref("home"), "asideOpen", true);
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

fn register_job(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let path = js::get_str(&s, "formPath");
    if path.trim().is_empty() {
        js::set_str(&s, "formError", "a job needs the path to its Jobfile on disk");
        redraw();
        return JsValue::UNDEFINED;
    }
    let payload = js::obj();
    js::set_str(&payload, "path", &path);
    api::jobs_post("job-register", &payload.into(), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get(&v, "ok").as_bool() == Some(false) {
            let error = js::get_str(&v, "error");
            js::set_str(&s, "formError", &if error.is_empty() { "could not register the job".to_string() } else { error });
            redraw();
            return;
        }
        js::set_bool(&s, "jobFormOpen", false);
        js::set_str(&s, "formPath", "");
        load_jobs(JsValue::UNDEFINED, js_sys::Array::new());
    });
    redraw();
    JsValue::UNDEFINED
}

fn delete_job(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let name = js::arg_str(&args, 0);
    if js::get_str(&s, "deleteArmed") != name {
        js::set_str(&s, "deleteArmed", &name);
        notice(&s, "press delete again to unregister this job");
        redraw();
        return JsValue::UNDEFINED;
    }
    js::set_str(&s, "deleteArmed", "");
    let payload = js::obj();
    js::set_str(&payload, "name", &name);
    api::jobs_post("job-delete", &payload.into(), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get(&v, "ok").as_bool() == Some(false) {
            notice(&s, &js::get_str(&v, "error"));
        }
        js::set_bool(&s, "jobOpen", false);
        load_jobs(JsValue::UNDEFINED, js_sys::Array::new());
    });
    redraw();
    JsValue::UNDEFINED
}

fn toggle_job(_this: JsValue, args: js_sys::Array) -> JsValue {
    let name = js::arg_str(&args, 0);
    let enabled = js::arg_str(&args, 1) == "1";
    let payload = js::obj();
    js::set_str(&payload, "name", &name);
    js::set_bool(&payload, "enabled", !enabled);
    api::jobs_post("job-enable", &payload.into(), move |_| {
        load_jobs(JsValue::UNDEFINED, js_sys::Array::new());
    });
    JsValue::UNDEFINED
}

fn trigger_job(_this: JsValue, args: js_sys::Array) -> JsValue {
    let name = js::arg_str(&args, 0);
    let payload = js::obj();
    js::set_str(&payload, "name", &name);
    api::jobs_post("run-enqueue", &payload.into(), move |v| {
        let s = mjs::store_ref(STORE);
        if js::get(&v, "ok").as_bool() == Some(false) {
            notice(&s, &js::get_str(&v, "error"));
        } else {
            notice(&s, "run enqueued");
        }
        load_runs(JsValue::UNDEFINED, js_sys::Array::new());
    });
    JsValue::UNDEFINED
}

fn trigger_pr_check(_this: JsValue, args: js_sys::Array) -> JsValue {
    let name = js::arg_str(&args, 0);
    let repo = js::arg_str(&args, 1);
    let branch = js::arg_str(&args, 2);
    let pr_id = js::arg_str(&args, 3);
    let payload = js::obj();
    js::set_str(&payload, "name", &name);
    js::set_str(&payload, "repo", &repo);
    js::set_str(&payload, "branch", &branch);
    if let Ok(id) = pr_id.parse::<f64>() {
        js::set_num(&payload, "prId", id);
    }
    api::jobs_post("run-enqueue", &payload.into(), move |_| {
        load_runs(JsValue::UNDEFINED, js_sys::Array::new());
    });
    JsValue::UNDEFINED
}

fn answer_run(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let id = js::get_str(&s, "runId");
    let answer = js::get_str(&s, "answerDraft");
    if answer.trim().is_empty() {
        notice(&s, "type an answer first");
        redraw();
        return JsValue::UNDEFINED;
    }
    let payload = js::obj();
    js::set_str(&payload, "runId", &id);
    js::set_str(&payload, "answer", &answer);
    api::jobs_post("run-answer", &payload.into(), move |_| {
        let s = mjs::store_ref(STORE);
        js::set_str(&s, "answerDraft", "");
        refresh_run();
    });
    JsValue::UNDEFINED
}

fn stop_run(_this: JsValue, args: js_sys::Array) -> JsValue {
    let id = js::arg_str(&args, 0);
    let payload = js::obj();
    js::set_str(&payload, "runId", &id);
    js::set_bool(&payload, "force", false);
    api::jobs_post("run-stop", &payload.into(), move |_| {
        refresh_run();
        load_runs(JsValue::UNDEFINED, js_sys::Array::new());
    });
    JsValue::UNDEFINED
}

fn job_of(_this: JsValue, args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    let name = js::arg_str(&args, 0);
    for job in js_sys::Array::from(&js::get(&s, "jobs")).iter() {
        if js::get_str(&job, "name") == name {
            return job;
        }
    }
    js::obj().into()
}

fn run_label(_this: JsValue, args: js_sys::Array) -> JsValue {
    let run = js::arg(&args, 0);
    let label = js::get_str(&run, "label");
    if !label.is_empty() {
        return JsValue::from_str(&label);
    }
    JsValue::from_str(&format!(
        "{}#{}",
        js::get_str(&run, "jobName"),
        js::get_str(&run, "number")
    ))
}

fn run_active(_this: JsValue, args: js_sys::Array) -> JsValue {
    let run = js::arg(&args, 0);
    let state = js::get_str(&run, "state");
    JsValue::from_bool(matches!(
        state.as_str(),
        "queued" | "running" | "blocked" | "posting" | "stopping"
    ))
}

fn stages_of_run(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::get(&js::get(&s, "run"), "stages")
}

fn journal_of_run(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::get(&s, "journal")
}

fn pending_question(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    js::get(&js::get(&s, "run"), "pendingQuestion")
}

pub fn apply_topic(v: JsValue) {
    let s = mjs::store_ref(STORE);
    let jobs = js::get(&v, "jobs");
    if !js_sys::Array::is_array(&jobs) {
        return;
    }
    js::set(&s, "jobs", &jobs);
    js::set(&s, "runs", &js::get(&v, "runs"));
    redraw();
}

fn poll_tick(_this: JsValue, _args: js_sys::Array) -> JsValue {
    let s = mjs::store_ref(STORE);
    if js::get_bool(&s, "runOpen") {
        refresh_run();
    }
    JsValue::UNDEFINED
}

pub fn register() {
    let mut d = Def::new().named("store jobs");
    d.val("jobs", &js_sys::Array::new().into())
        .val("runs", &js_sys::Array::new().into())
        .val("run", &js::obj().into())
        .val("journal", &js_sys::Array::new().into())
        .str("runId", "")
        .str("jobName", "")
        .str("formPath", "")
        .str("formError", "")
        .str("answerDraft", "")
        .str("deleteArmed", "")
        .str("notice", "")
        .bool("runOpen", false)
        .bool("jobOpen", false)
        .bool("jobFormOpen", false)
        .method("loadJobs", load_jobs)
        .method("loadRuns", load_runs)
        .method("runsForJob", runs_for_job)
        .method("jobsForBasedir", jobs_for_basedir)
        .method("basedirs", basedirs)
        .method("runsForBasedir", runs_for_basedir)
        .method("openRun", open_run)
        .method("pollRun", poll_run)
        .method("closeRun", close_run)
        .method("openJob", open_job)
        .method("openJobForm", open_job_form)
        .method("formInput", form_input)
        .method("registerJob", register_job)
        .method("deleteJob", delete_job)
        .method("toggleJob", toggle_job)
        .method("triggerJob", trigger_job)
        .method("triggerPrCheck", trigger_pr_check)
        .method("answerRun", answer_run)
        .method("stopRun", stop_run)
        .method("jobOf", job_of)
        .method("runLabel", run_label)
        .method("runActive", run_active)
        .method("stagesOfRun", stages_of_run)
        .method("journalOfRun", journal_of_run)
        .method("pendingQuestion", pending_question)
        .method("pollTick", poll_tick);
    let store = mjs::store(STORE, &d.finish());
    let tick = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
        poll_tick(JsValue::UNDEFINED, js_sys::Array::new());
    }) as Box<dyn FnMut()>)
    .into_js_value();
    js::set(&store, "__pollTick", &tick);
    js::call(&js::global(), "setInterval", &[tick, JsValue::from_f64(2000.0)]);
}

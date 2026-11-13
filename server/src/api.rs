//! `/api/*` — angela, running **in this process**.
//!
//! This used to forward to a separate angela daemon over HTTP, which is what
//! you do when the agent half is written in another language. It is a Rust
//! crate now, so the chat app is one server: one process to start, one set of
//! logs, no port to coordinate, and no proxy hop to buffer a stream.
//!
//! The adapter below is the whole seam. angela's router writes through its
//! `ResponseSink`; this maps that onto the dev server's `Stream`, so the same
//! routes serve both here and from the standalone `angela-serve` binary.

use std::sync::Arc;

use angela::http::{Request as ARequest, ResponseSink};
use angela::serve::{AppState, Server as AngelaServer};
use rustex1_devserver::{Request, State, Stream};
use serde_json::Value;

/// Where sessions live. `ANGELA_ROOT` overrides; otherwise the directory the
/// server was started from, which is the chat project itself.
fn project_root() -> std::path::PathBuf {
    match std::env::var("ANGELA_ROOT") {
        Ok(v) if !v.trim().is_empty() => std::path::PathBuf::from(v.trim()),
        _ => std::env::current_dir().unwrap_or_else(|_| ".".into()),
    }
}

/// Built once: it owns the session store and the tokio runtime that turns
/// drive agl on. Rebuilding per request would spin up a runtime per call.
fn angela(port: u16) -> &'static AngelaServer {
    use std::sync::OnceLock;
    static SERVER: OnceLock<AngelaServer> = OnceLock::new();
    SERVER.get_or_init(|| {
        let root = project_root();
        // The reported port is this server's: embedded angela has no port of
        // its own, and `/api/health` saying 0 would just be a lie.
        let state = AppState::new(&root, port).expect("angela state");
        eprintln!("angela embedded · sessions in {}/.angela/sessions", root.display());
        AngelaServer::new(Arc::new(state))
    })
}

/// Bridges angela's `ResponseSink` onto the dev server's `Stream`.
struct DevSink<'s, 'a> {
    out: &'s mut Stream<'a>,
    streaming: bool,
    ended: bool,
}

impl<'s, 'a> ResponseSink for DevSink<'s, 'a> {
    fn json(&mut self, status: u16, body: &Value) {
        let _ = self.out.json(status, body);
        self.ended = true;
    }

    fn begin_ndjson(&mut self) {
        // The dev server adds its own chunked framing, so only the content
        // type is ours to set. `no-cache` matters: a buffering intermediary
        // is exactly what makes a token stream arrive as one lump.
        let _ = self.out.start(
            200,
            &[
                ("content-type", "application/x-ndjson"),
                ("cache-control", "no-cache"),
            ],
        );
        self.streaming = true;
    }

    fn row(&mut self, value: &Value) -> bool {
        let mut line = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
        line.push(b'\n');
        // One frame per row, written straight out — this is what lets the UI
        // show a token appearing instead of a finished paragraph.
        self.out.chunk(&line).is_ok()
    }

    fn end(&mut self) {
        if self.ended {
            return;
        }
        self.ended = true;
        let _ = self.out.finish();
    }
}

/// The dev server's raw `/api/` handler.
pub fn handle_api(state: &Arc<State>, req: &Request, out: &mut Stream<'_>) {
    let areq = ARequest {
        method: req.method.clone(),
        path: req.path.clone(),
        query: req.query.clone(),
        headers: req
            .headers
            .iter()
            .map(|(k, v)| (k.to_ascii_lowercase(), v.clone()))
            .collect(),
        body: req.body.clone(),
    };
    let mut sink = DevSink {
        out,
        streaming: false,
        ended: false,
    };
    angela(state.config.port).dispatch(&areq, &mut sink);
    sink.end();
}

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;
use serde_json::Value;
use sha2::{Digest, Sha256};

static LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    static IN_DB: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_vcs",
        r#"
CREATE TABLE repos (
  id             TEXT    PRIMARY KEY,
  name           TEXT    NOT NULL UNIQUE,
  path           TEXT    NOT NULL,
  description    TEXT    NOT NULL DEFAULT '',
  default_branch TEXT    NOT NULL DEFAULT 'main',
  settings       TEXT    NOT NULL DEFAULT '{}',
  created_at     TEXT    NOT NULL,
  updated_at     TEXT    NOT NULL
);

CREATE TABLE worktrees (
  id            TEXT PRIMARY KEY,
  repo_id       TEXT NOT NULL,
  branch        TEXT NOT NULL,
  path          TEXT NOT NULL,
  session_id    TEXT NOT NULL DEFAULT '',
  branch_pattern TEXT NOT NULL DEFAULT '',
  path_pattern  TEXT NOT NULL DEFAULT '',
  sandbox_id    TEXT NOT NULL DEFAULT '',
  created_at    TEXT NOT NULL
);

CREATE INDEX idx_worktrees_repo ON worktrees (repo_id, branch);
CREATE INDEX idx_worktrees_session ON worktrees (session_id);

CREATE TABLE prs (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  repo_id       TEXT    NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
  number        INTEGER NOT NULL,
  source_branch TEXT    NOT NULL,
  target_branch TEXT    NOT NULL,
  title         TEXT    NOT NULL,
  description   TEXT    NOT NULL DEFAULT '',
  author        TEXT,
  merged_by     TEXT,
  state         TEXT    NOT NULL CHECK (state IN ('open', 'review', 'approved', 'merged', 'rejected')),
  merge_commit  TEXT,
  created_at    TEXT    NOT NULL,
  updated_at    TEXT    NOT NULL,
  UNIQUE (repo_id, number)
);

CREATE INDEX idx_prs_repo_state ON prs (repo_id, state, number DESC);

CREATE TABLE reviews (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  pr_id      INTEGER NOT NULL REFERENCES prs(id) ON DELETE CASCADE,
  reviewer   TEXT    NOT NULL,
  decision   TEXT    NOT NULL CHECK (decision IN ('comment', 'approve', 'request_changes', 'reject')),
  comment    TEXT    NOT NULL DEFAULT '',
  created_at TEXT    NOT NULL
);

CREATE INDEX idx_reviews_pr ON reviews (pr_id, created_at, id);
CREATE INDEX idx_reviews_pr_decision ON reviews (pr_id, decision);

CREATE TABLE pr_attachments (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  pr_id      INTEGER NOT NULL REFERENCES prs(id) ON DELETE CASCADE,
  name       TEXT    NOT NULL,
  path       TEXT    NOT NULL,
  size       INTEGER NOT NULL DEFAULT 0,
  mime       TEXT    NOT NULL DEFAULT '',
  created_at TEXT    NOT NULL
);

CREATE INDEX idx_pr_attachments_pr ON pr_attachments (pr_id);
"#,
    ),
    (
        "0002_jobs",
        r#"
CREATE TABLE jobs (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  name        TEXT    NOT NULL UNIQUE,
  description TEXT    NOT NULL DEFAULT '',
  basedir     TEXT    NOT NULL DEFAULT '',
  cron        TEXT,
  env         TEXT    NOT NULL DEFAULT '{}',
  definition  TEXT    NOT NULL,
  source_kind TEXT    NOT NULL CHECK (source_kind IN ('path', 'inline')),
  source_path TEXT,
  source_sha  TEXT    NOT NULL,
  enabled     INTEGER NOT NULL DEFAULT 1,
  next_run_at INTEGER NOT NULL DEFAULT 0,
  last_run_at INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT    NOT NULL,
  updated_at  TEXT    NOT NULL
);

CREATE TABLE job_queue_items (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  job_id      INTEGER NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  source      TEXT    NOT NULL CHECK (source IN ('manual', 'webhook', 'cron', 'agent', 'trigger')),
  payload     TEXT    NOT NULL DEFAULT '{}',
  state       TEXT    NOT NULL CHECK (state IN ('queued', 'claimed', 'cancelled')),
  created_at  TEXT    NOT NULL,
  claimed_at  TEXT,
  updated_at  TEXT    NOT NULL
);

CREATE INDEX idx_job_queue_state ON job_queue_items (state, id);

CREATE TABLE runs (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  job_id         INTEGER NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  job_name       TEXT    NOT NULL,
  number         INTEGER NOT NULL,
  queue_item_id  INTEGER REFERENCES job_queue_items(id) ON DELETE CASCADE,
  trigger_source TEXT    NOT NULL CHECK (trigger_source IN ('manual', 'webhook', 'cron', 'agent', 'trigger')),
  state          TEXT    NOT NULL CHECK (state IN ('queued', 'running', 'blocked', 'posting', 'stopping', 'success', 'failure', 'aborted')),
  env            TEXT    NOT NULL DEFAULT '{}',
  error          TEXT,
  repo_id        TEXT,
  pr_id          INTEGER,
  created_at     TEXT    NOT NULL,
  started_at     TEXT,
  ended_at       TEXT,
  updated_at     TEXT    NOT NULL,
  UNIQUE (job_id, number)
);

CREATE INDEX idx_runs_job ON runs (job_id, number DESC);
CREATE INDEX idx_runs_state ON runs (state, id DESC);
CREATE INDEX idx_runs_pr ON runs (pr_id);

CREATE TABLE run_stages (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id      INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
  name        TEXT    NOT NULL,
  position    INTEGER NOT NULL,
  group_name  TEXT,
  state       TEXT    NOT NULL CHECK (state IN ('pending', 'provisioning', 'running', 'blocked', 'success', 'failure', 'aborted', 'skipped')),
  sandbox_id  TEXT,
  session_id  TEXT,
  artifacts   TEXT    NOT NULL DEFAULT '[]',
  error       TEXT,
  created_at  TEXT    NOT NULL,
  started_at  TEXT,
  ended_at    TEXT,
  updated_at  TEXT    NOT NULL,
  UNIQUE (run_id, name)
);

CREATE INDEX idx_run_stages_run ON run_stages (run_id, position);

CREATE TABLE journal_entries (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id     INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
  stage_name TEXT,
  kind       TEXT    NOT NULL CHECK (kind IN ('stage_start', 'stage_end', 'step_start', 'step_end', 'qa', 'artifact', 'note', 'error', 'tool', 'llm')),
  level      TEXT    NOT NULL DEFAULT 'info' CHECK (level IN ('debug', 'info', 'warn', 'error')),
  message    TEXT    NOT NULL DEFAULT '',
  data       TEXT    NOT NULL DEFAULT '{}',
  created_at TEXT    NOT NULL
);

CREATE INDEX idx_journal_run ON journal_entries (run_id, id);

CREATE TABLE triggers (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  job_id        INTEGER NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  job_name      TEXT    NOT NULL,
  source        TEXT    NOT NULL CHECK (source IN ('manual', 'webhook', 'cron', 'agent', 'trigger')),
  dedupe_key    TEXT,
  payload       TEXT    NOT NULL DEFAULT '{}',
  queue_item_id INTEGER REFERENCES job_queue_items(id) ON DELETE CASCADE,
  created_at    TEXT    NOT NULL,
  UNIQUE (job_id, dedupe_key)
);

CREATE INDEX idx_triggers_job ON triggers (job_id, created_at DESC, id DESC);
"#,
    ),
];

pub fn open(root: &Path) -> Result<Connection, String> {
    let path = root.join(".angela").join("conversations.db");
    let conn = Connection::open(&path).map_err(|e| e.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;").map_err(|e| e.to_string())?;
    Ok(conn)
}

pub fn with<T>(root: &Path, f: impl FnOnce(&Connection) -> T) -> Result<T, String> {
    let nested = IN_DB.with(|v| v.get());
    let guard = if nested {
        None
    } else {
        let g = LOCK.lock().map_err(|e| e.to_string())?;
        IN_DB.with(|v| v.set(true));
        Some(g)
    };
    let conn = open(root);
    let out = conn.map(|c| f(&c));
    if guard.is_some() {
        IN_DB.with(|v| v.set(false));
    }
    out
}

pub fn migrate(root: &Path) -> Result<(), String> {
    with(root, |conn| {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS chat_migrations (
                id         TEXT PRIMARY KEY,
                checksum   TEXT NOT NULL,
                applied_at TEXT NOT NULL
            );",
        )
        .map_err(|e| e.to_string())?;
        let mut known = std::collections::HashMap::new();
        {
            let mut st = conn
                .prepare("SELECT id, checksum FROM chat_migrations")
                .map_err(|e| e.to_string())?;
            let rows = st
                .query_map([], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })
                .map_err(|e| e.to_string())?;
            for row in rows.flatten() {
                known.insert(row.0, row.1);
            }
        }
        for (id, sql) in MIGRATIONS {
            let mut hasher = Sha256::new();
            hasher.update(sql.as_bytes());
            let checksum = format!("{:x}", hasher.finalize());
            match known.get(*id) {
                Some(prev) if *prev == checksum => continue,
                Some(_) => return Err(format!("chat migration checksum drift for {id}")),
                None => {}
            }
            conn.execute_batch("BEGIN IMMEDIATE;").map_err(|e| e.to_string())?;
            let done = conn.execute_batch(sql).and_then(|()| {
                conn.execute(
                    "INSERT INTO chat_migrations (id, checksum, applied_at) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id, checksum, now_iso()],
                )
                .map(|_| ())
            });
            match done {
                Ok(()) => {
                    conn.execute_batch("COMMIT;").map_err(|e| e.to_string())?;
                }
                Err(e) => {
                    let _ = conn.execute_batch("ROLLBACK;");
                    return Err(format!("chat migration {id} failed: {e}"));
                }
            }
        }
        Ok(())
    })?
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn parse_json(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or(Value::Null)
}

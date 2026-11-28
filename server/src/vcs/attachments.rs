use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::store;
use crate::err::ApiError;

fn guess_mime(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".mp4") {
        "video/mp4"
    } else if lower.ends_with(".webm") {
        "video/webm"
    } else if lower.ends_with(".mov") {
        "video/quicktime"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".md") {
        "text/markdown"
    } else if lower.ends_with(".txt") || lower.ends_with(".log") {
        "text/plain"
    } else if lower.ends_with(".json") {
        "application/json"
    } else if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        "application/yaml"
    } else {
        "application/octet-stream"
    }
}

fn safe_name(raw: &str) -> String {
    let base = Path::new(raw)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('.').to_string();
    if trimmed.is_empty() {
        "file".to_string()
    } else {
        trimmed.chars().take(120).collect()
    }
}

fn attachments_dir(root: &Path, pr_id: i64) -> PathBuf {
    root.join(".angela")
        .join("vcs")
        .join("attachments")
        .join(pr_id.to_string())
}

pub fn attach_from_path(
    root: &Path,
    c: &rusqlite::Connection,
    pr_id: i64,
    source: &str,
    name_override: Option<&str>,
) -> Result<Value, ApiError> {
    let src = PathBuf::from(source);
    let meta = std::fs::metadata(&src).map_err(|e| {
        ApiError::validation(format!("attachment '{source}' is not readable: {e}"))
    })?;
    if !meta.is_file() {
        return Err(ApiError::validation(format!(
            "attachment '{source}' is not a file"
        )));
    }
    let name = safe_name(name_override.unwrap_or(source));
    let dir = attachments_dir(root, pr_id);
    std::fs::create_dir_all(&dir).map_err(|e| ApiError::internal(e.to_string()))?;
    let dest = dir.join(&name);
    std::fs::copy(&src, &dest).map_err(|e| ApiError::internal(e.to_string()))?;
    let size = meta.len() as i64;
    let mime = guess_mime(&name).to_string();
    let path = dest.to_string_lossy().to_string();
    let id = store::insert_attachment(c, pr_id, &name, &path, size, &mime)?;
    Ok(json!({
        "id": id,
        "name": name,
        "size": size,
        "mime": mime,
    }))
}

pub fn attach_bytes(
    root: &Path,
    c: &rusqlite::Connection,
    pr_id: i64,
    name: &str,
    bytes: &[u8],
) -> Result<Value, ApiError> {
    let name = safe_name(name);
    let dir = attachments_dir(root, pr_id);
    std::fs::create_dir_all(&dir).map_err(|e| ApiError::internal(e.to_string()))?;
    let dest = dir.join(&name);
    std::fs::write(&dest, bytes).map_err(|e| ApiError::internal(e.to_string()))?;
    let mime = guess_mime(&name).to_string();
    let path = dest.to_string_lossy().to_string();
    let id = store::insert_attachment(c, pr_id, &name, &path, bytes.len() as i64, &mime)?;
    Ok(json!({
        "id": id,
        "name": name,
        "size": bytes.len(),
        "mime": mime,
    }))
}

pub fn read_attachment(
    root: &Path,
    c: &rusqlite::Connection,
    attachment_id: i64,
) -> Result<(String, Vec<u8>), ApiError> {
    let _ = root;
    let (name, path) = c
        .query_row(
            "SELECT name, path FROM pr_attachments WHERE id = ?1",
            [attachment_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .map_err(|_| ApiError::not_found("attachment_not_found", "attachment not found"))?;
    let bytes = std::fs::read(&path).map_err(|e| ApiError::internal(e.to_string()))?;
    Ok((name, bytes))
}

pub fn delete_pr_files(root: &Path, c: &rusqlite::Connection, pr_id: i64) {
    let _ = root;
    let paths: Vec<String> = {
        let mut st = c
            .prepare("SELECT path FROM pr_attachments WHERE pr_id = ?1")
            .unwrap();
        st.query_map([pr_id], |r| r.get::<_, String>(0))
            .unwrap()
            .flatten()
            .collect()
    };
    for p in &paths {
        let _ = std::fs::remove_file(p);
    }
    if let Some(parent) = paths
        .first()
        .and_then(|p| Path::new(p).parent().map(|d| d.to_path_buf()))
    {
        let _ = std::fs::remove_dir_all(parent);
    }
}

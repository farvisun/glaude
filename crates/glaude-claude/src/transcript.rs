//! Claude Code transcripts on disk: listing sessions and loading history.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use glaude_core::model::{ConvEvent, Item};
use serde::Serialize;
use serde_json::Value;

use crate::translate::Translator;

const HEAD_BYTES: u64 = 96 * 1024;
const TAIL_BYTES: u64 = 128 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub provider: &'static str,
    pub id: String,
    pub cwd: String,
    pub title: String,
    /// Unix ms of the last write.
    pub updated: u64,
    /// Unix ms the session started (file creation; the last write if unknown).
    pub created: u64,
    pub path: String,
}

/// The folder name Claude Code uses for a project path.
pub fn encode_cwd(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

fn read_head(path: &Path) -> std::io::Result<String> {
    let mut f = File::open(path)?;
    let mut buf = Vec::new();
    f.by_ref().take(HEAD_BYTES).read_to_end(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn read_tail(path: &Path, len: u64) -> std::io::Result<String> {
    let mut f = File::open(path)?;
    let start = len.saturating_sub(TAIL_BYTES);
    f.seek(SeekFrom::Start(start))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf).into_owned();
    // Drop the partial first line when we started mid-file.
    Ok(if start > 0 {
        text.split_once('\n')
            .map(|(_, r)| r.to_string())
            .unwrap_or_default()
    } else {
        text
    })
}

fn prompt_text(v: &Value) -> Option<String> {
    if v.get("type")?.as_str()? != "user" || v.get("isMeta").and_then(Value::as_bool) == Some(true)
    {
        return None;
    }
    let content = v.get("message")?.get("content")?;
    let text = match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .find(|p| p.get("type").and_then(Value::as_str) == Some("text"))
            .and_then(|p| p.get("text")?.as_str())?
            .to_string(),
        _ => return None,
    };
    let t = text.trim();
    if t.is_empty() || t.starts_with('<') || t.starts_with("Caveat:") {
        return None;
    }
    Some(t.chars().take(160).collect())
}

/// Summarize one transcript file. `None` for sidechain or empty files.
pub fn summarize(path: &Path) -> Option<SessionSummary> {
    let meta = path.metadata().ok()?;
    let id = path.file_stem()?.to_str()?.to_string();
    let head = read_head(path).ok()?;
    let mut cwd = String::new();
    let mut first_prompt = None;
    for (i, line) in head.lines().enumerate() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if i == 0 && v.get("isSidechain").and_then(Value::as_bool) == Some(true) {
            return None;
        }
        if cwd.is_empty() {
            if let Some(c) = v.get("cwd").and_then(Value::as_str) {
                cwd = c.to_string();
            }
        }
        if first_prompt.is_none() {
            first_prompt = prompt_text(&v);
        }
        if !cwd.is_empty() && first_prompt.is_some() {
            break;
        }
    }
    let tail = read_tail(path, meta.len()).ok()?;
    let (mut custom, mut ai, mut last_prompt, mut summary) = (None, None, None, None);
    for line in tail.lines() {
        if !(line.contains("-title")
            || line.contains("last-prompt")
            || line.contains("\"summary\""))
        {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let get = |k: &str| {
            v.get(k)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        match v.get("type").and_then(Value::as_str) {
            Some("custom-title") => custom = get("customTitle").or(custom),
            Some("ai-title") => ai = get("aiTitle").or(ai),
            Some("last-prompt") => last_prompt = get("lastPrompt").or(last_prompt),
            Some("summary") => summary = get("summary").or(summary),
            _ => {}
        }
    }
    let custom_file = path.with_extension("").join("custom-title.json");
    let file_title = std::fs::read_to_string(custom_file)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| {
            v.get("customTitle")
                .or(v.get("title"))
                .and_then(Value::as_str)
                .map(str::to_string)
        });
    let title = custom
        .or(file_title)
        .or(ai)
        .or(summary)
        .or(last_prompt)
        .or(first_prompt)?;
    let updated = meta
        .modified()
        .ok()
        .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as u64);
    let created = meta
        .created()
        .ok()
        .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
        .map_or(updated, |d| d.as_millis() as u64);
    Some(SessionSummary {
        provider: "claude",
        id,
        cwd,
        title,
        updated,
        created,
        path: path.display().to_string(),
    })
}

/// All sessions under `<config>/projects`, newest first.
pub fn list_sessions(config_dir: &Path, limit: usize) -> Vec<SessionSummary> {
    let root = config_dir.join("projects");
    let mut files: Vec<(u64, PathBuf)> = Vec::new();
    let Ok(dirs) = std::fs::read_dir(&root) else {
        return Vec::new();
    };
    for d in dirs.flatten() {
        let Ok(entries) = std::fs::read_dir(d.path()) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("jsonl") {
                continue;
            }
            let mtime = e
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_millis() as u64);
            files.push((mtime, p));
        }
    }
    files.sort_by_key(|f| std::cmp::Reverse(f.0));
    files
        .into_iter()
        .filter_map(|(_, p)| summarize(&p))
        .take(limit)
        .collect()
}

/// Find the transcript of a session id.
pub fn find_transcript(config_dir: &Path, cwd: &str, session_id: &str) -> Option<PathBuf> {
    let direct = config_dir
        .join("projects")
        .join(encode_cwd(cwd))
        .join(format!("{session_id}.jsonl"));
    if direct.exists() {
        return Some(direct);
    }
    let root = config_dir.join("projects");
    std::fs::read_dir(root).ok()?.flatten().find_map(|d| {
        let p = d.path().join(format!("{session_id}.jsonl"));
        p.exists().then_some(p)
    })
}

/// Replay a transcript into conversation items.
pub fn load_items(path: &Path, cwd: &str) -> anyhow::Result<Vec<Item>> {
    let reader = BufReader::new(File::open(path)?);
    let mut t = Translator::for_history(cwd);
    let mut items: Vec<Item> = Vec::new();
    let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for line in reader.lines() {
        let line = line?;
        if !(line.starts_with("{\"")
            && (line.contains("\"type\":\"user\"")
                || line.contains("\"type\":\"assistant\"")
                || line.contains("\"type\":\"system\"")))
        {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if v.get("isSidechain").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        for ev in t.feed(&v).events {
            if let ConvEvent::Item { item } = ev {
                let id = item.id().to_string();
                match index.get(&id) {
                    Some(&i) => items[i] = item,
                    None => {
                        index.insert(id, items.len());
                        items.push(item);
                    }
                }
            }
        }
    }
    Ok(items)
}

/// The message a user message replied to (`parentUuid`), to rewind the
/// conversation to just before it. `Some(None)`: it was the first message.
pub fn parent_of(path: &Path, uuid: &str) -> Option<Option<String>> {
    let text = std::fs::read_to_string(path).ok()?;
    let needle = format!("\"uuid\":\"{uuid}\"");
    for line in text.lines().filter(|l| l.contains(&needle)) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v.get("uuid").and_then(|u| u.as_str()) == Some(uuid) {
            return Some(
                v.get("parentUuid")
                    .and_then(|p| p.as_str())
                    .map(str::to_string),
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_cwd_like_claude() {
        assert_eq!(encode_cwd("/Users/x/Projects/a.b"), "-Users-x-Projects-a-b");
    }

    #[test]
    fn summarizes_and_loads_a_transcript() {
        let dir = std::env::temp_dir().join(format!("glaude-tr-{}", std::process::id()));
        let proj = dir.join("projects").join("-tmp-p");
        std::fs::create_dir_all(&proj).unwrap();
        let file = proj.join("abc.jsonl");
        let lines = [
            r#"{"type":"user","cwd":"/tmp/p","sessionId":"abc","uuid":"u1","message":{"role":"user","content":"سلام، فایل را بخوان"}}"#,
            r#"{"type":"assistant","cwd":"/tmp/p","message":{"id":"m1","role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/tmp/p/a.txt"}}]}}"#,
            r#"{"type":"user","cwd":"/tmp/p","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"hi"}]}}"#,
            r#"{"type":"assistant","cwd":"/tmp/p","message":{"id":"m2","role":"assistant","content":[{"type":"text","text":"فایل نوشته: **hi**"}]}}"#,
            r#"{"type":"ai-title","aiTitle":"خواندن فایل","sessionId":"abc"}"#,
        ];
        std::fs::write(&file, lines.join("\n") + "\n").unwrap();
        let s = summarize(&file).unwrap();
        assert_eq!(s.title, "خواندن فایل");
        assert_eq!(s.cwd, "/tmp/p");
        let items = load_items(&file, "/tmp/p").unwrap();
        assert_eq!(items.len(), 3, "{items:?}");
        assert!(matches!(
            &items[1],
            Item::Tool {
                status: glaude_core::model::Status::Done,
                ..
            }
        ));
        assert_eq!(find_transcript(&dir, "/tmp/p", "abc").unwrap(), file);
        std::fs::remove_dir_all(dir).ok();
    }
}

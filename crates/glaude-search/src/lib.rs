//! History search: an SQLite FTS5 index of prompts and assistant text from
//! Claude transcripts and Codex rollouts. Incremental per file (offset,
//! size, mtime). Persian text is normalized the same way on both sides
//! (ZWNJ → space, Arabic yeh/kaf → Persian, Arabic/Persian digits → ASCII),
//! one character for one character, so match positions map back to the
//! original text for snippets.

use std::fs;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::Result;
use memchr::memmem::Finder;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;

const SCHEMA_VERSION: &str = "1";
const MAX_TEXT: usize = 16 * 1024;
/// Lines longer than this are not indexed (see `glaude_core::lines`).
const LINE_CAP: usize = 1 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Claude,
    Codex,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Claude => "claude",
            Kind::Codex => "codex",
        }
    }
}

/// A folder of session files to index.
#[derive(Debug, Clone)]
pub struct Source {
    pub kind: Kind,
    pub root: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub provider: &'static str,
    pub session_id: String,
    pub cwd: String,
    pub role: String,
    /// Unix ms.
    pub ts: i64,
    /// Escaped HTML with `<mark>` around matches.
    pub snippet: String,
    /// Matching messages in this session.
    pub matches: u32,
}

/// Map one char to its search form (always exactly one char).
fn norm_char(c: char) -> char {
    match c {
        '\u{200C}' | '\u{200D}' | '\u{00A0}' => ' ',
        '\u{064A}' | '\u{0649}' => '\u{06CC}',
        '\u{0643}' => '\u{06A9}',
        '\u{0623}' | '\u{0625}' => '\u{0627}',
        '\u{06C0}' => '\u{0647}',
        '\u{0660}'..='\u{0669}' => char::from(b'0' + (c as u32 - 0x0660) as u8),
        '\u{06F0}'..='\u{06F9}' => char::from(b'0' + (c as u32 - 0x06F0) as u8),
        c => c.to_lowercase().next().unwrap_or(c),
    }
}

pub fn normalize(s: &str) -> String {
    s.chars().map(norm_char).collect()
}

/// Query terms after normalization (quotes and FTS syntax stripped).
fn terms(q: &str) -> Vec<String> {
    normalize(q)
        .split(|c: char| c.is_whitespace() || matches!(c, '"' | '*' | '(' | ')' | ':' | '^'))
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .take(12)
        .collect()
}

/// Escaped snippet around the first match, matches wrapped in `<mark>`.
fn snippet(text: &str, terms: &[String]) -> String {
    let orig: Vec<char> = text.chars().collect();
    let low: Vec<char> = orig.iter().map(|c| norm_char(*c)).collect();
    let tchars: Vec<Vec<char>> = terms.iter().map(|t| t.chars().collect()).collect();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < low.len() {
        let mut hit = None;
        for t in &tchars {
            if !t.is_empty() && i + t.len() <= low.len() && low[i..i + t.len()] == t[..] {
                // Prefix match: only at a word start.
                if i == 0 || !low[i - 1].is_alphanumeric() {
                    hit = Some(t.len().max(hit.unwrap_or(0)));
                }
            }
        }
        if let Some(len) = hit {
            spans.push((i, i + len));
            i += len;
        } else {
            i += 1;
        }
    }
    const BEFORE: usize = 60;
    const LEN: usize = 220;
    let start = spans
        .first()
        .map(|s| s.0.saturating_sub(BEFORE))
        .unwrap_or(0);
    let start = (start..spans.first().map(|s| s.0).unwrap_or(0))
        .find(|&k| k == 0 || orig[k - 1].is_whitespace())
        .unwrap_or(start);
    let end = (start + LEN).min(orig.len());
    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    let mut k = start;
    for (a, b) in spans
        .iter()
        .copied()
        .filter(|(a, _)| *a >= start && *a < end)
    {
        push_escaped(&mut out, &orig[k..a]);
        out.push_str("<mark>");
        push_escaped(&mut out, &orig[a..b.min(orig.len())]);
        out.push_str("</mark>");
        k = b.min(orig.len());
    }
    if k < end {
        push_escaped(&mut out, &orig[k..end]);
    }
    if end < orig.len() {
        out.push('…');
    }
    out
}

fn push_escaped(out: &mut String, chars: &[char]) {
    for c in chars {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\n' | '\r' | '\t' => out.push(' '),
            c => out.push(*c),
        }
    }
}

fn ts_ms(v: Option<&Value>) -> i64 {
    match v {
        Some(Value::String(s)) => chrono::DateTime::parse_from_rfc3339(s)
            .map(|d| d.timestamp_millis())
            .unwrap_or(0),
        Some(Value::Number(n)) => {
            let f = n.as_f64().unwrap_or(0.0);
            if f > 1e11 {
                f as i64
            } else {
                (f * 1000.0) as i64
            }
        }
        _ => 0,
    }
}

fn text_parts(content: Option<&Value>, kinds: &[&str]) -> String {
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter(|p| {
                p.get("type")
                    .and_then(Value::as_str)
                    .is_some_and(|t| kinds.contains(&t))
            })
            .filter_map(|p| p.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// Prompt text that is really tool plumbing, not something the user typed.
fn is_plumbing(t: &str) -> bool {
    let t = t.trim_start();
    t.is_empty()
        || t.starts_with("<command-")
        || t.starts_with("<local-command")
        || t.starts_with("<system-reminder>")
        || t.starts_with("Caveat:")
        || t.starts_with("[Request interrupted")
        || t.starts_with("# AGENTS.md")
        || t.starts_with("<environment_context>")
        || t.starts_with("<user_instructions>")
}

struct Msg {
    role: &'static str,
    ts: i64,
    text: String,
}

struct Finders {
    user: Finder<'static>,
    assistant: Finder<'static>,
    tool_result: Finder<'static>,
    text: Finder<'static>,
    item_completed: Finder<'static>,
    user_message: Finder<'static>,
    agent_message: Finder<'static>,
    session_meta: Finder<'static>,
    user_item: Finder<'static>,
    agent_item: Finder<'static>,
}

impl Finders {
    fn new() -> Self {
        Self {
            user: Finder::new(b"\"type\":\"user\""),
            assistant: Finder::new(b"\"type\":\"assistant\""),
            tool_result: Finder::new(b"\"tool_result\""),
            text: Finder::new(b"\"text\""),
            item_completed: Finder::new(b"\"item_completed\""),
            user_message: Finder::new(b"\"user_message\""),
            agent_message: Finder::new(b"\"agent_message\""),
            session_meta: Finder::new(b"\"session_meta\""),
            user_item: Finder::new(b"\"UserMessage\""),
            agent_item: Finder::new(b"\"AgentMessage\""),
        }
    }
}

/// What a line contributes: messages and file metadata.
fn parse_claude(line: &[u8], f: &Finders, meta: &mut FileMeta) -> Option<Msg> {
    let is_user = f.user.find(line).is_some();
    let is_assistant = !is_user && f.assistant.find(line).is_some();
    if !(is_user || is_assistant) {
        return None;
    }
    if is_user && f.tool_result.find(line).is_some() {
        return None;
    }
    if is_assistant && f.text.find(line).is_none() {
        return None;
    }
    let v: Value = serde_json::from_slice(line).ok()?;
    if meta.cwd.is_empty() {
        if let Some(c) = v.get("cwd").and_then(Value::as_str) {
            meta.cwd = c.to_string();
        }
    }
    if meta.session_id.is_empty() {
        if let Some(s) = v.get("sessionId").and_then(Value::as_str) {
            meta.session_id = s.to_string();
        }
    }
    if v.get("isMeta").and_then(Value::as_bool) == Some(true)
        || v.get("isCompactSummary").and_then(Value::as_bool) == Some(true)
        || v.get("isSidechain").and_then(Value::as_bool) == Some(true)
    {
        return None;
    }
    let content = v.get("message").and_then(|m| m.get("content"));
    let text = text_parts(content, &["text"]);
    if is_user && is_plumbing(&text) || text.trim().is_empty() {
        return None;
    }
    Some(Msg {
        role: if is_user { "user" } else { "assistant" },
        ts: ts_ms(v.get("timestamp")),
        text,
    })
}

fn parse_codex(line: &[u8], f: &Finders, meta: &mut FileMeta) -> Option<Msg> {
    let head = &line[..line.len().min(4096)];
    if meta.session_id.is_empty() && f.session_meta.find(head).is_some() {
        // This line can be many MB (instructions); read id/cwd from its start.
        let start = &line[..line.len().min(16 * 1024)];
        if let Some(id) = json_str_field(start, "id") {
            meta.session_id = id;
        }
        if let Some(cwd) = json_str_field(start, "cwd") {
            meta.cwd = cwd;
        }
        return None;
    }
    // Only chat items; other items can carry megabytes (command output,
    // generated images) that are not worth parsing. The item type sits
    // near the start of the line.
    let item = f.item_completed.find(head).is_some()
        && (f.user_item.find(head).is_some() || f.agent_item.find(head).is_some());
    let legacy =
        !item && (f.user_message.find(head).is_some() || f.agent_message.find(head).is_some());
    if !item && !legacy {
        return None;
    }
    let v: Value = serde_json::from_slice(line).ok()?;
    let p = v.get("payload")?;
    let ts = ts_ms(v.get("timestamp"));
    let (role, text) = match p.get("type").and_then(Value::as_str)? {
        "item_completed" => {
            let it = p.get("item")?;
            match it.get("type").and_then(Value::as_str)? {
                "UserMessage" => (
                    "user",
                    text_parts(it.get("content"), &["text", "Text", "input_text"]),
                ),
                "AgentMessage" => (
                    "assistant",
                    text_parts(it.get("content"), &["Text", "text", "output_text"]),
                ),
                _ => return None,
            }
        }
        "user_message" => (
            "user",
            p.get("message").and_then(Value::as_str)?.to_string(),
        ),
        "agent_message" => (
            "assistant",
            p.get("message").and_then(Value::as_str)?.to_string(),
        ),
        _ => return None,
    };
    if text.trim().is_empty() || role == "user" && is_plumbing(&text) {
        return None;
    }
    Some(Msg { role, ts, text })
}

/// First `"key":"value"` string in a JSON prefix, unescaped.
fn json_str_field(bytes: &[u8], key: &str) -> Option<String> {
    let pat = format!("\"{key}\":\"");
    let at = memchr::memmem::find(bytes, pat.as_bytes())? + pat.len() - 1;
    let rest = &bytes[at..];
    let mut i = 1;
    while i < rest.len() {
        match rest[i] {
            b'\\' => i += 2,
            b'"' => return serde_json::from_slice::<String>(&rest[..=i]).ok(),
            _ => i += 1,
        }
    }
    None
}

#[derive(Default)]
struct FileMeta {
    session_id: String,
    cwd: String,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub files: u64,
    pub scanned: u64,
    pub messages: u64,
}

pub struct Index {
    conn: Mutex<Connection>,
}

impl Index {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(p) = path.parent() {
            fs::create_dir_all(p)?;
        }
        let conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS meta(k TEXT PRIMARY KEY, v TEXT);",
        )?;
        let version: Option<String> = conn
            .query_row("SELECT v FROM meta WHERE k='schema'", [], |r| r.get(0))
            .optional()?;
        if version.as_deref() != Some(SCHEMA_VERSION) {
            conn.execute_batch(
                "DROP TABLE IF EXISTS docs; DROP TABLE IF EXISTS msgs; DROP TABLE IF EXISTS files;",
            )?;
        }
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS files(
                id INTEGER PRIMARY KEY, path TEXT UNIQUE NOT NULL, provider TEXT NOT NULL,
                session_id TEXT NOT NULL DEFAULT '', cwd TEXT NOT NULL DEFAULT '',
                size INTEGER NOT NULL DEFAULT 0, mtime INTEGER NOT NULL DEFAULT 0, offset INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE IF NOT EXISTS msgs(
                id INTEGER PRIMARY KEY, file_id INTEGER NOT NULL, role TEXT NOT NULL, ts INTEGER NOT NULL, text TEXT NOT NULL);
             CREATE INDEX IF NOT EXISTS msgs_file ON msgs(file_id);
             CREATE VIRTUAL TABLE IF NOT EXISTS docs USING fts5(
                norm, content='', contentless_delete=1, tokenize='unicode61 remove_diacritics 2');",
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO meta(k, v) VALUES('schema', ?1)",
            [SCHEMA_VERSION],
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Index what changed since the last run. `progress` gets 0–1 by bytes.
    pub fn update(&self, sources: &[Source], progress: &mut dyn FnMut(f32)) -> Result<Stats> {
        let mut files: Vec<(Kind, PathBuf, fs::Metadata)> = Vec::new();
        for s in sources {
            if !s.root.is_dir() {
                continue;
            }
            for e in walkdir::WalkDir::new(&s.root)
                .follow_links(true)
                .into_iter()
                .filter_entry(|e| {
                    let n = e.file_name().to_string_lossy();
                    e.depth() == 0 || !(n.starts_with('.') || n == "subagents")
                })
                .flatten()
            {
                if e.file_type().is_file() && e.file_name().to_string_lossy().ends_with(".jsonl") {
                    if let Ok(m) = e.metadata() {
                        files.push((s.kind, e.path().to_path_buf(), m));
                    }
                }
            }
        }
        let mut stats = Stats {
            files: files.len() as u64,
            ..Default::default()
        };
        // Work out what to read first, for progress.
        let mut todo = Vec::new();
        let mut total = 0u64;
        {
            let conn = self.conn.lock();
            let mut q =
                conn.prepare_cached("SELECT size, mtime, offset FROM files WHERE path = ?1")?;
            for (kind, path, m) in &files {
                let key = path.to_string_lossy().to_string();
                let mtime = mtime(m);
                let row: Option<(i64, i64, i64)> = q
                    .query_row([&key], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                    .optional()?;
                let size = m.len() as i64;
                match row {
                    Some((s, t, _)) if s == size && t == mtime => {}
                    Some((_, _, off)) if size >= off => {
                        total += (size - off) as u64;
                        todo.push((*kind, path.clone(), off, false));
                    }
                    Some(_) => {
                        total += size as u64;
                        todo.push((*kind, path.clone(), 0, true));
                    }
                    None => {
                        total += size as u64;
                        todo.push((*kind, path.clone(), 0, false));
                    }
                }
            }
        }
        let finders = Finders::new();
        let mut done = 0u64;
        for (kind, path, offset, reset) in todo {
            let read = self.index_file(kind, &path, offset, reset, &finders, &mut stats)?;
            done += read;
            stats.scanned += 1;
            if total > 0 {
                progress((done as f32 / total as f32).min(1.0));
            }
        }
        // Forget files that disappeared.
        {
            let conn = self.conn.lock();
            let known: Vec<(i64, String)> = conn
                .prepare("SELECT id, path FROM files")?
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .flatten()
                .collect();
            for (id, p) in known {
                if !Path::new(&p).exists() {
                    drop_file(&conn, id)?;
                }
            }
        }
        progress(1.0);
        Ok(stats)
    }

    fn index_file(
        &self,
        kind: Kind,
        path: &Path,
        offset: i64,
        reset: bool,
        f: &Finders,
        stats: &mut Stats,
    ) -> Result<u64> {
        let Ok(meta) = fs::metadata(path) else {
            return Ok(0);
        };
        let mut file = BufReader::with_capacity(1 << 20, fs::File::open(path)?);
        let key = path.to_string_lossy().to_string();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let existing: Option<(i64, String, String)> = tx
            .query_row(
                "SELECT id, session_id, cwd FROM files WHERE path = ?1",
                [&key],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let mut fmeta = FileMeta::default();
        let file_id = match existing {
            Some((id, sid, cwd)) => {
                if reset {
                    drop_msgs(&tx, id)?;
                } else {
                    fmeta.session_id = sid;
                    fmeta.cwd = cwd;
                }
                id
            }
            None => {
                tx.execute(
                    "INSERT INTO files(path, provider) VALUES(?1, ?2)",
                    params![key, kind.as_str()],
                )?;
                tx.last_insert_rowid()
            }
        };
        let start = if reset { 0 } else { offset.max(0) as u64 };
        file.seek(SeekFrom::Start(start))?;
        let mut pos = start;
        let mut line = Vec::with_capacity(64 * 1024);
        {
            let mut ins_msg = tx.prepare_cached(
                "INSERT INTO msgs(file_id, role, ts, text) VALUES(?1, ?2, ?3, ?4)",
            )?;
            let mut ins_doc = tx.prepare_cached("INSERT INTO docs(rowid, norm) VALUES(?1, ?2)")?;
            loop {
                let l = glaude_core::lines::read_line_capped(&mut file, &mut line, LINE_CAP)?;
                if !l.complete {
                    break; // EOF, or a line still being written
                }
                pos += l.consumed as u64;
                // Oversized lines are images or tool output; only a Codex
                // session_meta prefix is still useful.
                if l.truncated && kind == Kind::Claude {
                    continue;
                }
                let msg = match kind {
                    Kind::Claude => parse_claude(&line, f, &mut fmeta),
                    Kind::Codex => parse_codex(&line, f, &mut fmeta),
                };
                if let Some(mut m) = msg {
                    if m.text.len() > MAX_TEXT {
                        let mut cut = MAX_TEXT;
                        while !m.text.is_char_boundary(cut) {
                            cut -= 1;
                        }
                        m.text.truncate(cut);
                    }
                    ins_msg.execute(params![file_id, m.role, m.ts, m.text])?;
                    let id = tx.last_insert_rowid();
                    ins_doc.execute(params![id, normalize(&m.text)])?;
                    stats.messages += 1;
                }
            }
        }
        if fmeta.session_id.is_empty() {
            fmeta.session_id = session_from_name(kind, path);
        }
        tx.execute(
            "UPDATE files SET session_id=?2, cwd=?3, size=?4, mtime=?5, offset=?6 WHERE id=?1",
            params![
                file_id,
                fmeta.session_id,
                fmeta.cwd,
                meta.len() as i64,
                mtime(&meta),
                pos as i64
            ],
        )?;
        tx.commit()?;
        Ok(pos - start)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Hit>> {
        let terms = terms(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let fts = terms
            .iter()
            .map(|t| format!("\"{}\"*", t.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" ");
        let conn = self.conn.lock();
        let mut stmt = conn.prepare_cached(
            "SELECT f.provider, f.session_id, f.cwd, m.role, m.ts, m.text
             FROM docs JOIN msgs m ON m.id = docs.rowid JOIN files f ON f.id = m.file_id
             WHERE docs MATCH ?1 ORDER BY m.ts DESC LIMIT 3000",
        )?;
        let rows = stmt.query_map([&fts], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, String>(5)?,
            ))
        })?;
        let mut hits: Vec<Hit> = Vec::new();
        for row in rows.flatten() {
            let (provider, session_id, cwd, role, ts, text) = row;
            if let Some(h) = hits
                .iter_mut()
                .find(|h| h.session_id == session_id && h.provider == provider)
            {
                h.matches += 1;
                continue;
            }
            if hits.len() >= limit {
                continue;
            }
            hits.push(Hit {
                provider: if provider == "codex" {
                    "codex"
                } else {
                    "claude"
                },
                session_id,
                cwd,
                role,
                ts,
                snippet: snippet(&text, &terms),
                matches: 1,
            });
        }
        Ok(hits)
    }

    pub fn message_count(&self) -> u64 {
        self.conn
            .lock()
            .query_row("SELECT COUNT(*) FROM msgs", [], |r| r.get::<_, i64>(0))
            .map(|n| n as u64)
            .unwrap_or(0)
    }
}

fn mtime(m: &fs::Metadata) -> i64 {
    m.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

fn session_from_name(kind: Kind, path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    match kind {
        Kind::Claude => stem,
        // rollout-<date>-<uuid>
        Kind::Codex => stem
            .get(stem.len().saturating_sub(36)..)
            .unwrap_or(&stem)
            .to_string(),
    }
}

fn drop_msgs(conn: &Connection, file_id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM docs WHERE rowid IN (SELECT id FROM msgs WHERE file_id = ?1)",
        [file_id],
    )?;
    conn.execute("DELETE FROM msgs WHERE file_id = ?1", [file_id])?;
    Ok(())
}

fn drop_file(conn: &Connection, file_id: i64) -> rusqlite::Result<()> {
    drop_msgs(conn, file_id)?;
    conn.execute("DELETE FROM files WHERE id = ?1", [file_id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_fields_from_a_prefix() {
        let line = br#"{"type":"session_meta","payload":{"session_id":"x","id":"abc","cwd":"/a \"b\"","base_instructions":"#;
        assert_eq!(json_str_field(line, "id").as_deref(), Some("abc"));
        assert_eq!(json_str_field(line, "cwd").as_deref(), Some(r#"/a "b""#));
        assert_eq!(json_str_field(line, "nope"), None);
    }

    #[test]
    fn normalization_is_one_to_one() {
        let s = "مي‌خواهم كتاب ۱۲۳ را";
        assert_eq!(s.chars().count(), normalize(s).chars().count());
        assert_eq!(normalize(s), "می خواهم کتاب 123 را");
    }

    #[test]
    fn snippet_marks_original_text() {
        let s = snippet("سلام، من مي‌خواهم یک Rust app بسازم", &terms("میخواهم rust"));
        // "می‌خواهم" (with ZWNJ) is two terms after normalization; "rust" matches.
        assert!(s.contains("<mark>Rust</mark>"), "{s}");
        let s2 = snippet("کتاب‌ها را بخوان", &terms("كتاب"));
        assert!(s2.starts_with("<mark>کتاب</mark>"), "{s2}");
    }

    fn write_lines(path: &Path, lines: &[Value]) {
        let body: String = lines.iter().map(|l| format!("{l}\n")).collect();
        fs::write(path, body).unwrap();
    }

    #[test]
    fn indexes_both_tools_incrementally() {
        let dir = std::env::temp_dir().join(format!("glaude-search-{}", std::process::id()));
        let claude = dir.join("claude/projects/-p");
        let codex = dir.join("codex/sessions/2026/11/22");
        fs::create_dir_all(&claude).unwrap();
        fs::create_dir_all(&codex).unwrap();
        let cf = claude.join("s1.jsonl");
        write_lines(
            &cf,
            &[
                json!({"type": "user", "sessionId": "s1", "cwd": "/p", "timestamp": "2026-11-22T10:00:00Z", "message": {"role": "user", "content": "چطور كتاب‌خانه را نصب کنم؟"}}),
                json!({"type": "user", "isMeta": true, "message": {"content": "<command-name>/clear</command-name>"}}),
                json!({"type": "user", "message": {"content": [{"type": "tool_result", "content": "secret tool output kitab"}]}}),
                json!({"type": "assistant", "timestamp": "2026-11-22T10:00:05Z", "message": {"content": [{"type": "text", "text": "Run cargo install glaude"}]}}),
            ],
        );
        let xf =
            codex.join("rollout-2026-11-22T10-00-00-01a20385-94fd-7da0-aa72-26bf1ecdd1c6.jsonl");
        write_lines(
            &xf,
            &[
                json!({"type": "session_meta", "payload": {"id": "01a20385-94fd-7da0-aa72-26bf1ecdd1c6", "cwd": "/q"}}),
                json!({"type": "response_item", "payload": {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "# AGENTS.md instructions"}]}}),
                json!({"type": "event_msg", "timestamp": "2026-11-22T11:00:00Z", "payload": {"type": "item_completed", "item": {"type": "UserMessage", "content": [{"type": "text", "text": "fix the پرداخت flow"}]}}}),
                json!({"type": "event_msg", "timestamp": "2026-11-22T11:00:09Z", "payload": {"type": "item_completed", "item": {"type": "AgentMessage", "content": [{"type": "Text", "text": "Done, payment flow fixed."}]}}}),
            ],
        );
        let idx = Index::open(&dir.join("idx/search.sqlite")).unwrap();
        let sources = [
            Source {
                kind: Kind::Claude,
                root: dir.join("claude/projects"),
            },
            Source {
                kind: Kind::Codex,
                root: dir.join("codex/sessions"),
            },
        ];
        let st = idx.update(&sources, &mut |_| {}).unwrap();
        assert_eq!(st.messages, 4, "plumbing and tool output skipped");

        let h = idx.search("کتابخانه", 10).unwrap();
        assert!(h.is_empty(), "ZWNJ splits words, as typed");
        let h = idx.search("كتاب", 10).unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].session_id, "s1");
        assert_eq!(h[0].cwd, "/p");
        // The snippet shows the original letters (Arabic kaf here).
        assert!(
            h[0].snippet.contains("<mark>كتاب</mark>"),
            "{}",
            h[0].snippet
        );
        assert!(
            idx.search("kitab", 10).unwrap().is_empty(),
            "tool results not indexed"
        );

        let h = idx.search("پرداخت", 10).unwrap();
        assert_eq!(h[0].provider, "codex");
        assert_eq!(h[0].session_id, "01a20385-94fd-7da0-aa72-26bf1ecdd1c6");
        assert!(idx.search("payme", 10).unwrap().len() == 1, "prefix match");
        assert!(idx.search("agents", 10).unwrap().is_empty());

        // Append: only the new line is read.
        let mut text = fs::read_to_string(&cf).unwrap();
        text.push_str(&format!(
            "{}\n",
            json!({"type": "assistant", "timestamp": "2026-11-22T10:01:00Z", "message": {"content": [{"type": "text", "text": "تمام شد"}]}})
        ));
        fs::write(&cf, text).unwrap();
        let st = idx.update(&sources, &mut |_| {}).unwrap();
        assert_eq!((st.scanned, st.messages), (1, 1));
        let h = idx.search("تمام", 10).unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(idx.search("glaude", 10).unwrap()[0].matches, 1);

        // Deleted file disappears from results.
        fs::remove_file(&xf).unwrap();
        idx.update(&sources, &mut |_| {}).unwrap();
        assert!(idx.search("پرداخت", 10).unwrap().is_empty());
        fs::remove_dir_all(dir).ok();
    }
}

//! Codex accounts: profile from `auth.json` (id_token claims, decoded only),
//! usage from `state_5.sqlite` and the tail of recent rollouts.

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::Result;
use base64::Engine;
use chrono::{Local, TimeZone};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Map, Value};

use crate::tool::{Account, Ctx, Kind};
use crate::util::{iso, now_utc, parse_ts, read_obj, write_json};
use crate::Profile;

pub fn jwt_claims(token: &str) -> Map<String, Value> {
    let Some(payload) = token.split('.').nth(1) else {
        return Map::new();
    };
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    engine
        .decode(payload.trim_end_matches('='))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn auth(acct: &Account) -> Map<String, Value> {
    read_obj(&acct.path.join("auth.json"))
}

fn tokens(auth: &Map<String, Value>) -> Map<String, Value> {
    auth.get("tokens")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

pub fn read_profile(acct: &Account) -> Profile {
    let auth = auth(acct);
    let claims = jwt_claims(
        tokens(&auth)
            .get("id_token")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let oa = claims
        .get("https://api.openai.com/auth")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let u = &acct.usage;
    let mut email = claims
        .get("email")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if email.is_empty()
        && auth
            .get("OPENAI_API_KEY")
            .is_some_and(|v| !v.is_null() && v != "")
    {
        email = "(api key)".into();
    }
    let plan = oa
        .get("chatgpt_plan_type")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .or_else(|| u.get("plan_type").and_then(Value::as_str))
        .unwrap_or("")
        .to_string();
    let mut p = Profile {
        email,
        plan,
        limits_as_of: u
            .get("limits_as_of")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        ..Default::default()
    };
    for (key, src) in [("5h", "limit_5h"), ("week", "limit_weekly")] {
        if let Some(w) = u.get(src).and_then(Value::as_object) {
            if let Some(used) = w.get("used_percent").filter(|v| !v.is_null()) {
                p.set_limit(key, used, w.get("resets_at"));
            }
        }
    }
    p
}

/// Complete lines from the end of a file, newest first (256 KB chunks,
/// at most 8 MB read).
fn tail_lines(path: &Path, mut each: impl FnMut(&[u8]) -> bool) -> std::io::Result<()> {
    const MAX: u64 = 8_000_000;
    let mut f = fs::File::open(path)?;
    let mut pos = f.seek(SeekFrom::End(0))?;
    let mut carry: Vec<u8> = Vec::new();
    let mut read = 0u64;
    while pos > 0 && read < MAX {
        let size = (256 * 1024).min(pos);
        pos -= size;
        read += size;
        f.seek(SeekFrom::Start(pos))?;
        let mut chunk = vec![0u8; size as usize];
        f.read_exact(&mut chunk)?;
        chunk.extend_from_slice(&carry);
        let mut parts: Vec<&[u8]> = chunk.split(|b| *b == b'\n').collect();
        carry = parts.remove(0).to_vec();
        for line in parts.iter().rev() {
            if !line.is_empty() && !each(line) {
                return Ok(());
            }
        }
    }
    if pos == 0 && !carry.is_empty() {
        each(&carry);
    }
    Ok(())
}

fn window(limit: &Map<String, Value>, stamp: Option<chrono::DateTime<chrono::Utc>>) -> Value {
    if limit.get("used_percent").is_none_or(Value::is_null) {
        return json!({});
    }
    let mut resets = limit.get("resets_at").cloned().unwrap_or(Value::Null);
    let empty = matches!(&resets, Value::Null) || resets == json!(0) || resets == json!("");
    if empty {
        if let (Some(secs), Some(st)) = (
            limit
                .get("resets_in_seconds")
                .and_then(Value::as_f64)
                .filter(|s| *s != 0.0),
            stamp,
        ) {
            let t = st + chrono::Duration::milliseconds((secs * 1000.0) as i64);
            resets = json!(t.timestamp_millis() as f64 / 1000.0);
        }
    }
    json!({"used_percent": limit.get("used_percent"), "window_minutes": limit.get("window_minutes"), "resets_at": resets})
}

fn has_window(v: Option<&Value>) -> bool {
    v.and_then(Value::as_object)
        .and_then(|w| w.get("window_minutes"))
        .is_some_and(|m| m.as_f64().is_some_and(|f| f != 0.0))
}

pub fn refresh_usage(
    ctx: &Ctx,
    acct: &Account,
    progress: &mut dyn FnMut(f32),
) -> Result<Map<String, Value>> {
    let (mut total, mut sessions, mut tokens_7d, mut tokens_today) = (0i64, 0i64, 0i64, 0i64);
    let mut latest = String::new();
    let mut own_rollouts: Vec<String> = Vec::new();
    let db = acct.path.join("state_5.sqlite");
    let account_id = tokens(&auth(acct))
        .get("account_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    if db.exists() {
        let res: rusqlite::Result<()> = (|| {
            let conn = Connection::open_with_flags(
                &db,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
            )?;
            conn.busy_timeout(std::time::Duration::from_secs(2))?;
            let cols: Vec<String> = conn
                .prepare("PRAGMA table_info(threads)")?
                .query_map([], |r| r.get::<_, String>(1))?
                .flatten()
                .collect();
            let mut filter: Option<(String, String)> = None;
            if let Some(id) = &account_id {
                if cols.iter().any(|c| c == "creator_account_id") {
                    let w = if acct.protected {
                        " WHERE (creator_account_id = ? OR creator_account_id IS NULL)"
                    } else {
                        " WHERE (creator_account_id = ?)"
                    };
                    filter = Some((w.to_string(), id.clone()));
                }
            }
            let week = chrono::Utc::now().timestamp() - 7 * 86400;
            let today = Local
                .from_local_datetime(
                    &Local::now()
                        .date_naive()
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default(),
                )
                .earliest()
                .map(|d| d.timestamp())
                .unwrap_or(0);
            let sql = format!(
                "SELECT COALESCE(SUM(tokens_used),0), COUNT(*), MAX(updated_at), \
                 COALESCE(SUM(CASE WHEN updated_at >= ?1 THEN tokens_used END),0), \
                 COALESCE(SUM(CASE WHEN updated_at >= ?2 THEN tokens_used END),0) FROM threads{}",
                filter
                    .as_ref()
                    .map(|(w, _)| w.replace('?', "?3"))
                    .unwrap_or_default()
            );
            let row = |r: &rusqlite::Row| -> rusqlite::Result<(
                i64,
                i64,
                Option<rusqlite::types::Value>,
                i64,
                i64,
            )> {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            };
            let (t, s, last, w7, td) = match &filter {
                Some((_, id)) => conn.query_row(&sql, rusqlite::params![week, today, id], row)?,
                None => conn.query_row(&sql, rusqlite::params![week, today], row)?,
            };
            total = t;
            sessions = s;
            tokens_7d = w7;
            tokens_today = td;
            let last = match last {
                Some(rusqlite::types::Value::Integer(i)) => json!(i),
                Some(rusqlite::types::Value::Real(f)) => json!(f),
                Some(rusqlite::types::Value::Text(t)) => json!(t),
                _ => Value::Null,
            };
            latest = iso(parse_ts(&last));
            if let Some((w, id)) = &filter {
                let q = format!(
                    "SELECT rollout_path FROM threads{w} ORDER BY updated_at DESC LIMIT 40"
                );
                own_rollouts = conn
                    .prepare(&q)?
                    .query_map([id], |r| r.get::<_, Option<String>>(0))?
                    .flatten()
                    .flatten()
                    .collect();
            }
            Ok(())
        })();
        if let Err(e) = res {
            tracing::debug!("codex usage db {}: {e}", db.display());
        }
    }
    progress(0.5);

    let files: Vec<PathBuf> = if !own_rollouts.is_empty() {
        own_rollouts.iter().map(PathBuf::from).collect()
    } else {
        let mut v: Vec<PathBuf> = Vec::new();
        let sessions_dir = acct.path.join("sessions");
        for y in read_dirs(&sessions_dir) {
            for m in read_dirs(&y) {
                for d in read_dirs(&m) {
                    for f in fs::read_dir(&d).into_iter().flatten().flatten() {
                        let p = f.path();
                        let name = f.file_name().to_string_lossy().to_string();
                        if !name.starts_with('.') && name.ends_with(".jsonl") {
                            v.push(p);
                        }
                    }
                }
            }
        }
        v.sort_by(|a, b| b.to_string_lossy().cmp(&a.to_string_lossy()));
        v
    };

    let mut last_session: Map<String, Value> = Map::new();
    let mut have_last = false;
    let mut limits: Map<String, Value> = Map::new();
    let mut stamp = None;
    for path in files.iter().take(40) {
        let _ = tail_lines(path, |line| {
            if !contains(line, b"\"token_count\"") {
                return true;
            }
            let Ok(rec) = serde_json::from_slice::<Value>(line) else {
                return true;
            };
            let Some(payload) = rec.get("payload").and_then(Value::as_object) else {
                return true;
            };
            if payload.get("type").and_then(Value::as_str) != Some("token_count") {
                return true;
            }
            if !have_last {
                if let Some(info) = payload.get("info").and_then(Value::as_object) {
                    last_session = info
                        .get("total_token_usage")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default();
                    have_last = !last_session.is_empty();
                }
            }
            if limits.is_empty() {
                if let Some(rl) = payload.get("rate_limits").and_then(Value::as_object) {
                    if has_window(rl.get("primary")) || has_window(rl.get("secondary")) {
                        limits = rl.clone();
                        stamp = rec.get("timestamp").and_then(parse_ts);
                    }
                }
            }
            !(have_last && !limits.is_empty())
        });
        if have_last && !limits.is_empty() {
            break;
        }
    }

    let (mut limit_5h, mut limit_week) = (json!({}), json!({}));
    for key in ["primary", "secondary"] {
        let Some(w) = limits.get(key).and_then(Value::as_object) else {
            continue;
        };
        if !has_window(Some(&Value::Object(w.clone()))) {
            continue;
        }
        let win = window(w, stamp);
        let minutes = w
            .get("window_minutes")
            .and_then(Value::as_f64)
            .unwrap_or(0.0) as i64;
        if minutes >= 1440 {
            if limit_week.as_object().is_some_and(Map::is_empty) {
                limit_week = win;
            }
        } else if limit_5h.as_object().is_some_and(Map::is_empty) {
            limit_5h = win;
        }
    }
    let ls = |k: &str| last_session.get(k).cloned().unwrap_or(json!(0));
    let usage = json!({
        "total_tokens": total,
        "session_count": sessions,
        "tokens_7d": tokens_7d,
        "tokens_today": tokens_today,
        "latest_activity": latest,
        "last_session": {
            "input_tokens": ls("input_tokens"),
            "cached_input_tokens": ls("cached_input_tokens"),
            "output_tokens": ls("output_tokens"),
            "reasoning_output_tokens": ls("reasoning_output_tokens"),
        },
        "total_input_tokens": ls("input_tokens"),
        "total_output_tokens": ls("output_tokens"),
        "total_cached_input_tokens": ls("cached_input_tokens"),
        "total_reasoning_output_tokens": ls("reasoning_output_tokens"),
        "cost_usd": 0,
        "limit_5h": limit_5h,
        "limit_weekly": limit_week,
        "limits_as_of": iso(stamp),
        "plan_type": limits.get("plan_type").and_then(Value::as_str).unwrap_or(""),
        "last_updated": iso(Some(now_utc())),
    });
    write_json(
        &Kind::Codex.state_dir(ctx, acct).join(".usage.json"),
        &usage,
        true,
    )?;
    Ok(usage.as_object().cloned().unwrap_or_default())
}

fn read_dirs(p: &Path) -> Vec<PathBuf> {
    fs::read_dir(p)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect()
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    memchr::memmem::find(hay, needle).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_unpadded_jwt() {
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"email":"me@example.com","https://api.openai.com/auth":{"chatgpt_plan_type":"pro"}}"#);
        let c = jwt_claims(&format!("h.{payload}.s"));
        assert_eq!(c["email"], "me@example.com");
        assert!(jwt_claims("garbage").is_empty());
    }

    #[test]
    fn tail_reads_newest_first() {
        let p = std::env::temp_dir().join(format!("glaude-tail-{}.jsonl", std::process::id()));
        let body: String = (0..50_000).map(|i| format!("line {i}\n")).collect();
        fs::write(&p, body).unwrap();
        let mut got = Vec::new();
        tail_lines(&p, |l| {
            got.push(String::from_utf8_lossy(l).to_string());
            got.len() < 3
        })
        .unwrap();
        assert_eq!(got, ["line 49999", "line 49998", "line 49997"]);
        fs::remove_file(p).ok();
    }
}

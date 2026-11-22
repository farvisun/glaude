//! Claude Code accounts: profile from `.claude.json`, token usage from the
//! transcripts (incremental, index v2 shared with account_manager.py).

use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use chrono::Local;
use regex::Regex;
use serde_json::{json, Map, Value};

use crate::tool::{is_symlink, Account, Ctx, Kind};
use crate::util::{
    as_u64, days_total, iso, mtime_ns, now_utc, parse_ts, py_str, read_json, read_obj, write_json,
};
use crate::Profile;

pub const INDEX_VERSION: u64 = 2;

pub fn plan(oauth: &Map<String, Value>) -> String {
    let s = |k: &str| oauth.get(k).map(|v| py_str(Some(v))).unwrap_or_default();
    let tier = {
        let u = oauth.get("userRateLimitTier").filter(|v| truthy(v));
        let o = oauth.get("organizationRateLimitTier").filter(|v| truthy(v));
        u.or(o).map(|v| py_str(Some(v))).unwrap_or_default()
    };
    let re = Regex::new(r"max_(\d+x)").expect("regex");
    let mut plan = if let Some(m) = re.captures(&tier) {
        format!("max {}", &m[1])
    } else if tier.contains("claude_ai") || tier.contains("pro") {
        "pro".into()
    } else {
        String::new()
    };
    let seat = oauth
        .get("seatTier")
        .filter(|v| truthy(v))
        .map(|v| py_str(Some(v)))
        .unwrap_or_default();
    if seat.contains("team") {
        plan = format!("team {}", plan.replace("max ", ""))
            .trim()
            .to_string();
    } else if seat.contains("enterprise") {
        plan = format!("ent {plan}").trim().to_string();
    }
    if plan.is_empty() && oauth.get("billingType").is_some_and(truthy) {
        plan = s("billingType").split('_').next().unwrap_or("").to_string();
    }
    plan
}

/// Python truthiness for JSON values.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

pub fn claude_json(ctx: &Ctx, acct: &Account) -> std::path::PathBuf {
    if Kind::Claude.is_default(ctx, acct) {
        ctx.home.join(".claude.json")
    } else {
        acct.path.join(".claude.json")
    }
}

pub fn read_profile(ctx: &Ctx, acct: &Account) -> Profile {
    let data = read_obj(&claude_json(ctx, acct));
    let oauth = data
        .get("oauthAccount")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let util = data
        .get("cachedUsageUtilization")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let windows = util
        .get("utilization")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut p = Profile {
        email: oauth
            .get("emailAddress")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        org: oauth
            .get("organizationName")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        plan: plan(&oauth),
        limits_as_of: iso(util.get("fetchedAtMs").and_then(parse_ts)),
        ..Default::default()
    };
    for (key, src) in [("5h", "five_hour"), ("week", "seven_day")] {
        if let Some(w) = windows.get(src).and_then(Value::as_object) {
            if let Some(used) = w.get("utilization").filter(|v| !v.is_null()) {
                p.set_limit(key, used, w.get("resets_at"));
            }
        }
    }
    p
}

fn index_path(ctx: &Ctx, acct: &Account) -> std::path::PathBuf {
    let shared = is_symlink(&acct.path.join("projects")) || Kind::Claude.is_default(ctx, acct);
    if shared {
        ctx.root.join(".usage_index.shared.json")
    } else {
        acct.path.join(".usage_index.json")
    }
}

/// `projects/**/*.jsonl` relative to the account, following symlinks and
/// skipping dot entries (Python `glob(..., recursive=True)`), sorted.
fn transcripts(acct: &Account) -> Vec<String> {
    let mut out = Vec::new();
    let root = acct.path.join("projects");
    if !root.is_dir() {
        return out;
    }
    let walker = walkdir::WalkDir::new(&root)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'));
    for e in walker.flatten() {
        if e.file_type().is_file() && e.file_name().to_string_lossy().ends_with(".jsonl") {
            if let Ok(rel) = e.path().strip_prefix(&acct.path) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    out.sort();
    out
}

fn fresh_entry() -> Map<String, Value> {
    let v = json!({"offset": 0, "in": 0, "out": 0, "cr": 0, "cw": 0, "msgs": 0,
                   "days": {}, "models": {}, "last_ts": "", "recent": []});
    v.as_object().cloned().unwrap_or_default()
}

fn get_u(e: &Map<String, Value>, k: &str) -> u64 {
    e.get(k).map(as_u64).unwrap_or(0)
}

/// Parse only the bytes appended since the last scan (Python `_scan_claude_file`).
pub fn scan_file(
    path: &Path,
    prev: Option<&Map<String, Value>>,
    on_bytes: &mut dyn FnMut(u64),
) -> std::io::Result<Map<String, Value>> {
    let meta = fs::metadata(path)?;
    let size = meta.len();
    let mtime = mtime_ns(&meta);
    if let Some(p) = prev {
        if p.get("size").and_then(Value::as_u64) == Some(size)
            && p.get("mtime").and_then(Value::as_i64) == Some(mtime)
        {
            return Ok(p.clone());
        }
    }
    let mut entry = match prev {
        Some(p) if size >= get_u(p, "offset") => p.clone(),
        _ => fresh_entry(),
    };
    let recent_old: Vec<String> = entry
        .get("recent")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let mut seen: HashSet<String> = recent_old.iter().cloned().collect();
    let mut recent = recent_old;
    let mut offset = get_u(&entry, "offset");
    let mut sums = [
        get_u(&entry, "in"),
        get_u(&entry, "out"),
        get_u(&entry, "cr"),
        get_u(&entry, "cw"),
    ];
    let mut msgs = get_u(&entry, "msgs");
    let mut days = entry
        .get("days")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut models = entry
        .get("models")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut last_ts = entry
        .get("last_ts")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let mut pending = 0u64;

    let usage_finder = memchr::memmem::Finder::new(b"\"usage\"");
    let assistant_finder = memchr::memmem::Finder::new(b"\"assistant\"");
    let mut f = BufReader::with_capacity(1 << 20, fs::File::open(path)?);
    f.seek(SeekFrom::Start(offset))?;
    let mut line = Vec::with_capacity(64 * 1024);
    loop {
        line.clear();
        let n = f.read_until(b'\n', &mut line)?;
        if n == 0 || line.last() != Some(&b'\n') {
            break; // partially written line; picked up next time
        }
        offset += n as u64;
        pending += n as u64;
        if pending > 4_000_000 {
            on_bytes(pending);
            pending = 0;
        }
        if usage_finder.find(&line).is_some() && assistant_finder.find(&line).is_some() {
            let Ok(rec) = serde_json::from_slice::<Value>(&line) else {
                continue;
            };
            let Some(msg) = rec.get("message").and_then(Value::as_object) else {
                continue;
            };
            let Some(u) = msg.get("usage").and_then(Value::as_object) else {
                continue;
            };
            let key = format!(
                "{}:{}",
                py_str(msg.get("id")),
                py_str(rec.as_object().and_then(|o| o.get("requestId")))
            );
            if key != ":" {
                if seen.contains(&key) {
                    continue;
                }
                seen.insert(key.clone());
                recent.push(key);
            }
            let vals: Vec<u64> = [
                "input_tokens",
                "output_tokens",
                "cache_read_input_tokens",
                "cache_creation_input_tokens",
            ]
            .iter()
            .map(|k| u.get(*k).map(as_u64).unwrap_or(0))
            .collect();
            for (s, v) in sums.iter_mut().zip(&vals) {
                *s += v;
            }
            msgs += 1;
            let model = msg
                .get("model")
                .filter(|v| truthy(v))
                .map(|v| py_str(Some(v)))
                .unwrap_or_default();
            if !model.is_empty() && !model.starts_with('<') {
                let c = models.get(&model).map(as_u64).unwrap_or(0) + 1;
                models.insert(model, json!(c));
            }
            if let Some(ts) = rec.get("timestamp").and_then(parse_ts) {
                let day = ts.with_timezone(&Local).format("%Y-%m-%d").to_string();
                let c = days.get(&day).map(as_u64).unwrap_or(0) + vals.iter().sum::<u64>();
                days.insert(day, json!(c));
                let stamp = iso(Some(ts));
                if stamp > last_ts {
                    last_ts = stamp;
                }
            }
        }
    }
    on_bytes(pending);
    let keep = recent.len().saturating_sub(24);
    entry.insert("offset".into(), json!(offset));
    entry.insert("in".into(), json!(sums[0]));
    entry.insert("out".into(), json!(sums[1]));
    entry.insert("cr".into(), json!(sums[2]));
    entry.insert("cw".into(), json!(sums[3]));
    entry.insert("msgs".into(), json!(msgs));
    entry.insert("days".into(), Value::Object(days));
    entry.insert("models".into(), Value::Object(models));
    entry.insert("last_ts".into(), json!(last_ts));
    entry.insert("recent".into(), json!(recent[keep..].to_vec()));
    entry.insert("size".into(), json!(size));
    entry.insert("mtime".into(), json!(mtime));
    Ok(entry)
}

/// Scan the account's transcripts and write `.usage.json`.
pub fn refresh_usage(
    ctx: &Ctx,
    acct: &Account,
    progress: &mut dyn FnMut(f32),
) -> Result<Map<String, Value>> {
    let index_path = index_path(ctx, acct);
    let index = read_json(&index_path);
    let old: Map<String, Value> =
        if index.get("version").and_then(Value::as_u64) == Some(INDEX_VERSION) {
            index
                .get("files")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default()
        } else {
            Map::new()
        };
    let files = transcripts(acct);
    let mut todo = Vec::new();
    let mut total_bytes = 0u64;
    for rel in &files {
        let Ok(meta) = fs::metadata(acct.path.join(rel)) else {
            continue;
        };
        let prev = old.get(rel).and_then(Value::as_object);
        if let Some(p) = prev {
            if p.get("size").and_then(Value::as_u64) == Some(meta.len())
                && p.get("mtime").and_then(Value::as_i64) == Some(mtime_ns(&meta))
            {
                continue;
            }
        }
        let start = prev
            .map(|p| get_u(p, "offset"))
            .filter(|o| meta.len() >= *o)
            .unwrap_or(0);
        todo.push(rel.clone());
        total_bytes += meta.len().saturating_sub(start);
    }
    let mut new_files = Map::new();
    for rel in &files {
        if let Some(v) = old.get(rel) {
            new_files.insert(rel.clone(), v.clone());
        }
    }
    let mut done = 0u64;
    let mut last_save = Instant::now();
    let save = |files: &Map<String, Value>| {
        write_json(
            &index_path,
            &json!({"version": INDEX_VERSION, "files": files}),
            false,
        )
    };
    for rel in &todo {
        let prev = old.get(rel).and_then(Value::as_object);
        let mut on_bytes = |n: u64| {
            done += n;
            if total_bytes > 0 {
                progress((done as f32 / total_bytes as f32).min(0.99));
            }
        };
        match scan_file(&acct.path.join(rel), prev, &mut on_bytes) {
            Ok(e) => {
                new_files.insert(rel.clone(), Value::Object(e));
            }
            Err(_) => continue,
        }
        if last_save.elapsed().as_secs_f32() > 3.0 {
            save(&new_files)?;
            last_save = Instant::now();
        }
    }
    let old_keys: HashSet<&String> = old.keys().collect();
    let new_keys: HashSet<&String> = new_files.keys().collect();
    if !todo.is_empty() || old_keys != new_keys {
        save(&new_files)?;
    }

    let mut tot = [0u64; 5];
    let mut days = Map::new();
    let mut models: Vec<(String, u64)> = Vec::new();
    let mut sessions = 0u64;
    let mut latest = String::new();
    for (rel, e) in &new_files {
        let Some(e) = e.as_object() else { continue };
        for (i, k) in ["in", "out", "cr", "cw", "msgs"].iter().enumerate() {
            tot[i] += get_u(e, k);
        }
        for (d, v) in e
            .get("days")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            let c = days.get(d).map(as_u64).unwrap_or(0) + as_u64(v);
            days.insert(d.clone(), json!(c));
        }
        for (m, v) in e
            .get("models")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            match models.iter_mut().find(|(n, _)| n == m) {
                Some((_, c)) => *c += as_u64(v),
                None => models.push((m.clone(), as_u64(v))),
            }
        }
        if get_u(e, "msgs") > 0 && !rel.contains("/subagents/") {
            sessions += 1;
        }
        let ts = e.get("last_ts").and_then(Value::as_str).unwrap_or("");
        if ts > latest.as_str() {
            latest = ts.to_string();
        }
    }
    // Python: sorted(..., key=-count) is stable on insertion order.
    models.sort_by_key(|m| std::cmp::Reverse(m.1));
    let top: Map<String, Value> = models
        .into_iter()
        .take(5)
        .map(|(m, c)| (m, json!(c)))
        .collect();
    let usage = json!({
        "total_input_tokens": tot[0],
        "total_output_tokens": tot[1],
        "total_cache_read_input_tokens": tot[2],
        "total_cache_creation_input_tokens": tot[3],
        "total_tokens": tot[0] + tot[1] + tot[2] + tot[3],
        "usage_record_count": tot[4],
        "session_count": sessions,
        "tokens_today": days_total(&days, 1),
        "tokens_7d": days_total(&days, 7),
        "models": top,
        "latest_activity": latest,
        "last_updated": iso(Some(now_utc())),
    });
    write_json(
        &Kind::Claude.state_dir(ctx, acct).join(".usage.json"),
        &usage,
        true,
    )?;
    Ok(usage.as_object().cloned().unwrap_or_default())
}

/// Copy MCP servers and per-project trust/MCP settings from `~/.claude.json`
/// into a linked account (Python `ClaudeProvider.after_share`).
pub fn after_share(ctx: &Ctx, acct: &mut Account) -> Result<Vec<String>> {
    const PROJECT_KEYS: &[&str] = &[
        "mcpServers",
        "enabledMcpjsonServers",
        "disabledMcpjsonServers",
        "hasTrustDialogAccepted",
        "allowedTools",
    ];
    let path = acct.path.join(".claude.json");
    let central = read_json(&ctx.home.join(".claude.json"));
    let data = read_json(&path);
    let (Some(central), Some(mut data)) = (central.as_object().cloned(), data.as_object().cloned())
    else {
        return Ok(Vec::new());
    };
    if !data.get("oauthAccount").is_some_and(truthy) {
        return Ok(Vec::new());
    }
    let before = Value::Object(data.clone());
    let central_mcp = central
        .get("mcpServers")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut servers = data
        .get("mcpServers")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let synced: Vec<String> = acct
        .meta
        .get("synced_mcp")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    for name in &synced {
        if !central_mcp.contains_key(name) {
            servers.remove(name);
        }
    }
    for (k, v) in &central_mcp {
        servers.insert(k.clone(), v.clone());
    }
    data.insert("mcpServers".into(), Value::Object(servers));
    let projects = data
        .entry("projects")
        .or_insert_with(|| Value::Object(Map::new()));
    if let (Some(projects), Some(cp)) = (
        projects.as_object_mut(),
        central.get("projects").and_then(Value::as_object),
    ) {
        for (proj, cfg) in cp {
            let Some(cfg) = cfg.as_object() else { continue };
            let entry = projects
                .entry(proj.clone())
                .or_insert_with(|| Value::Object(Map::new()));
            if let Some(entry) = entry.as_object_mut() {
                for k in PROJECT_KEYS {
                    if let Some(v) = cfg.get(*k) {
                        entry.insert((*k).to_string(), v.clone());
                    }
                }
            }
        }
    }
    let mut want: Vec<String> = central_mcp.keys().cloned().collect();
    want.sort();
    if acct.meta.get("synced_mcp") != Some(&json!(want)) {
        acct.meta.insert("synced_mcp".into(), json!(want));
        write_json(
            &Kind::Claude.state_dir(ctx, acct).join(".meta.json"),
            &Value::Object(acct.meta.clone()),
            true,
        )?;
    }
    let after = Value::Object(data);
    if after == before {
        return Ok(Vec::new());
    }
    write_json(&path, &after, true)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(vec!["synced MCP servers and project settings".into()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(v: Value) -> Map<String, Value> {
        v.as_object().cloned().unwrap()
    }

    #[test]
    fn plans() {
        assert_eq!(
            plan(&obj(json!({"userRateLimitTier": "default_claude_max_20x"}))),
            "max 20x"
        );
        assert_eq!(
            plan(&obj(
                json!({"organizationRateLimitTier": "default_claude_ai"})
            )),
            "pro"
        );
        assert_eq!(
            plan(&obj(
                json!({"userRateLimitTier": "default_claude_max_5x", "seatTier": "team_standard"})
            )),
            "team 5x"
        );
        assert_eq!(
            plan(&obj(json!({"billingType": "stripe_subscription"}))),
            "stripe"
        );
        assert_eq!(plan(&obj(json!({}))), "");
    }

    #[test]
    fn incremental_scan_dedups_and_resumes() {
        let dir = std::env::temp_dir().join(format!("glaude-acct-scan-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("s.jsonl");
        let rec = |id: &str, req: &str, n: u64| {
            format!(
                "{}\n",
                json!({"type": "assistant", "requestId": req, "timestamp": "2026-11-20T10:00:00Z",
                       "message": {"id": id, "model": "claude-x", "usage": {"input_tokens": n, "output_tokens": 1}}})
            )
        };
        fs::write(
            &f,
            rec("m1", "r1", 10) + &rec("m1", "r1", 10) + &rec("m2", "r2", 5) + "{\"partial\":",
        )
        .unwrap();
        let e = scan_file(&f, None, &mut |_| {}).unwrap();
        assert_eq!(
            get_u(&e, "in"),
            15,
            "duplicate streamed record counted once"
        );
        assert_eq!(get_u(&e, "msgs"), 2);
        let first_offset = get_u(&e, "offset");
        // Finish the partial line and add one more record.
        let mut text = fs::read_to_string(&f).unwrap();
        text.truncate(first_offset as usize);
        text.push_str(&rec("m3", "r3", 7));
        fs::write(&f, text).unwrap();
        let e2 = scan_file(&f, Some(&e), &mut |_| {}).unwrap();
        assert_eq!(get_u(&e2, "in"), 22);
        assert_eq!(e2["models"]["claude-x"], 3);
        assert_eq!(e2["recent"].as_array().unwrap().len(), 3);
        fs::remove_dir_all(dir).ok();
    }
}

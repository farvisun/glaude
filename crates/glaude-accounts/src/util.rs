//! Helpers shared with account_manager.py: timestamps, JSON files, names.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde_json::{Map, Value};

pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// `%Y-%m-%dT%H:%M:%SZ` in UTC, or "" (Python `iso`).
pub fn iso(dt: Option<DateTime<Utc>>) -> String {
    dt.map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_default()
}

/// ISO string, epoch seconds or epoch milliseconds (> 1e10) → UTC
/// (Python `parse_ts`). Strings without a zone are UTC.
pub fn parse_ts(v: &Value) -> Option<DateTime<Utc>> {
    match v {
        Value::Null | Value::Bool(_) => None,
        Value::Number(n) => {
            let f = n.as_f64()?;
            if f == 0.0 {
                return None;
            }
            from_epoch(f)
        }
        Value::String(s) => parse_ts_str(s),
        _ => None,
    }
}

pub fn parse_ts_str(s: &str) -> Option<DateTime<Utc>> {
    if s.is_empty() || s == "0" {
        return None;
    }
    // Python: str(value).replace(".", "", 1).isdigit()
    let once = s.replacen('.', "", 1);
    if !once.is_empty() && once.chars().all(|c| c.is_ascii_digit()) {
        return from_epoch(s.parse::<f64>().ok()?);
    }
    let t = s.replace('Z', "+00:00");
    if let Ok(d) = DateTime::parse_from_rfc3339(&t) {
        return Some(d.with_timezone(&Utc));
    }
    for fmt in [
        "%Y-%m-%dT%H:%M:%S%.f%:z",
        "%Y-%m-%d %H:%M:%S%.f%:z",
        "%Y-%m-%dT%H:%M%:z",
    ] {
        if let Ok(d) = DateTime::parse_from_str(&t, fmt) {
            return Some(d.with_timezone(&Utc));
        }
    }
    for fmt in [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M",
    ] {
        if let Ok(d) = NaiveDateTime::parse_from_str(&t, fmt) {
            return Some(Utc.from_utc_datetime(&d));
        }
    }
    if let Ok(d) = NaiveDate::parse_from_str(&t, "%Y-%m-%d") {
        return Some(Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0)?));
    }
    None
}

fn from_epoch(mut f: f64) -> Option<DateTime<Utc>> {
    if !f.is_finite() {
        return None;
    }
    if f > 10_000_000_000.0 {
        f /= 1000.0;
    }
    let secs = f.floor() as i64;
    let nanos = ((f - f.floor()) * 1e9) as u32;
    Utc.timestamp_opt(secs, nanos).single()
}

/// Lenient read: anything unreadable or invalid is `{}`.
pub fn read_json(path: &Path) -> Value {
    fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| Value::Object(Map::new()))
}

/// The object at `path`, or an empty map.
pub fn read_obj(path: &Path) -> Map<String, Value> {
    match read_json(path) {
        Value::Object(m) => m,
        _ => Map::new(),
    }
}

/// Atomic write via `.<name>.<pid>.tmp` + rename (Python `write_json`).
/// `pretty` = indent 2; otherwise compact.
pub fn write_json(path: &Path, data: &Value, pretty: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    let bytes = if pretty {
        serde_json::to_vec_pretty(data)?
    } else {
        serde_json::to_vec(data)?
    };
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(&bytes)?;
        f.flush()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Trim, whitespace → `_`, anything outside `[A-Za-z0-9._-]` → `_`.
pub fn normalize_name(raw: &str) -> String {
    raw.trim()
        .chars()
        .map(|c| {
            if c.is_whitespace() {
                '_'
            } else if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// `^[A-Za-z0-9][A-Za-z0-9._-]*$`, not `.`/`..`.
pub fn check_name_shape(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Account name cannot be empty.");
    }
    let first_ok = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric());
    let rest_ok = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if !first_ok || !rest_ok || name == "." || name == ".." {
        bail!("Use letters, numbers, '.', '_' or '-', starting with a letter or number.");
    }
    Ok(())
}

/// Sum of `days` entries within the last `within` local calendar days.
pub fn days_total(days: &Map<String, Value>, within: i64) -> u64 {
    let cutoff = (Local::now().date_naive() - chrono::Duration::days(within - 1))
        .format("%Y-%m-%d")
        .to_string();
    days.iter()
        .filter(|(k, _)| k.as_str() >= cutoff.as_str())
        .map(|(_, v)| as_u64(v))
        .sum()
}

/// Python `int(x or 0)` for counters: ints, floats and numeric strings.
pub fn as_u64(v: &Value) -> u64 {
    match v {
        Value::Number(n) => n
            .as_u64()
            .or_else(|| n.as_f64().map(|f| f.max(0.0) as u64))
            .unwrap_or(0),
        Value::String(s) => s
            .trim()
            .parse::<f64>()
            .map(|f| f.max(0.0) as u64)
            .unwrap_or(0),
        Value::Bool(true) => 1,
        _ => 0,
    }
}

/// Python `str()` of a JSON value read with `.get(key, '')`.
pub fn py_str(v: Option<&Value>) -> String {
    match v {
        None => String::new(),
        Some(Value::Null) => "None".into(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => if *b { "True" } else { "False" }.into(),
        Some(other) => other.to_string(),
    }
}

/// `~/…` for display.
pub fn tilde(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".into(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// `~` / `~/x` → absolute (Python `expanduser`).
pub fn expand_user(raw: &str, home: &Path) -> PathBuf {
    if raw == "~" {
        home.to_path_buf()
    } else if let Some(rest) = raw.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(raw)
    }
}

/// Modification time in integer nanoseconds (Python `st_mtime_ns`).
pub fn mtime_ns(meta: &fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_ts_like_python() {
        let a = parse_ts(&json!("2026-11-22T10:00:00.123Z")).unwrap();
        assert_eq!(iso(Some(a)), "2026-11-22T10:00:00Z");
        assert_eq!(iso(parse_ts(&json!(1795355000))), "2026-11-22T13:43:20Z");
        assert_eq!(
            iso(parse_ts(&json!(1795355000123u64))),
            "2026-11-22T13:43:20Z"
        );
        assert_eq!(iso(parse_ts(&json!("1795355000"))), "2026-11-22T13:43:20Z");
        assert_eq!(
            iso(parse_ts(&json!("2026-11-22T10:00:00"))),
            "2026-11-22T10:00:00Z"
        );
        assert_eq!(
            iso(parse_ts(&json!("2026-11-22T13:30:00+03:30"))),
            "2026-11-22T10:00:00Z"
        );
        assert!(parse_ts(&json!(0)).is_none());
        assert!(parse_ts(&json!("")).is_none());
        assert!(parse_ts(&json!("nope")).is_none());
    }

    #[test]
    fn names() {
        assert_eq!(normalize_name("  my work acct "), "my_work_acct");
        assert_eq!(normalize_name("a@b.com"), "a_b.com");
        assert!(check_name_shape("a_b.com").is_ok());
        assert!(check_name_shape("_x").is_err());
        assert!(check_name_shape("..").is_err());
    }

    #[test]
    fn counters() {
        assert_eq!(as_u64(&json!(12)), 12);
        assert_eq!(as_u64(&json!(12.9)), 12);
        assert_eq!(as_u64(&json!("7")), 7);
        assert_eq!(as_u64(&json!(null)), 0);
        assert_eq!(py_str(Some(&json!(null))), "None");
        assert_eq!(py_str(None), "");
    }
}

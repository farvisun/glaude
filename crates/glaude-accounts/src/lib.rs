//! mamadoo in Rust: Claude Code and Codex accounts, file-compatible with
//! `account_manager.py` (see docs/mamadoo-port.md).

pub mod claude;
pub mod codex;
pub mod paths;
pub mod share;
pub mod tool;
pub mod util;

use anyhow::Result;
use serde::Serialize;
use serde_json::{Map, Value};

pub use tool::{Account, Ctx, Kind};

/// One limit window as the UI shows it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Limit {
    /// Percent used (0–100).
    pub used: f64,
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub email: String,
    pub org: String,
    pub plan: String,
    pub five_hour: Option<Limit>,
    pub week: Option<Limit>,
    pub limits_as_of: String,
}

impl Profile {
    pub(crate) fn set_limit(&mut self, key: &str, used: &Value, resets: Option<&Value>) {
        let used = match used {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse().ok(),
            _ => None,
        };
        let Some(used) = used else { return };
        let lim = Limit {
            used,
            resets_at: resets.and_then(util::parse_ts).map(|t| util::iso(Some(t))),
        };
        match key {
            "5h" => self.five_hour = Some(lim),
            _ => self.week = Some(lim),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    pub label: String,
    pub text: String,
}

/// Everything the Accounts page shows for one account.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub provider: Kind,
    pub name: String,
    pub path: String,
    pub archived: bool,
    pub protected: bool,
    pub active: bool,
    pub running: bool,
    #[serde(flatten)]
    pub profile: Profile,
    pub tokens: u64,
    pub tokens_today: u64,
    pub tokens_7d: u64,
    /// Sessions found in the usage scan.
    pub sessions: u64,
    /// Launches recorded in `.meta.json`.
    pub launches: u64,
    pub last_used: Option<String>,
    pub created_at: Option<String>,
    pub usage_updated: Option<String>,
    pub shared_linked: usize,
    pub shared_pending: usize,
    pub shared_total: usize,
    pub details: Vec<Detail>,
}

fn u(m: &Map<String, Value>, k: &str) -> u64 {
    m.get(k).map(util::as_u64).unwrap_or(0)
}

fn fmt_tokens(n: u64) -> String {
    if n == 0 {
        return "-".into();
    }
    for (limit, suffix) in [(1_000_000_000u64, "B"), (1_000_000, "M"), (1_000, "K")] {
        if n >= limit {
            return format!("{:.1}{suffix}", n as f64 / limit as f64);
        }
    }
    n.to_string()
}

fn details(kind: Kind, usage: &Map<String, Value>) -> Vec<Detail> {
    let mut out = Vec::new();
    if usage.is_empty() {
        return out;
    }
    let d = |label: &str, text: String| Detail {
        label: label.into(),
        text,
    };
    match kind {
        Kind::Claude => {
            out.push(d(
                "usage",
                format!(
                    "in {} · out {} · cache read {} · cache write {} · {} msgs · {} sessions",
                    fmt_tokens(u(usage, "total_input_tokens")),
                    fmt_tokens(u(usage, "total_output_tokens")),
                    fmt_tokens(u(usage, "total_cache_read_input_tokens")),
                    fmt_tokens(u(usage, "total_cache_creation_input_tokens")),
                    fmt_tokens(u(usage, "usage_record_count")),
                    u(usage, "session_count")
                ),
            ));
            if let Some(models) = usage.get("models").and_then(Value::as_object) {
                let total: u64 = models.values().map(util::as_u64).sum::<u64>().max(1);
                let text = models
                    .iter()
                    .take(3)
                    .map(|(m, v)| {
                        format!(
                            "{} {}%",
                            m.replace("claude-", ""),
                            util::as_u64(v) * 100 / total
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                if !text.is_empty() {
                    out.push(d("models", text));
                }
            }
        }
        Kind::Codex => {
            out.push(d(
                "usage",
                format!(
                    "{} tokens · {} threads · today {} · 7d {}",
                    fmt_tokens(u(usage, "total_tokens")),
                    u(usage, "session_count"),
                    fmt_tokens(u(usage, "tokens_today")),
                    fmt_tokens(u(usage, "tokens_7d"))
                ),
            ));
            if let Some(ls) = usage.get("last_session").and_then(Value::as_object) {
                if ls.values().any(|v| util::as_u64(v) > 0) {
                    out.push(d(
                        "last run",
                        format!(
                            "in {} · cached {} · out {} · reasoning {}",
                            fmt_tokens(u(ls, "input_tokens")),
                            fmt_tokens(u(ls, "cached_input_tokens")),
                            fmt_tokens(u(ls, "output_tokens")),
                            fmt_tokens(u(ls, "reasoning_output_tokens"))
                        ),
                    ));
                }
            }
        }
    }
    out
}

pub fn read_profile(ctx: &Ctx, acct: &Account) -> Profile {
    match acct.kind {
        Kind::Claude => claude::read_profile(ctx, acct),
        Kind::Codex => codex::read_profile(acct),
    }
}

fn created_at(acct: &Account) -> Option<String> {
    if let Some(t) = acct.meta.get("created_at").and_then(util::parse_ts) {
        return Some(util::iso(Some(t)));
    }
    if acct.kind == Kind::Codex {
        let db = acct.path.join("state_5.sqlite");
        if db.exists() {
            let min = rusqlite::Connection::open_with_flags(
                &db,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .and_then(|c| {
                c.query_row("SELECT MIN(created_at) FROM threads", [], |r| {
                    r.get::<_, Option<i64>>(0)
                })
            })
            .ok()
            .flatten();
            if let Some(t) = min.and_then(|m| util::parse_ts(&Value::from(m))) {
                return Some(util::iso(Some(t)));
            }
        }
    }
    std::fs::metadata(&acct.path)
        .and_then(|m| m.created().or_else(|_| m.modified()))
        .ok()
        .map(|t| util::iso(Some(chrono::DateTime::<chrono::Utc>::from(t))))
}

pub fn view(
    ctx: &Ctx,
    acct: &Account,
    active: &str,
    running: &std::collections::HashSet<String>,
) -> AccountView {
    let kind = acct.kind;
    let (linked, pending) = share::state(ctx, kind, acct);
    let last_used = [
        acct.meta.get("last_used").and_then(util::parse_ts),
        acct.usage.get("latest_activity").and_then(util::parse_ts),
    ]
    .into_iter()
    .flatten()
    .max();
    let path_str = acct.path.display().to_string();
    AccountView {
        provider: kind,
        name: acct.name.clone(),
        path: util::tilde(&acct.path, &ctx.home),
        archived: acct.archived,
        protected: acct.protected,
        active: acct.name == active,
        running: running.contains(&path_str),
        profile: read_profile(ctx, acct),
        tokens: u(&acct.usage, "total_tokens"),
        tokens_today: u(&acct.usage, "tokens_today"),
        tokens_7d: u(&acct.usage, "tokens_7d"),
        sessions: u(&acct.usage, "session_count"),
        launches: u(&acct.meta, "session_count"),
        last_used: last_used.map(|t| util::iso(Some(t))),
        created_at: created_at(acct),
        usage_updated: acct
            .usage
            .get("last_updated")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        shared_linked: linked,
        shared_pending: pending,
        shared_total: kind.shared().len(),
        details: details(kind, &acct.usage),
    }
}

/// All accounts of a tool with what is already on disk (no scan).
pub fn list(ctx: &Ctx, kind: Kind) -> Vec<AccountView> {
    let active = kind.active(ctx);
    let running = share::running_homes(kind.home_env());
    kind.accounts(ctx, true)
        .iter()
        .map(|a| view(ctx, a, &active, &running))
        .collect()
}

/// Rescan usage for one account and return its fresh view.
pub fn refresh(
    ctx: &Ctx,
    kind: Kind,
    name: &str,
    progress: &mut dyn FnMut(f32),
) -> Result<AccountView> {
    let acct = kind.find(ctx, name)?;
    match kind {
        Kind::Claude => claude::refresh_usage(ctx, &acct, progress)?,
        Kind::Codex => codex::refresh_usage(ctx, &acct, progress)?,
    };
    progress(1.0);
    let acct = kind.find(ctx, name)?;
    let running = share::running_homes(kind.home_env());
    Ok(view(ctx, &acct, &kind.active(ctx), &running))
}

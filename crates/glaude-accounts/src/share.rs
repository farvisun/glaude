//! Link an account's shared items into the central home (Python `share`).
//! Real files found in the account are merged into the central copy first;
//! whatever is not kept goes to `~/.agent-accounts-backup/<ts>/`, never deleted.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;
use rusqlite::Connection;

use crate::tool::{is_symlink, Account, Ctx, Kind};
use crate::util::{parse_ts, tilde};

const APPEND_LOGS: &[&str] = &["history.jsonl", "session_index.jsonl"];

fn exists_or_link(p: &Path) -> bool {
    p.exists() || is_symlink(p)
}

/// Whether `name` still has to be linked for this account.
pub fn needs_link(ctx: &Ctx, kind: Kind, acct: &Account, name: &str) -> bool {
    let link = acct.path.join(name);
    let target = kind.central(ctx).join(name);
    // Exact readlink text, like Python's os.readlink(link) == str(target).
    if is_symlink(&link) && fs::read_link(&link).is_ok_and(|t| t.as_os_str() == target.as_os_str())
    {
        return false;
    }
    target.exists() || link.exists() || is_symlink(&link)
}

/// `(linked, pending)` counts for the UI.
pub fn state(ctx: &Ctx, kind: Kind, acct: &Account) -> (usize, usize) {
    if same_dir(&acct.path, &kind.central(ctx)) {
        return (kind.shared().len(), 0);
    }
    let pending = kind
        .shared()
        .iter()
        .filter(|n| needs_link(ctx, kind, acct, n))
        .count();
    let linked = kind
        .shared()
        .iter()
        .filter(|n| {
            let link = acct.path.join(n);
            is_symlink(&link)
                && fs::read_link(&link)
                    .is_ok_and(|t| t.as_os_str() == kind.central(ctx).join(n).as_os_str())
        })
        .count();
    (linked, pending)
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// Homes of claude/codex processes running now, from their environment
/// (`ps eww`). Only the agents count, not programs started from their shells.
pub fn running_homes(env_key: &str) -> HashSet<String> {
    let mut homes = HashSet::new();
    #[cfg(unix)]
    {
        let out = std::process::Command::new("ps")
            .args(["eww", "-ax", "-o", "command="])
            .output();
        let Ok(out) = out else { return homes };
        let text = String::from_utf8_lossy(&out.stdout);
        let re =
            Regex::new(&format!(r"(?:^|[^A-Z_]){}=(\S+)", regex::escape(env_key))).expect("regex");
        for line in text.lines() {
            let words: Vec<&str> = line.splitn(3, char::is_whitespace).collect();
            let exe_ok = words.iter().take(2).any(|w| {
                let base = Path::new(w)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                base.starts_with("claude") || base.starts_with("codex")
            });
            let first = line.split(' ').next().unwrap_or("");
            if !exe_ok && !first.contains("/claude/versions/") {
                continue;
            }
            for c in re.captures_iter(line) {
                homes.insert(c[1].trim_end_matches('/').to_string());
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (env_key, Regex::new("").ok());
    }
    homes
}

pub fn is_running(kind: Kind, acct: &Account) -> bool {
    running_homes(kind.home_env()).contains(&acct.path.display().to_string())
}

/// Move `src` to `backup`, adding `.1`, `.2`… if that name is taken.
fn stash(src: &Path, backup: &Path) -> Result<()> {
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut dest = backup.to_path_buf();
    let mut n = 1;
    while exists_or_link(&dest) {
        let name = backup
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        dest = backup.with_file_name(format!("{name}.{n}"));
        n += 1;
    }
    move_path(src, &dest)
}

/// `shutil.move`: rename, or copy + remove across file systems.
fn move_path(src: &Path, dst: &Path) -> Result<()> {
    if fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    copy_tree(src, dst)?;
    let meta = fs::symlink_metadata(src)?;
    if meta.is_dir() {
        fs::remove_dir_all(src)?;
    } else {
        fs::remove_file(src)?;
    }
    Ok(())
}

fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(src)?;
    if meta.file_type().is_symlink() {
        let target = fs::read_link(src)?;
        make_symlink(&target, dst)?;
    } else if meta.is_dir() {
        fs::create_dir_all(dst)?;
        for e in fs::read_dir(src)? {
            let e = e?;
            copy_tree(&e.path(), &dst.join(e.file_name()))?;
        }
    } else {
        fs::copy(src, dst)?;
    }
    Ok(())
}

fn line_stamp(line: &str) -> f64 {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
        return 0.0;
    };
    let Some(o) = v.as_object() else { return 0.0 };
    let pick = ["timestamp", "ts", "updated_at"]
        .iter()
        .find_map(|k| o.get(*k).filter(|x| truthy(x)));
    pick.and_then(parse_ts)
        .map(|t| t.timestamp_micros() as f64 / 1e6)
        .unwrap_or(0.0)
}

fn truthy(v: &serde_json::Value) -> bool {
    !matches!(v, serde_json::Value::Null)
        && v != &serde_json::json!("")
        && v != &serde_json::json!(0)
        && v != &serde_json::json!(false)
}

/// Append `src` lines that `dst` lacks, oldest first.
fn append_missing_lines(src: &Path, dst: &Path) -> Result<()> {
    let have_text = String::from_utf8_lossy(&fs::read(dst)?).to_string();
    let have: HashSet<&str> = have_text.lines().collect();
    let src_text = String::from_utf8_lossy(&fs::read(src)?).to_string();
    let mut seen = HashSet::new();
    let mut new: Vec<&str> = src_text
        .lines()
        .filter(|l| !l.trim().is_empty() && !have.contains(l))
        .filter(|l| seen.insert(*l))
        .collect();
    if new.is_empty() {
        return Ok(());
    }
    // Stable sort, like Python's sorted(key=stamp).
    new.sort_by(|a, b| {
        line_stamp(a)
            .partial_cmp(&line_stamp(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    use std::io::Write;
    let mut f = fs::OpenOptions::new().append(true).open(dst)?;
    let mut buf = String::new();
    for l in new {
        buf.push_str(l);
        buf.push('\n');
    }
    f.write_all(buf.as_bytes())?;
    Ok(())
}

fn merge_into(src: &Path, dst: &Path, backup: &Path, deep: bool) -> Result<()> {
    if !exists_or_link(dst) {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        return move_path(src, dst);
    }
    let src_is_real_dir = src.is_dir() && !is_symlink(src);
    if src_is_real_dir && dst.is_dir() {
        let mut children: Vec<_> = fs::read_dir(src)?.flatten().map(|e| e.path()).collect();
        children.sort();
        for child in children {
            let name = child
                .file_name()
                .map(|n| n.to_os_string())
                .unwrap_or_default();
            let child_dir = child.is_dir() && !is_symlink(&child);
            if child_dir && dst.join(&name).is_dir() && !deep {
                stash(&child, &backup.join(&name))?;
            } else {
                merge_into(&child, &dst.join(&name), &backup.join(&name), deep)?;
            }
        }
        if src.exists() {
            fs::remove_dir(src)?;
        }
        return Ok(());
    }
    if src.is_file() && dst.is_file() {
        let name = src
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if APPEND_LOGS.contains(&name.as_str()) {
            append_missing_lines(src, dst)?;
        } else if deep
            && src.extension().is_some_and(|e| e == "jsonl")
            && fs::metadata(src)?.len() > fs::metadata(dst)?.len()
        {
            let bname = backup
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            stash(dst, &backup.with_file_name(format!("{bname}.central")))?;
            move_path(src, dst)?;
            return Ok(());
        }
    }
    if exists_or_link(src) {
        stash(src, backup)?;
    }
    Ok(())
}

#[cfg(unix)]
fn make_symlink(target: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link)
        .with_context(|| format!("linking {} -> {}", link.display(), target.display()))
}

#[cfg(windows)]
fn make_symlink(target: &Path, link: &Path) -> Result<()> {
    use std::os::windows::fs::{symlink_dir, symlink_file};
    if target.is_dir() {
        if symlink_dir(target, link).is_ok() {
            return Ok(());
        }
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .status()?;
        anyhow::ensure!(
            status.success(),
            "Could not create a junction for {}",
            link.display()
        );
        return Ok(());
    }
    symlink_file(target, link).map_err(|e| {
        anyhow::anyhow!(
            "Could not link {}: {e}. Turn on Developer Mode in Windows settings to allow symlinks.",
            link.display()
        )
    })
}

/// Link pending items; returns what changed (or would change, `dry_run`).
pub fn share(ctx: &Ctx, kind: Kind, acct: &mut Account, dry_run: bool) -> Result<Vec<String>> {
    let central = kind.central(ctx);
    if same_dir(&acct.path, &central) {
        return Ok(Vec::new());
    }
    let pending: Vec<&str> = kind
        .shared()
        .iter()
        .copied()
        .filter(|n| needs_link(ctx, kind, acct, n))
        .collect();
    if dry_run {
        return Ok(pending.iter().map(|n| format!("would link {n}")).collect());
    }
    if is_running(kind, acct) {
        return Ok(if pending.is_empty() {
            Vec::new()
        } else {
            vec![format!(
                "{} is running; close its sessions and share again",
                acct.name
            )]
        });
    }
    if pending.is_empty() {
        return after_share(ctx, kind, acct);
    }
    let backup = ctx
        .backup_root
        .join(chrono::Local::now().format("%Y%m%d-%H%M%S").to_string())
        .join(kind.as_str())
        .join(&acct.name);
    let mut log = Vec::new();
    if kind == Kind::Codex {
        codex_merge_dbs(ctx, acct, &mut log)?;
    }
    for name in pending {
        let link = acct.path.join(name);
        let target = central.join(name);
        if is_symlink(&link) {
            fs::remove_file(&link)?;
        } else if link.exists() {
            let deep = kind.data_dirs().contains(&name);
            merge_into(&link, &target, &backup.join(name), deep)?;
        }
        for side in [format!("{name}-wal"), format!("{name}-shm")] {
            let p = acct.path.join(&side);
            if p.exists() {
                stash(&p, &backup.join(&side))?;
            }
        }
        make_symlink(&target, &link)?;
        log.push(format!("linked {name}"));
    }
    if backup.exists() {
        log.push(format!(
            "replaced copies kept in {}",
            tilde(&backup, &ctx.home)
        ));
    }
    log.extend(after_share(ctx, kind, acct)?);
    Ok(log)
}

fn after_share(ctx: &Ctx, kind: Kind, acct: &mut Account) -> Result<Vec<String>> {
    match kind {
        Kind::Claude => crate::claude::after_share(ctx, acct),
        Kind::Codex => codex_repoint(ctx, acct),
    }
}

/// Copy the account's thread rows into the central dbs, pointing rollouts
/// at the central home (Python `CodexProvider.before_share`).
fn codex_merge_dbs(ctx: &Ctx, acct: &Account, log: &mut Vec<String>) -> Result<()> {
    let central = Kind::Codex.central(ctx);
    let tables: &[(&str, &[&str])] = &[
        ("state_5.sqlite", &["threads", "thread_spawn_edges"]),
        ("goals_1.sqlite", &["thread_goals"]),
    ];
    for (name, tables) in tables {
        let src = acct.path.join(name);
        let dst = central.join(name);
        if is_symlink(&src) || !src.exists() || !dst.exists() {
            continue;
        }
        {
            let c = Connection::open(&src)?;
            c.busy_timeout(std::time::Duration::from_secs(30))?;
            let _ = c.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()));
        }
        let conn = Connection::open(&dst)?;
        conn.busy_timeout(std::time::Duration::from_secs(30))?;
        conn.execute("ATTACH DATABASE ?1 AS acct", [src.display().to_string()])?;
        let mut copied = 0usize;
        for table in *tables {
            let cols_of = |schema: &str| -> rusqlite::Result<Vec<String>> {
                conn.prepare(&format!("PRAGMA {schema}.table_info(\"{table}\")"))?
                    .query_map([], |r| r.get::<_, String>(1))?
                    .collect()
            };
            let central_cols: HashSet<String> = cols_of("main")?.into_iter().collect();
            let cols: Vec<String> = cols_of("acct")?
                .into_iter()
                .filter(|c| central_cols.contains(c))
                .collect();
            if cols.is_empty() {
                continue;
            }
            let names = cols
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ");
            let select = cols
                .iter()
                .map(|c| {
                    if c == "rollout_path" {
                        "replace(rollout_path, ?1, ?2)".to_string()
                    } else {
                        format!("\"{c}\"")
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "INSERT OR IGNORE INTO main.\"{table}\" ({names}) SELECT {select} FROM acct.\"{table}\""
            );
            copied += if cols.iter().any(|c| c == "rollout_path") {
                conn.execute(
                    &sql,
                    [
                        format!("{}/", acct.path.display()),
                        format!("{}/", central.display()),
                    ],
                )?
            } else {
                conn.execute(&sql, [])?
            };
        }
        conn.execute("DETACH DATABASE acct", [])?;
        if copied > 0 {
            log.push(format!("merged {copied} rows into {name}"));
        }
    }
    Ok(())
}

/// Repoint rollout paths written through this account's links to the
/// central home (Python `CodexProvider.after_share`).
pub fn codex_repoint(ctx: &Ctx, acct: &Account) -> Result<Vec<String>> {
    let central = Kind::Codex.central(ctx);
    let db = central.join("state_5.sqlite");
    if !db.exists() {
        return Ok(Vec::new());
    }
    let prefix = format!("{}/", acct.path.display());
    let conn = Connection::open(&db)?;
    conn.busy_timeout(std::time::Duration::from_secs(30))?;
    let n = conn.execute(
        "UPDATE threads SET rollout_path = ?1 || substr(rollout_path, ?2) WHERE substr(rollout_path, 1, ?3) = ?4",
        rusqlite::params![
            format!("{}/", central.display()),
            prefix.chars().count() as i64 + 1,
            prefix.chars().count() as i64,
            prefix
        ],
    )?;
    Ok(if n > 0 {
        vec![format!(
            "repointed {n} thread paths to {}",
            tilde(&central, &ctx.home)
        )]
    } else {
        Vec::new()
    })
}

/// Everything a launch does before the process starts: make the home,
/// share (errors never block), record the launch, mark it active.
pub fn prepare_launch(ctx: &Ctx, kind: Kind, name: &str, bump: bool) -> Result<Vec<String>> {
    let path = kind.account_dir(ctx, name);
    fs::create_dir_all(&path)?;
    let mut acct = kind.find(ctx, name)?;
    let log =
        share(ctx, kind, &mut acct, false).unwrap_or_else(|e| vec![format!("share failed: {e:#}")]);
    if bump {
        let fresh = kind.find(ctx, name)?;
        kind.bump(ctx, &fresh)?;
    }
    kind.set_active(ctx, Some(name))?;
    Ok(log)
}

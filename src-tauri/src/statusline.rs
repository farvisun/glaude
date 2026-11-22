//! "Use my statusline script": feed the user's Claude `statusLine.command`
//! the same JSON Claude Code gives it and show its output (ANSI → HTML).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use glaude_accounts::paths;
use glaude_core::{ansi, env};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub used: f64,
    pub resets_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusInput {
    pub account: String,
    pub cwd: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub version: Option<String>,
    pub context_used: Option<u64>,
    pub context_window: Option<u64>,
    pub cost_usd: Option<f64>,
    pub lines_added: Option<u64>,
    pub lines_removed: Option<u64>,
    pub five_hour: Option<Window>,
    pub week: Option<Window>,
}

/// "claude-opus-5-5[1m]" → "Opus 5.5"; aliases ("opus") → "Opus".
fn display_name(model: &str) -> String {
    let base = model.split('[').next().unwrap_or(model);
    let base = base.strip_prefix("claude-").unwrap_or(base);
    let mut parts: Vec<&str> = base.split('-').collect();
    if parts
        .last()
        .is_some_and(|p| p.len() == 8 && p.chars().all(|c| c.is_ascii_digit()))
    {
        parts.pop();
    }
    let Some((family, version)) = parts.split_first() else {
        return model.to_string();
    };
    let mut name: String = family
        .chars()
        .enumerate()
        .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
        .collect();
    if !version.is_empty() {
        name.push(' ');
        name.push_str(&version.join("."));
    }
    name
}

fn epoch(iso: &Option<String>) -> Value {
    iso.as_deref()
        .and_then(glaude_accounts::util::parse_ts_str)
        .map(|t| json!(t.timestamp()))
        .unwrap_or(Value::Null)
}

fn build_input(i: &StatusInput) -> Value {
    let model = i.model.clone().unwrap_or_default();
    let pct = match (i.context_used, i.context_window) {
        (Some(u), Some(m)) if m > 0 => json!((u as f64 / m as f64 * 100.0).round()),
        _ => Value::Null,
    };
    let window = |w: &Option<Window>| {
        w.as_ref()
            .map(|w| json!({"used_percentage": w.used, "resets_at": epoch(&w.resets_at)}))
            .unwrap_or(Value::Null)
    };
    json!({
        "hook_event_name": "Status",
        "session_id": i.session_id,
        "transcript_path": "",
        "cwd": i.cwd,
        "model": {"id": model, "display_name": display_name(&model)},
        "workspace": {"current_dir": i.cwd, "project_dir": i.cwd},
        "version": i.version,
        "output_style": {"name": "default"},
        "cost": {
            "total_cost_usd": i.cost_usd,
            "total_duration_ms": 0,
            "total_api_duration_ms": 0,
            "total_lines_added": i.lines_added,
            "total_lines_removed": i.lines_removed,
        },
        "context_window": {"used_tokens": i.context_used, "max_tokens": i.context_window, "used_percentage": pct},
        "exceeds_200k_tokens": i.context_used.unwrap_or(0) > 200_000,
        "rate_limits": {"five_hour": window(&i.five_hour), "seven_day": window(&i.week)},
        "effort": {"level": i.effort},
    })
}

/// `statusLine.command` from the account's settings.json, `~` expanded.
fn command_for(account: &str) -> Option<String> {
    let settings = paths::claude_home(account).join("settings.json");
    let v: Value = serde_json::from_slice(&std::fs::read(settings).ok()?).ok()?;
    let sl = v.get("statusLine")?;
    if sl.get("type").and_then(Value::as_str).unwrap_or("command") != "command" {
        return None;
    }
    let cmd = sl.get("command")?.as_str()?.trim().to_string();
    if cmd.is_empty() {
        return None;
    }
    let home = env::home_dir().display().to_string();
    Some(if let Some(rest) = cmd.strip_prefix("~/") {
        format!("{home}/{rest}")
    } else {
        cmd
    })
}

fn run(cmd: &str, account: &str, cwd: &str, input: &[u8]) -> Option<String> {
    let mut c = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", cmd]);
        c
    } else {
        let mut c = Command::new("/bin/sh");
        c.args(["-c", cmd]);
        c
    };
    let mut overrides: Vec<(&str, Option<String>)> = Vec::new();
    if let Some(dir) = paths::claude_config_dir(account) {
        overrides.push(("CLAUDE_CONFIG_DIR", Some(dir.display().to_string())));
    }
    c.env_clear()
        .envs(env::agent_env(&overrides))
        .current_dir(if PathBuf::from(cwd).is_dir() {
            PathBuf::from(cwd)
        } else {
            env::home_dir()
        })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = c.spawn().ok()?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input);
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(15)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let out = child.wait_with_output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    text.lines()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim_end().to_string())
}

/// HTML for the statusline, or `None` when no script is configured or it
/// failed (the built-in statusline is shown then).
#[tauri::command]
pub async fn statusline_render(input: StatusInput) -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(move || {
        let cmd = command_for(&input.account)?;
        let json = serde_json::to_vec(&build_input(&input)).ok()?;
        let line = run(&cmd, &input.account, &input.cwd, &json)?;
        Some(ansi::to_html(&line))
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_names() {
        assert_eq!(display_name("claude-opus-5-5"), "Opus 5.5");
        assert_eq!(display_name("claude-haiku-4-5-20251001"), "Haiku 4.5");
        assert_eq!(display_name("claude-fable-5-2[1m]"), "Fable 5.2");
        assert_eq!(display_name("opus"), "Opus");
    }

    #[cfg(unix)]
    #[test]
    fn runs_a_script_with_the_json() {
        let out = run(
            r#"read -r j; printf '\033[32m%s\033[0m\n' "$(printf '%s' "$j" | head -c 22)""#,
            "default",
            "/",
            br#"{"hook_event_name":"Status"}"#,
        )
        .expect("output");
        assert!(out.contains("hook_event_name"), "{out}");
        assert!(ansi::to_html(&out).starts_with("<span class=\"ansi-fg-2\">"));
    }
}

#[cfg(test)]
mod live {
    use super::*;

    /// Runs the configured script of the default account, if any:
    /// cargo test -p glaude --lib statusline::live -- --ignored --nocapture
    #[test]
    #[ignore]
    fn users_script() {
        let Some(cmd) = command_for("default") else {
            eprintln!("no statusLine.command configured");
            return;
        };
        let input = StatusInput {
            account: "default".into(),
            cwd: env!("CARGO_MANIFEST_DIR").into(),
            session_id: Some("00000000-0000-0000-0000-000000000000".into()),
            model: Some("claude-opus-5-5".into()),
            effort: Some("high".into()),
            version: Some("2.1.329".into()),
            context_used: Some(84_000),
            context_window: Some(200_000),
            cost_usd: Some(1.23),
            lines_added: Some(120),
            lines_removed: Some(7),
            five_hour: Some(Window {
                used: 42.0,
                resets_at: Some("2026-11-22T18:00:00Z".into()),
            }),
            week: Some(Window {
                used: 63.5,
                resets_at: None,
            }),
        };
        let json = serde_json::to_vec(&build_input(&input)).unwrap();
        let line = run(&cmd, "default", &input.cwd, &json).expect("script output");
        eprintln!("ANSI: {line:?}\nHTML: {}", ansi::to_html(&line));
    }
}

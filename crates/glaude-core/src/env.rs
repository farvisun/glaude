//! Process environment for spawned agents.
//!
//! A GUI app started from Finder or a desktop launcher does not inherit the
//! user's shell PATH, so `claude`/`codex` would not be found. We read the
//! login shell environment once at startup and use it for every child.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

pub type EnvMap = BTreeMap<String, String>;

static LOGIN_ENV: OnceLock<EnvMap> = OnceLock::new();

// Only the Unix login-shell loader (and its tests) read the shell dump.
#[cfg_attr(not(unix), allow(dead_code))]
const MARKER: &str = "__GLAUDE_ENV_BEGIN__";

/// Variables that must never leak from Glaude's own environment into an
/// agent: they would switch billing to an API key or make the CLI think it
/// is a nested session.
pub fn is_scrubbed(key: &str) -> bool {
    matches!(
        key,
        "ANTHROPIC_API_KEY" | "ANTHROPIC_AUTH_TOKEN" | "CLAUDE_PID" | "NODE_OPTIONS" | "CLAUDECODE"
    ) || key.starts_with("CLAUDE_CODE_")
}

/// The login shell environment, merged over the current process environment.
pub fn login_env() -> &'static EnvMap {
    LOGIN_ENV.get_or_init(|| {
        let mut env: EnvMap = std::env::vars().collect();
        match read_login_shell_env() {
            Ok(shell) => {
                tracing::info!(vars = shell.len(), "loaded login shell environment");
                env.extend(shell);
            }
            Err(err) => tracing::warn!(%err, "could not read login shell environment"),
        }
        env
    })
}

#[cfg(unix)]
fn read_login_shell_env() -> anyhow::Result<EnvMap> {
    use std::io::Read;
    use std::process::Stdio;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let script = format!("printf '%s\\0' '{MARKER}'; env -0");
    let mut child = Command::new(&shell)
        .args(["-ilc", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut out = child.stdout.take().expect("piped stdout");
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = out.read_to_end(&mut buf);
        buf
    });

    // Interactive shells with heavy rc files can hang; never block startup.
    let deadline = std::time::Instant::now() + Duration::from_secs(6);
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            anyhow::bail!("login shell timed out");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let buf = reader.join().unwrap_or_default();
    Ok(parse_env_dump(&buf))
}

#[cfg(not(unix))]
fn read_login_shell_env() -> anyhow::Result<EnvMap> {
    Ok(EnvMap::new())
}

#[cfg_attr(not(unix), allow(dead_code))]
fn parse_env_dump(buf: &[u8]) -> EnvMap {
    let text = String::from_utf8_lossy(buf);
    let after = match text.find(MARKER) {
        Some(i) => &text[i + MARKER.len()..],
        None => return EnvMap::new(),
    };
    after
        .split('\0')
        .filter_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            let k = k.trim_start_matches(['\n', '\r']);
            (!k.is_empty() && !k.contains(char::is_whitespace))
                .then(|| (k.to_string(), v.to_string()))
        })
        .collect()
}

/// Environment for an agent process: login env minus scrubbed keys, plus
/// the given overrides; `None` values remove a key.
pub fn agent_env(overrides: &[(&str, Option<String>)]) -> EnvMap {
    let mut env: EnvMap = login_env()
        .iter()
        .filter(|(k, _)| !is_scrubbed(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (k, v) in overrides {
        match v {
            Some(v) => {
                env.insert((*k).to_string(), v.clone());
            }
            None => {
                env.remove(*k);
            }
        }
    }
    env
}

pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

/// Look a program up on the login PATH.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = login_env().get("PATH").cloned().unwrap_or_default();
    let exts: &[&str] = if cfg!(windows) {
        &["exe", "cmd", "bat", ""]
    } else {
        &[""]
    };
    for dir in std::env::split_paths(&path) {
        for ext in exts {
            let mut p = dir.join(name);
            if !ext.is_empty() {
                p.set_extension(ext);
            }
            if is_executable(&p) {
                return Some(p);
            }
        }
    }
    None
}

/// Resolve an agent binary: explicit env override, known install
/// locations, then PATH.
pub fn resolve_binary(name: &str, env_key: &str, candidates: &[PathBuf]) -> Option<PathBuf> {
    if let Some(v) = login_env().get(env_key).filter(|v| !v.is_empty()) {
        let p = PathBuf::from(v);
        if is_executable(&p) {
            return Some(p);
        }
        if let Some(found) = which(v) {
            return Some(found);
        }
    }
    candidates
        .iter()
        .find(|p| is_executable(p))
        .cloned()
        .or_else(|| which(name))
}

pub fn claude_binary() -> Option<PathBuf> {
    let home = home_dir();
    let exe = if cfg!(windows) {
        "claude.exe"
    } else {
        "claude"
    };
    resolve_binary("claude", "CLAUDE_BIN", &[home.join(".local/bin").join(exe)])
}

pub fn codex_binary() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if cfg!(target_os = "macos") {
        candidates.push(PathBuf::from("/opt/homebrew/bin/codex"));
        candidates.push(PathBuf::from("/usr/local/bin/codex"));
    }
    resolve_binary("codex", "CODEX_BIN", &candidates)
}

/// `claude --version` style probe, first line of stdout. Gives up after
/// 5 s: a binary can stall before it runs (e.g. a macOS signature check
/// waiting on the network), and nothing should wait on that forever.
pub fn binary_version(bin: &Path) -> Option<String> {
    binary_version_within(bin, Duration::from_secs(5))
}

fn binary_version_within(bin: &Path, limit: Duration) -> Option<String> {
    use std::io::Read;
    use std::process::Stdio;
    let mut child = Command::new(bin)
        .arg("--version")
        .envs(agent_env(&[]))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = std::time::Instant::now() + limit;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25))
            }
            _ => {
                tracing::warn!("{} --version did not finish in {limit:?}", bin.display());
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    out.lines().next().map(|l| l.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn version_probe_gives_up_on_a_stalled_binary() {
        let dir = std::env::temp_dir().join(format!("glaude-probe-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ok = dir.join("ok");
        let slow = dir.join("slow");
        std::fs::write(&ok, "#!/bin/sh\necho 'tool 1.2.3'\n").unwrap();
        std::fs::write(&slow, "#!/bin/sh\nexec sleep 30\n").unwrap();
        for f in [&ok, &slow] {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(f, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert_eq!(binary_version(&ok).as_deref(), Some("tool 1.2.3"));
        let t = std::time::Instant::now();
        assert_eq!(
            binary_version_within(&slow, std::time::Duration::from_millis(300)),
            None
        );
        assert!(t.elapsed() < std::time::Duration::from_secs(5));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn parses_env_dump_after_marker() {
        let dump = format!("motd noise\n{MARKER}\0PATH=/usr/bin:/bin\0HOME=/Users/x\0MULTI=a\nb\0");
        let env = parse_env_dump(dump.as_bytes());
        assert_eq!(env.get("PATH").unwrap(), "/usr/bin:/bin");
        assert_eq!(env.get("MULTI").unwrap(), "a\nb");
        assert_eq!(env.len(), 3);
    }

    #[test]
    fn scrubs_billing_and_nesting_vars() {
        assert!(is_scrubbed("ANTHROPIC_API_KEY"));
        assert!(is_scrubbed("CLAUDE_CODE_ENTRYPOINT"));
        assert!(!is_scrubbed("CLAUDE_CONFIG_DIR"));
        assert!(!is_scrubbed("PATH"));
    }
}

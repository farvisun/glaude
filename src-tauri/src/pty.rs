//! Terminal tabs: a PTY per tab (portable-pty, ConPTY on Windows). Output
//! goes to the webview as raw bytes; xterm.js decodes UTF-8 itself, so
//! multi-byte characters split across reads are fine.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use glaude_accounts::paths;
use glaude_core::env;
use parking_lot::Mutex;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Deserialize;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::State;

struct Pty {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
}

#[derive(Default)]
pub struct Ptys(Mutex<HashMap<String, Arc<Pty>>>);

/// What runs in the terminal: a login shell, or a CLI's own TUI.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSpec {
    pub provider: String,
    #[serde(default = "default_account")]
    pub account: String,
    pub resume: Option<String>,
    /// Start the tool's login flow (new accounts).
    #[serde(default)]
    pub login: bool,
}

fn default_account() -> String {
    "default".into()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtySpec {
    pub cwd: Option<String>,
    pub cols: u16,
    pub rows: u16,
    pub run: Option<RunSpec>,
}

fn shell() -> (PathBuf, Vec<String>) {
    if cfg!(windows) {
        for exe in ["pwsh.exe", "powershell.exe"] {
            if let Some(p) = env::which(exe) {
                return (p, vec!["-NoLogo".into()]);
            }
        }
        return (PathBuf::from("cmd.exe"), Vec::new());
    }
    let sh = env::login_env()
        .get("SHELL")
        .cloned()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/bin/zsh".into());
    (PathBuf::from(sh), vec!["-l".into()])
}

fn command(spec: &PtySpec) -> Result<CommandBuilder, String> {
    let mut overrides: Vec<(&str, Option<String>)> = vec![
        ("TERM", Some("xterm-256color".into())),
        ("COLORTERM", Some("truecolor".into())),
        ("TERM_PROGRAM", Some("glaude".into())),
    ];
    if env::login_env().get("LANG").is_none_or(|l| l.is_empty()) {
        overrides.push(("LANG", Some("en_US.UTF-8".into())));
    }
    let (program, args) = match &spec.run {
        None => shell(),
        Some(run) if run.provider == "claude" => {
            let bin = env::claude_binary().ok_or("Claude Code CLI not found")?;
            if let Some(dir) = paths::claude_config_dir(&run.account) {
                overrides.push(("CLAUDE_CONFIG_DIR", Some(dir.display().to_string())));
            }
            let mut args = Vec::new();
            if let Some(sid) = &run.resume {
                args.extend(["--resume".to_string(), sid.clone()]);
            }
            (bin, args)
        }
        Some(run) if run.provider == "codex" => {
            let bin = env::codex_binary().ok_or("Codex CLI not found")?;
            overrides.push((
                "CODEX_HOME",
                Some(paths::codex_home(&run.account).display().to_string()),
            ));
            let args = if run.login {
                vec!["login".to_string()]
            } else if let Some(id) = &run.resume {
                vec!["resume".to_string(), id.clone()]
            } else {
                Vec::new()
            };
            (bin, args)
        }
        Some(run) => return Err(format!("Unknown tool {}", run.provider)),
    };
    let mut cmd = CommandBuilder::new(program);
    cmd.args(args);
    cmd.env_clear();
    for (k, v) in env::agent_env(&overrides) {
        cmd.env(k, v);
    }
    let cwd = spec
        .cwd
        .as_deref()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(env::home_dir);
    cmd.cwd(cwd);
    Ok(cmd)
}

#[tauri::command]
pub fn pty_open(
    ptys: State<'_, Ptys>,
    id: String,
    spec: PtySpec,
    on_data: Channel<InvokeResponseBody>,
    on_exit: Channel<Option<i32>>,
) -> Result<(), String> {
    if ptys.0.lock().contains_key(&id) {
        return Ok(());
    }
    let size = PtySize {
        rows: spec.rows.max(2),
        cols: spec.cols.max(10),
        pixel_width: 0,
        pixel_height: 0,
    };
    let pair = native_pty_system()
        .openpty(size)
        .map_err(|e| format!("Could not open a terminal: {e}"))?;
    let mut child = pair
        .slave
        .spawn_command(command(&spec)?)
        .map_err(|e| format!("Could not start the shell: {e}"))?;
    // The reader only sees EOF once every slave handle is closed.
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
    let killer = child.clone_killer();

    std::thread::Builder::new()
        .name(format!("pty-read-{id}"))
        .spawn(move || {
            let mut buf = vec![0u8; 64 * 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if on_data
                            .send(InvokeResponseBody::Raw(buf[..n].to_vec()))
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
        })
        .map_err(|e| e.to_string())?;
    std::thread::Builder::new()
        .name(format!("pty-wait-{id}"))
        .spawn(move || {
            let code = child.wait().ok().map(|s| s.exit_code() as i32);
            let _ = on_exit.send(code);
        })
        .map_err(|e| e.to_string())?;

    ptys.0.lock().insert(
        id,
        Arc::new(Pty {
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
            killer: Mutex::new(killer),
        }),
    );
    Ok(())
}

fn get(ptys: &Ptys, id: &str) -> Result<Arc<Pty>, String> {
    ptys.0
        .lock()
        .get(id)
        .cloned()
        .ok_or_else(|| "This terminal has ended.".to_string())
}

#[tauri::command]
pub fn pty_write(ptys: State<'_, Ptys>, id: String, data: String) -> Result<(), String> {
    let pty = get(&ptys, &id)?;
    let mut w = pty.writer.lock();
    w.write_all(data.as_bytes())
        .and_then(|_| w.flush())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_resize(ptys: State<'_, Ptys>, id: String, cols: u16, rows: u16) -> Result<(), String> {
    let pty = get(&ptys, &id)?;
    let size = PtySize {
        rows: rows.max(2),
        cols: cols.max(10),
        pixel_width: 0,
        pixel_height: 0,
    };
    let res = pty.master.lock().resize(size);
    res.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_close(ptys: State<'_, Ptys>, id: String) {
    let pty = ptys.0.lock().remove(&id);
    if let Some(p) = pty {
        let _ = p.killer.lock().kill();
    }
}

pub fn close_all(ptys: &Ptys) {
    for (_, p) in ptys.0.lock().drain() {
        let _ = p.killer.lock().kill();
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// The user's login shell runs in a PTY, keeps UTF-8 intact and exits.
    #[test]
    fn login_shell_round_trip() {
        let spec = PtySpec {
            cwd: None,
            cols: 100,
            rows: 30,
            run: None,
        };
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 30,
                cols: 100,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut child = pair.slave.spawn_command(command(&spec).unwrap()).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let mut writer = pair.master.take_writer().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut all = Vec::new();
            let mut buf = [0u8; 8192];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                all.extend_from_slice(&buf[..n]);
                if String::from_utf8_lossy(&all).contains("done:سلام") {
                    break;
                }
            }
            let _ = tx.send(String::from_utf8_lossy(&all).to_string());
        });
        writer
            .write_all("printf 'done:%s\\n' \"$(printf '\\330\\263\\331\\204\\330\\247\\331\\205')\"; exit\r".as_bytes())
            .unwrap();
        let out = rx
            .recv_timeout(std::time::Duration::from_secs(20))
            .expect("shell output");
        assert!(out.contains("done:سلام"), "{out}");
        let _ = child.wait();
    }
}

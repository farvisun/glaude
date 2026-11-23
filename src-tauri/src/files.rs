//! Paths mentioned in replies: which ones exist, and opening them in an
//! editor, the file manager, or the app's own read-only viewer.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PathHit {
    pub abs: String,
    pub dir: bool,
    pub line: Option<u32>,
    pub col: Option<u32>,
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Split `path:12:3`, `path#L12` or `path(12,3)` into the path and position.
fn split_position(s: &str) -> (&str, Option<u32>, Option<u32>) {
    if let Some((p, l)) = s.rsplit_once("#L") {
        let line = l.split(['-', 'C']).next().and_then(|v| v.parse().ok());
        return (p, line, None);
    }
    let mut rest = s;
    let mut nums = Vec::new();
    while let Some((p, n)) = rest.rsplit_once(':') {
        match n.parse::<u32>() {
            Ok(v) if nums.len() < 2 => {
                nums.push(v);
                rest = p;
            }
            _ => break,
        }
    }
    nums.reverse();
    (rest, nums.first().copied(), nums.get(1).copied())
}

/// Resolve one candidate from a reply against the conversation folder.
pub fn resolve(cwd: &Path, raw: &str, home: &Path) -> Option<PathHit> {
    let raw = raw
        .trim()
        .trim_end_matches(['.', ',', ';', ')', '`', '\'', '"']);
    if raw.is_empty()
        || raw.len() > 1024
        || raw.contains('\n')
        || raw.contains("://") && !raw.starts_with("file://")
    {
        return None;
    }
    let raw = match raw.strip_prefix("file://") {
        Some(rest) => percent_decode(rest.strip_prefix("localhost").unwrap_or(rest)),
        None => raw.to_string(),
    };
    let (path, line, col) = split_position(&raw);
    let p = if let Some(rest) = path.strip_prefix("~/") {
        home.join(rest)
    } else {
        let p = PathBuf::from(path);
        if p.is_absolute() {
            p
        } else {
            cwd.join(p)
        }
    };
    let meta = std::fs::metadata(&p).ok()?;
    Some(PathHit {
        abs: p.display().to_string(),
        dir: meta.is_dir(),
        line,
        col,
    })
}

/// Which of the candidates exist (same order; null for the rest).
#[tauri::command]
pub async fn paths_check(cwd: String, candidates: Vec<String>) -> Vec<Option<PathHit>> {
    tokio::task::spawn_blocking(move || {
        let home = glaude_core::env::home_dir();
        let cwd = PathBuf::from(cwd);
        candidates
            .iter()
            .take(500)
            .map(|c| resolve(&cwd, c, &home))
            .collect()
    })
    .await
    .unwrap_or_default()
}

/// Files the system would run instead of show: only revealed, never opened.
fn runs_as_program(p: &Path) -> bool {
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    const RISKY: &[&str] = &[
        "app",
        "command",
        "tool",
        "sh",
        "bash",
        "zsh",
        "fish",
        "py",
        "rb",
        "pl",
        "pkg",
        "dmg",
        "workflow",
        "scpt",
        "applescript",
        "terminal",
        "exe",
        "bat",
        "cmd",
        "com",
        "msi",
        "ps1",
        "vbs",
        "js",
        "jar",
        "desktop",
        "appimage",
        "run",
        "lnk",
        "url",
        "webloc",
        "inetloc",
    ];
    RISKY.contains(&ext.as_str()) || has_exec_bit(p)
}

/// A file with an execute bit. Windows has none: the extension decides.
fn has_exec_bit(p: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        let _ = p;
        false
    }
}

fn editor_cli(editor: &str) -> Option<(&'static str, &'static str)> {
    // (command, macOS app name)
    match editor {
        "vscode" => Some(("code", "Visual Studio Code")),
        "cursor" => Some(("cursor", "Cursor")),
        "zed" => Some(("zed", "Zed")),
        _ => None,
    }
}

fn app_installed(name: &str) -> bool {
    cfg!(target_os = "macos")
        && (Path::new(&format!("/Applications/{name}.app")).exists()
            || glaude_core::env::home_dir()
                .join(format!("Applications/{name}.app"))
                .exists())
}

/// Editors that can be chosen in Settings.
#[tauri::command]
pub fn editors_available() -> Vec<String> {
    ["vscode", "cursor", "zed"]
        .into_iter()
        .filter(|e| {
            let (cli, app) = editor_cli(e).unwrap_or_default();
            glaude_core::env::which(cli).is_some() || app_installed(app)
        })
        .map(str::to_string)
        .collect()
}

fn open_in_editor(
    editor: &str,
    p: &Path,
    line: Option<u32>,
    col: Option<u32>,
) -> Result<(), String> {
    let (cli, app) = editor_cli(editor).ok_or("Unknown editor")?;
    let target = match (line, col) {
        (Some(l), Some(c)) => format!("{}:{l}:{c}", p.display()),
        (Some(l), None) => format!("{}:{l}", p.display()),
        _ => p.display().to_string(),
    };
    let env = glaude_core::env::agent_env(&[]);
    if let Some(bin) = glaude_core::env::which(cli) {
        let mut cmd = std::process::Command::new(bin);
        if editor != "zed" {
            cmd.arg("-g");
        }
        cmd.arg(&target)
            .env_clear()
            .envs(env)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        return Ok(());
    }
    if app_installed(app) {
        let status = std::process::Command::new("open")
            .args(["-a", app])
            .arg(p)
            .status()
            .map_err(|e| e.to_string())?;
        return if status.success() {
            Ok(())
        } else {
            Err(format!("{app} did not open the file"))
        };
    }
    Err(format!("{app} is not installed (no `{cli}` command found)"))
}

/// Open a path: `editor` (with the chosen editor), `default` (the system's
/// app for it), or `reveal` (show it in the file manager).
#[tauri::command]
pub async fn open_path(
    app: AppHandle,
    path: String,
    how: String,
    editor: Option<String>,
    line: Option<u32>,
    col: Option<u32>,
) -> Result<String, String> {
    let p = PathBuf::from(&path);
    if !p.is_absolute() || !p.exists() {
        return Err("That path does not exist".into());
    }
    let reveal = |app: &AppHandle| {
        app.opener()
            .reveal_item_in_dir(&p)
            .map_err(|e| e.to_string())
    };
    match how.as_str() {
        "reveal" => reveal(&app).map(|_| "reveal".into()),
        "editor" if !p.is_dir() || editor.as_deref().is_some_and(|e| e != "default") => {
            match editor.as_deref().unwrap_or("default") {
                "default" => open_default(&app, &p),
                e => {
                    let (e, p2) = (e.to_string(), p.clone());
                    tokio::task::spawn_blocking(move || open_in_editor(&e, &p2, line, col))
                        .await
                        .map_err(|e| e.to_string())?
                        .map(|_| "editor".into())
                }
            }
        }
        _ => open_default(&app, &p),
    }
}

fn open_default(app: &AppHandle, p: &Path) -> Result<String, String> {
    if runs_as_program(p) {
        // Never start a program from a link in a reply.
        app.opener()
            .reveal_item_in_dir(p)
            .map_err(|e| e.to_string())?;
        return Ok("revealed".into());
    }
    app.opener()
        .open_path(p.display().to_string(), None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok("default".into())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileView {
    pub path: String,
    pub text: String,
    /// Highlighted HTML (classed spans), when the language is known.
    pub html: Option<String>,
    pub lines: usize,
    pub size: u64,
    pub truncated: bool,
    pub binary: bool,
}

const VIEW_LIMIT: u64 = 2 << 20;

/// A file's text for the read-only viewer tab.
#[tauri::command]
pub async fn read_text_file(path: String) -> Result<FileView, String> {
    tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let p = PathBuf::from(&path);
        let meta = std::fs::metadata(&p).map_err(|_| "That file does not exist".to_string())?;
        if meta.is_dir() {
            return Err("That is a folder".into());
        }
        let mut buf = Vec::new();
        std::fs::File::open(&p)
            .and_then(|f| f.take(VIEW_LIMIT).read_to_end(&mut buf))
            .map_err(|e| e.to_string())?;
        let binary = buf.iter().take(8000).any(|&b| b == 0);
        let text = if binary {
            String::new()
        } else {
            String::from_utf8_lossy(&buf).into_owned()
        };
        let ext = p
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .or_else(|| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_default();
        let html = if binary {
            None
        } else {
            glaude_core::render::highlight(&text, &ext)
        };
        Ok(FileView {
            path,
            lines: text.lines().count(),
            text,
            html,
            size: meta.len(),
            truncated: meta.len() > VIEW_LIMIT,
            binary,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_paths_with_positions() {
        let root = std::env::temp_dir().join(format!("glaude-paths-{}", std::process::id()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/a b.ts"), "x").unwrap();
        std::fs::write(root.join("main.rs"), "x").unwrap();
        let home = root.clone();
        let hit = |s: &str| resolve(&root, s, &home);

        assert_eq!(hit("main.rs:12").and_then(|h| h.line), Some(12));
        let h = hit("main.rs:12:4").unwrap();
        assert_eq!((h.line, h.col, h.dir), (Some(12), Some(4), false));
        assert_eq!(hit("main.rs#L7").and_then(|h| h.line), Some(7));
        assert!(hit("src").unwrap().dir);
        assert!(hit("~/src/a b.ts").is_some());
        let url = format!("file://{}/src/a%20b.ts", root.display());
        assert!(hit(&url).is_some());
        assert!(hit("main.rs.").is_some(), "trailing period from a sentence");
        assert!(hit("missing.rs").is_none());
        assert!(hit("https://example.com/main.rs").is_none());
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn programs_are_not_opened() {
        assert!(runs_as_program(Path::new("/x/Tool.app")));
        assert!(runs_as_program(Path::new("/x/run.command")));
        assert!(!runs_as_program(Path::new("/x/notes.md")));
    }
}

use glaude_accounts::paths;
use glaude_core::env;
use glaude_core::model::Attachment;
use serde::Serialize;
use tauri::State;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub path: Option<String>,
    pub version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub os: String,
    pub home: String,
    pub claude: Tool,
    pub codex: Tool,
}

fn probe(bin: Option<std::path::PathBuf>) -> Tool {
    let version = bin.as_deref().and_then(env::binary_version);
    Tool {
        path: bin.map(|p| p.display().to_string()),
        version,
    }
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub async fn app_info() -> Result<AppInfo, String> {
    tokio::task::spawn_blocking(|| AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        home: env::home_dir().display().to_string(),
        claude: probe(env::claude_binary()),
        codex: probe(env::codex_binary()),
    })
    .await
    .map_err(err)
}

#[tauri::command]
pub fn default_cwd() -> String {
    let home = env::home_dir();
    let projects = home.join("Projects");
    if projects.is_dir() { projects } else { home }
        .display()
        .to_string()
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle, start: Option<String>) -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(move || {
        let mut d = app.dialog().file().set_title("Choose a project folder");
        if let Some(s) = start.filter(|s| std::path::Path::new(s).is_dir()) {
            d = d.set_directory(s);
        }
        d.blocking_pick_folder().map(|p| p.to_string())
    })
    .await
    .map_err(err)
}

fn mime_for(path: &std::path::Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "md" | "txt" | "rs" | "ts" | "js" | "py" | "json" | "toml" | "yaml" | "yml" => "text/plain",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[tauri::command]
pub async fn pick_files(app: AppHandle) -> Result<Vec<Attachment>, String> {
    tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Attach files")
            .blocking_pick_files()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|f| f.into_path().ok())
            .map(|p| Attachment {
                name: p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                mime: mime_for(&p),
                path: p.display().to_string(),
            })
            .collect()
    })
    .await
    .map_err(err)
}

/// Save pasted/dropped bytes into the app cache so agents can read them.
#[tauri::command]
pub async fn save_attachment(
    app: AppHandle,
    name: String,
    mime: String,
    data: String,
) -> Result<Attachment, String> {
    use base64::Engine;
    let dir = app.path().app_cache_dir().map_err(err)?.join("attachments");
    std::fs::create_dir_all(&dir).map_err(err)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(err)?;
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let path = dir.join(format!(
        "{}-{}",
        uuid::Uuid::new_v4().simple(),
        if safe.is_empty() { "file".into() } else { safe }
    ));
    std::fs::write(&path, bytes).map_err(err)?;
    Ok(Attachment {
        name,
        mime,
        path: path.display().to_string(),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountBrief {
    pub provider: &'static str,
    pub name: String,
    pub email: Option<String>,
    pub active: bool,
    pub archived: bool,
}

/// Minimal account list for the new-conversation dialog. The full
/// mamadoo view lives in the accounts module.
#[tauri::command]
pub fn accounts_brief() -> Vec<AccountBrief> {
    let mut out = vec![AccountBrief {
        provider: "claude",
        name: "default".into(),
        email: None,
        active: false,
        archived: false,
    }];
    if let Ok(rd) = std::fs::read_dir(paths::root()) {
        let mut names: Vec<String> = rd
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().to_str().map(str::to_string))
            .filter(|n| !n.starts_with('.'))
            .collect();
        names.sort_by_key(|n| n.to_lowercase());
        out.extend(names.into_iter().map(|name| AccountBrief {
            provider: "claude",
            name,
            email: None,
            active: false,
            archived: false,
        }));
    }
    out.push(AccountBrief {
        provider: "codex",
        name: "default".into(),
        email: None,
        active: false,
        archived: false,
    });
    if let Ok(rd) = std::fs::read_dir(env::home_dir()) {
        let mut names: Vec<String> = rd
            .flatten()
            .filter_map(|e| {
                e.file_name()
                    .to_str()
                    .and_then(|n| n.strip_prefix(".codex_"))
                    .map(str::to_string)
            })
            .filter(|n| !n.is_empty())
            .collect();
        names.sort_by_key(|n| n.to_lowercase());
        out.extend(names.into_iter().map(|name| AccountBrief {
            provider: "codex",
            name,
            email: None,
            active: false,
            archived: false,
        }));
    }
    out
}

/// Past Claude sessions, newest first.
#[tauri::command]
pub async fn sessions_list() -> Result<Vec<glaude_claude::transcript::SessionSummary>, String> {
    tokio::task::spawn_blocking(|| {
        glaude_claude::transcript::list_sessions(&paths::claude_home("default"), 400)
    })
    .await
    .map_err(err)
}

/// Past Codex threads. Separate from `sessions_list` because the first call
/// starts the app-server, which takes several seconds.
#[tauri::command]
pub async fn codex_sessions_list(
    pool: State<'_, crate::conv::CodexPool>,
) -> Result<Vec<glaude_claude::transcript::SessionSummary>, String> {
    let server = pool
        .0
        .get(&paths::codex_home("default"))
        .await
        .map_err(err)?;
    let threads = server.list_threads(300).await.map_err(err)?;
    Ok(threads
        .into_iter()
        .map(|t| glaude_claude::transcript::SessionSummary {
            provider: "codex",
            id: t.id,
            cwd: t.cwd,
            title: t.title,
            updated: t.updated,
            path: t.path,
        })
        .collect())
}

/// Codex models for the new-conversation dialog.
#[tauri::command]
pub async fn codex_models(
    pool: State<'_, crate::conv::CodexPool>,
    account: Option<String>,
) -> Result<Vec<glaude_core::model::ModelChoice>, String> {
    let home = paths::codex_home(account.as_deref().unwrap_or("default"));
    let server = pool.0.get(&home).await.map_err(err)?;
    server.models().await.map_err(err)
}

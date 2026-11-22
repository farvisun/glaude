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
        .get(
            &paths::codex_home("default"),
            crate::proxy::for_account("codex", "default").as_ref(),
        )
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
            created: t.created,
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
    let account = account.as_deref().unwrap_or("default");
    let home = paths::codex_home(account);
    let proxy = crate::proxy::for_account("codex", account);
    let server = pool.0.get(&home, proxy.as_ref()).await.map_err(err)?;
    server.models().await.map_err(err)
}

/// Agents for the new-conversation picker before any session runs:
/// built-ins plus `agents/*.md` of the account and of the project.
/// (Sessions report the full list, plugins included, once started.)
#[tauri::command]
pub async fn claude_agents(
    account: String,
    cwd: Option<String>,
) -> Vec<glaude_core::model::AgentInfo> {
    use glaude_core::model::AgentInfo;
    let home = glaude_accounts::paths::claude_config_dir(&account)
        .unwrap_or_else(|| glaude_core::env::home_dir().join(".claude"));
    tokio::task::spawn_blocking(move || {
        let builtin = |name: &str, description: &str| AgentInfo {
            name: name.into(),
            description: description.into(),
            model: None,
        };
        let mut out = vec![
            builtin(
                "general-purpose",
                "General-purpose agent for research and multi-step tasks.",
            ),
            builtin("Explore", "Fast read-only search across the codebase."),
            builtin(
                "Plan",
                "Designs an implementation plan without editing files.",
            ),
        ];
        let mut dirs = vec![home.join("agents")];
        if let Some(c) = cwd.filter(|c| !c.is_empty()) {
            dirs.push(std::path::Path::new(&c).join(".claude").join("agents"));
        }
        for dir in dirs {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for e in entries.flatten() {
                let path = e.path();
                if path.extension().is_none_or(|x| x != "md") {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let Some(front) = frontmatter(&text) else {
                    continue;
                };
                let get = |k: &str| {
                    front.lines().find_map(|l| {
                        let (key, val) = l.split_once(':')?;
                        (key.trim() == k).then(|| val.trim().trim_matches(['"', '\'']).to_string())
                    })
                };
                let Some(name) = get("name").filter(|n| !n.is_empty()) else {
                    continue;
                };
                let mut description = get("description").unwrap_or_default();
                if description.chars().count() > 300 {
                    description = description.chars().take(300).collect::<String>() + "…";
                }
                out.retain(|a| a.name != name);
                out.push(AgentInfo {
                    name,
                    description,
                    model: get("model").filter(|m| !m.is_empty() && m != "inherit"),
                });
            }
        }
        out
    })
    .await
    .unwrap_or_default()
}

fn frontmatter(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

/// Open a memory file (CLAUDE.md, rules, notes) or memory folder in the
/// system's default app. Only Markdown files and folders inside a Claude
/// config folder are opened.
#[tauri::command]
pub fn open_memory_path(app: tauri::AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let p = std::path::PathBuf::from(&path);
    let meta = std::fs::metadata(&p).map_err(|_| "That file does not exist yet".to_string())?;
    let in_claude_dir = p
        .components()
        .any(|c| c.as_os_str().to_string_lossy().starts_with(".claude"));
    let ok = if meta.is_dir() {
        in_claude_dir
    } else {
        p.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"))
    };
    if !ok {
        return Err("Only memory files and folders can be opened".into());
    }
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

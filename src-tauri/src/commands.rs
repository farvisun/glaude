use glaude_core::env;
use serde::Serialize;

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

#[tauri::command]
pub async fn app_info() -> AppInfo {
    tokio::task::spawn_blocking(|| AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        home: env::home_dir().display().to_string(),
        claude: probe(env::claude_binary()),
        codex: probe(env::codex_binary()),
    })
    .await
    .expect("app_info task")
}

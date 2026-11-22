mod commands;

use tracing_subscriber::EnvFilter;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("glaude=info,glaude_lib=info,glaude_core=info,glaude_claude=info,glaude_codex=info,glaude_accounts=info")),
        )
        .init();

    // Read the login shell environment off the main thread so the window
    // shows at once; the first agent spawn waits on the same OnceLock.
    std::thread::spawn(|| {
        glaude_core::env::login_env();
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::app_info])
        .run(tauri::generate_context!())
        .expect("error while running Glaude");
}

mod accounts;
mod commands;
mod conv;
mod pty;
mod search;
mod sink;
mod statusline;

use tauri::Manager;
use tracing_subscriber::EnvFilter;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("glaude=info,glaude_lib=info,glaude_core=info,glaude_claude=info,glaude_codex=info,glaude_accounts=info,glaude_search=info")),
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
        .manage(conv::Convs::default())
        .manage(conv::CodexPool::default())
        .manage(pty::Ptys::default())
        .manage(search::SearchState::default())
        .setup(|app| {
            // Bring the history index up to date in the background.
            let state = app.state::<search::SearchState>().inner().clone();
            state.0.refresh(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::default_cwd,
            commands::pick_folder,
            commands::pick_files,
            commands::save_attachment,
            accounts::accounts_brief,
            accounts::accounts_list,
            accounts::accounts_refresh,
            accounts::accounts_create,
            accounts::accounts_rename,
            accounts::accounts_delete,
            accounts::accounts_archive,
            accounts::accounts_set_active,
            accounts::accounts_share,
            accounts::accounts_pref,
            commands::sessions_list,
            commands::codex_sessions_list,
            commands::codex_models,
            statusline::statusline_render,
            search::search_status,
            search::search_refresh,
            search::search_query,
            pty::pty_open,
            pty::pty_write,
            pty::pty_resize,
            pty::pty_close,
            conv::conv_start,
            conv::conv_send,
            conv::conv_interrupt,
            conv::conv_permission,
            conv::conv_set_mode,
            conv::conv_set_model,
            conv::conv_close,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Glaude")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                conv::close_all(&app.state::<conv::Convs>(), &app.state::<conv::CodexPool>());
                pty::close_all(&app.state::<pty::Ptys>());
            }
        });
}

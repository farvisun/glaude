mod accounts;
mod commands;
mod conv;
pub mod host;
mod pty;
mod search;
mod sink;
mod statusline;

use tauri::Manager;
use tracing_subscriber::EnvFilter;

/// SIGTERM/SIGINT/SIGHUP go through the normal exit path, so agent
/// processes and PTYs are stopped instead of left running.
fn exit_on_signals(app: tauri::AppHandle) {
    #[cfg(unix)]
    tauri::async_runtime::spawn(async move {
        use tokio::signal::unix::{signal, SignalKind};
        let (Ok(mut term), Ok(mut int), Ok(mut hup)) = (
            signal(SignalKind::terminate()),
            signal(SignalKind::interrupt()),
            signal(SignalKind::hangup()),
        ) else {
            return;
        };
        tokio::select! {
            _ = term.recv() => {}
            _ = int.recv() => {}
            _ = hup.recv() => {}
        }
        tracing::info!("signal received, shutting down");
        app.exit(0);
    });
    #[cfg(not(unix))]
    let _ = app;
}

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
        .manage(host::client::HostState::default())
        .manage(conv::CodexPool::default())
        .manage(pty::Ptys::default())
        .manage(search::SearchState::default())
        .setup(|app| {
            // Bring the history index up to date in the background.
            let state = app.state::<search::SearchState>().inner().clone();
            state.0.refresh(app.handle().clone());
            exit_on_signals(app.handle().clone());
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
            conv::conv_attach,
            conv::host_list,
            conv::conv_queue,
            conv::conv_schedule,
            conv::conv_send,
            conv::conv_interrupt,
            conv::conv_permission,
            conv::conv_set_mode,
            conv::conv_set_model,
            conv::conv_close,
            conv::conv_control,
            conv::conv_answer_question,
            conv::conv_hook_runs,
            conv::claude_rewind_target,
            conv::task_output_file,
            commands::claude_agents,
            commands::open_memory_path,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Glaude")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                conv::close_all(&app.state::<conv::CodexPool>());
                pty::close_all(&app.state::<pty::Ptys>());
            }
        });
}

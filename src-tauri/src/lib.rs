mod accounts;
mod commands;
mod conv;
mod files;
mod git;
pub mod host;
mod menu;
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
        .manage(menu::AppPrefs::default())
        .manage(conv::CodexPool::default())
        .manage(pty::Ptys::default())
        .manage(search::SearchState::default())
        .setup(|app| {
            // Bring the history index up to date in the background.
            let state = app.state::<search::SearchState>().inner().clone();
            state.0.refresh(app.handle().clone());
            exit_on_signals(app.handle().clone());
            #[cfg(target_os = "macos")]
            app.set_menu(menu::app_menu(app.handle())?)?;
            app.on_menu_event(|app, event| menu::on_menu(app, event.id().as_ref()));
            if let Err(e) = menu::create_tray(app.handle()) {
                tracing::warn!("no menu bar icon: {e}");
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // macOS: closing the window keeps Glaude in the menu bar and the
            // Dock; Cmd+Q quits.
            #[cfg(target_os = "macos")]
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (window, event);
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
            git::git_info,
            git::git_diff,
            git::git_log,
            git::git_commit,
            git::git_push,
            git::git_pr_status,
            git::git_pr_create,
            git::git_message,
            git::project_config,
            git::worktree_add,
            git::worktree_remove,
            files::paths_check,
            files::open_path,
            files::editors_available,
            files::read_text_file,
            menu::app_prefs_set,
            menu::tray_update,
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
        .run(|app, event| match event {
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => menu::show_main(app),
            tauri::RunEvent::Exit => {
                // Settings → Background: quitting can also stop the agents.
                let keep = app
                    .state::<menu::AppPrefs>()
                    .keep_agents
                    .load(std::sync::atomic::Ordering::Relaxed);
                if !keep {
                    let host = app.state::<host::client::HostState>();
                    tauri::async_runtime::block_on(async {
                        if let Some(c) = host.current().await {
                            let _ = tokio::time::timeout(
                                std::time::Duration::from_secs(3),
                                c.call("close_all", serde_json::json!({})),
                            )
                            .await;
                        }
                    });
                }
                conv::close_all(&app.state::<conv::CodexPool>());
                pty::close_all(&app.state::<pty::Ptys>());
            }
            _ => {}
        });
}

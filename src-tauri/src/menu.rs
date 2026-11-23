//! The macOS menu bar (app menus), the status icon in the menu bar with the
//! agents that are working or waiting, and app settings the UI pushes here
//! (quit behaviour, notifications while closed).

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Deserialize;
use serde_json::json;
use tauri::image::Image;
#[cfg(target_os = "macos")]
use tauri::menu::{AboutMetadataBuilder, SubmenuBuilder};
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::host::client::HostState;

const TRAY_ID: &str = "glaude";

pub struct AppPrefs {
    /// Quitting leaves running agents to finish in the host.
    pub keep_agents: AtomicBool,
    /// The host may notify while no window is open.
    pub notify_closed: AtomicBool,
}

impl Default for AppPrefs {
    fn default() -> Self {
        Self {
            keep_agents: AtomicBool::new(true),
            notify_closed: AtomicBool::new(true),
        }
    }
}

#[tauri::command]
pub async fn app_prefs_set(
    prefs: State<'_, AppPrefs>,
    host: State<'_, HostState>,
    keep_agents: bool,
    notify_closed: bool,
) -> Result<(), String> {
    prefs.keep_agents.store(keep_agents, Ordering::Relaxed);
    prefs.notify_closed.store(notify_closed, Ordering::Relaxed);
    // Only an already running host; it gets the settings when it connects.
    if let Some(c) = host.current().await {
        let _ = c.call("config", json!({ "notify": notify_closed })).await;
    }
    Ok(())
}

/// Show and focus the main window (after it was closed to the menu bar).
pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Menu items run in the UI (it owns tabs and conversations); a few are
/// handled here.
pub fn on_menu<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "close-window" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.close();
            }
        }
        "show" => show_main(app),
        "quit" => app.exit(0),
        "host-log" => {
            use tauri_plugin_opener::OpenerExt;
            let _ = app.opener().reveal_item_in_dir(crate::host::log_path());
        }
        _ => {
            // From the status icon the window may be hidden.
            if id.starts_with("open-conv:") || id == "new-conversation" || id == "agents" {
                show_main(app);
            }
            let _ = app.emit("menu", id);
        }
    }
}

#[cfg(target_os = "macos")]
pub fn app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let item = |id: &str, text: &str, accel: Option<&str>| {
        let mut b = MenuItemBuilder::with_id(id, text);
        if let Some(a) = accel {
            b = b.accelerator(a);
        }
        b.build(app)
    };
    let about = AboutMetadataBuilder::new()
        .name(Some("Glaude"))
        .version(Some(env!("CARGO_PKG_VERSION")))
        .comments(Some("A desktop app for Claude Code and Codex."))
        .build();

    let glaude = SubmenuBuilder::new(app, "Glaude")
        .about(Some(about))
        .separator()
        .item(&item("settings", "Settings…", Some("CmdOrCtrl+,"))?)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;
    let file = SubmenuBuilder::new(app, "File")
        .item(&item(
            "new-conversation",
            "New Conversation…",
            Some("CmdOrCtrl+N"),
        )?)
        .item(&item("new-terminal", "New Terminal", Some("CmdOrCtrl+T"))?)
        .item(&item("open-folder", "Open Folder…", Some("CmdOrCtrl+O"))?)
        .separator()
        .item(&item("close-tab", "Close Tab", Some("CmdOrCtrl+W"))?)
        .item(&item(
            "close-window",
            "Close Window",
            Some("CmdOrCtrl+Shift+W"),
        )?)
        .build()?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .separator()
        .item(&item("find", "Find in Conversation…", Some("CmdOrCtrl+F"))?)
        .item(&item("search", "Search History…", Some("CmdOrCtrl+K"))?)
        .build()?;
    let theme = SubmenuBuilder::new(app, "Appearance")
        .item(&item("theme-system", "System", None)?)
        .item(&item("theme-light", "Light", None)?)
        .item(&item("theme-dark", "Dark", None)?)
        .build()?;
    let view = SubmenuBuilder::new(app, "View")
        .item(&item(
            "palette",
            "Command Palette…",
            Some("CmdOrCtrl+Shift+P"),
        )?)
        .separator()
        .item(&item(
            "toggle-sidebar",
            "Toggle Sidebar",
            Some("CmdOrCtrl+B"),
        )?)
        .item(&item(
            "toggle-progress",
            "Toggle Progress Panel",
            Some("Alt+CmdOrCtrl+P"),
        )?)
        .item(&item(
            "toggle-terminal",
            "Toggle Terminal Panel",
            Some("Ctrl+Backquote"),
        )?)
        .item(&item("maximize-terminal", "Maximize Terminal Panel", None)?)
        .separator()
        .item(&item("zoom-in", "Zoom In", Some("CmdOrCtrl+="))?)
        .item(&item("zoom-out", "Zoom Out", Some("CmdOrCtrl+-"))?)
        .item(&item("zoom-reset", "Actual Size", Some("CmdOrCtrl+0"))?)
        .separator()
        .item(&theme)
        .separator()
        .fullscreen()
        .build()?;
    let conversation = SubmenuBuilder::new(app, "Conversation")
        .item(&item("changes", "Changes…", Some("CmdOrCtrl+Shift+G"))?)
        .item(&item(
            "review",
            "Ask the Other Tool to Review Changes",
            Some("CmdOrCtrl+Shift+R"),
        )?)
        .item(&item("side-chat", "Side Chat", None)?)
        .item(&item(
            "session-details",
            "Session Details",
            Some("CmdOrCtrl+I"),
        )?)
        .separator()
        .item(&item("stop", "Stop Agent", Some("CmdOrCtrl+."))?)
        .separator()
        .item(&item("split-right", "Split Right", Some("CmdOrCtrl+\\"))?)
        .item(&item(
            "split-down",
            "Split Down",
            Some("CmdOrCtrl+Shift+\\"),
        )?)
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .item(&item("agents", "Agents Board", Some("Alt+CmdOrCtrl+A"))?)
        .item(&item(
            "next-waiting",
            "Next Agent Waiting for You",
            Some("CmdOrCtrl+Shift+A"),
        )?)
        .item(&item("usage", "Usage", None)?)
        .separator()
        .item(&item("next-tab", "Show Next Tab", Some("Ctrl+Tab"))?)
        .item(&item(
            "prev-tab",
            "Show Previous Tab",
            Some("Ctrl+Shift+Tab"),
        )?)
        .separator()
        .bring_all_to_front()
        .build()?;
    let help = SubmenuBuilder::new(app, "Help")
        .item(&item("shortcuts", "Keyboard Shortcuts", None)?)
        .item(&item("host-log", "Show Agent Host Log", None)?)
        .build()?;
    MenuBuilder::new(app)
        .items(&[&glaude, &file, &edit, &view, &conversation, &window, &help])
        .build()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayConv {
    pub id: String,
    pub title: String,
    /// working | waiting | unread | idle
    pub state: String,
    /// What it is doing now ("Editing App.svelte", "Allow Bash?").
    #[serde(default)]
    pub step: Option<String>,
}

fn tray_menu<R: Runtime>(app: &AppHandle<R>, convs: &[TrayConv]) -> tauri::Result<Menu<R>> {
    let working = convs.iter().filter(|c| c.state == "working").count();
    let waiting = convs.iter().filter(|c| c.state == "waiting").count();
    let summary = match (working, waiting) {
        (0, 0) => "No agents working".to_string(),
        (w, 0) => format!("{w} working"),
        (0, n) => format!("{n} need{} you", if n == 1 { "s" } else { "" }),
        (w, n) => format!(
            "{w} working · {n} need{} you",
            if n == 1 { "s" } else { "" }
        ),
    };
    let mut b = MenuBuilder::new(app).item(
        &MenuItemBuilder::with_id("summary", summary)
            .enabled(false)
            .build(app)?,
    );
    let mut listed: Vec<&TrayConv> = convs.iter().collect();
    // Waiting first, then working, then finished.
    listed.sort_by_key(|c| match c.state.as_str() {
        "waiting" => 0,
        "working" => 1,
        "unread" => 2,
        _ => 3,
    });
    if !listed.is_empty() {
        b = b.separator();
    }
    for c in listed.into_iter().take(14) {
        let mark = match c.state.as_str() {
            "waiting" => "● ",
            "working" => "◐ ",
            "unread" => "✓ ",
            _ => "    ",
        };
        let mut title: String = c.title.chars().take(40).collect();
        if c.title.chars().count() > 40 {
            title.push('…');
        }
        if let Some(step) = c.step.as_deref().filter(|s| !s.is_empty()) {
            title = format!("{title}  —  {step}");
        }
        b = b.item(
            &MenuItemBuilder::with_id(format!("open-conv:{}", c.id), format!("{mark}{title}"))
                .build(app)?,
        );
    }
    b.separator()
        .item(&MenuItemBuilder::with_id("new-conversation", "New Conversation…").build(app)?)
        .item(&MenuItemBuilder::with_id("agents", "Agents Board").build(app)?)
        .item(&MenuItemBuilder::with_id("show", "Show Glaude").build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("quit", "Quit Glaude").build(app)?)
        .build()
}

pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Glaude")
        .menu(&tray_menu(app, &[])?)
        .show_menu_on_left_click(true)
        .build(app)?;
    Ok(())
}

/// The UI's conversations for the status icon menu, and its title (how many
/// are working, or how many need you).
#[tauri::command]
pub fn tray_update(app: AppHandle, convs: Vec<TrayConv>) -> Result<(), String> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    let menu = tray_menu(&app, &convs).map_err(|e| e.to_string())?;
    tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    let working = convs.iter().filter(|c| c.state == "working").count();
    let waiting = convs.iter().filter(|c| c.state == "waiting").count();
    let title = if waiting > 0 {
        Some(format!("{waiting} waiting"))
    } else if working > 0 {
        Some(working.to_string())
    } else {
        None
    };
    tray.set_title(title).map_err(|e| e.to_string())?;
    Ok(())
}

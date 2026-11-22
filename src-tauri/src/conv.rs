//! Conversation commands. The agent processes live in the host process
//! (`host`), so they keep working while the window is closed; these
//! commands forward to it and route its events to the webview.

use glaude_accounts::paths;
use serde_json::{json, Value};
use tauri::{AppHandle, State};

use crate::host::client::{EventChannel, HostState};

/// One `codex app-server` per account home for the app's own requests
/// (models, account limits); conversations use the host's.
#[derive(Default)]
pub struct CodexPool(pub glaude_codex::Pool);

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

async fn call(
    app: &AppHandle,
    host: &HostState,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    host.get(app).await?.call(method, params).await
}

#[tauri::command]
pub async fn conv_start(
    app: AppHandle,
    host: State<'_, HostState>,
    conv_id: String,
    spec: Value,
    on_event: EventChannel,
) -> Result<(), String> {
    let client = host.get(&app).await?;
    client.set_channel(&conv_id, on_event);
    let res = client
        .call("start", json!({"convId": conv_id, "spec": spec}))
        .await;
    if res.is_err() {
        client.drop_channel(&conv_id);
    }
    res.map(|_| ())
}

/// Reattach to a conversation the host is still running (after the window
/// was closed or reloaded): its whole state arrives first, then the stream.
#[tauri::command]
pub async fn conv_attach(
    app: AppHandle,
    host: State<'_, HostState>,
    conv_id: String,
    on_event: EventChannel,
) -> Result<Value, String> {
    let client = host.get(&app).await?;
    client.set_channel(&conv_id, on_event);
    let res = client.call("attach", json!({"convId": conv_id})).await;
    if res.is_err() {
        client.drop_channel(&conv_id);
    }
    res
}

/// Conversations the host is running.
#[tauri::command]
pub async fn host_list(app: AppHandle, host: State<'_, HostState>) -> Result<Value, String> {
    call(&app, &host, "list", json!({})).await
}

#[tauri::command]
pub async fn conv_send(
    app: AppHandle,
    host: State<'_, HostState>,
    conv_id: String,
    text: String,
    attachments: Value,
    steer: Option<bool>,
) -> Result<(), String> {
    let method = if steer.unwrap_or(false) {
        "steer"
    } else {
        "send"
    };
    call(
        &app,
        &host,
        method,
        json!({"convId": conv_id, "text": text, "attachments": attachments}),
    )
    .await
    .map(|_| ())
}

/// Replace the conversation's queue (messages sent when the turn ends).
#[tauri::command]
pub async fn conv_queue(
    app: AppHandle,
    host: State<'_, HostState>,
    conv_id: String,
    items: Value,
) -> Result<(), String> {
    call(
        &app,
        &host,
        "queue",
        json!({"convId": conv_id, "items": items}),
    )
    .await
    .map(|_| ())
}

/// Continue automatically at `resume_at` (RFC 3339), or cancel with null.
#[tauri::command]
pub async fn conv_schedule(
    app: AppHandle,
    host: State<'_, HostState>,
    conv_id: String,
    resume_at: Option<String>,
) -> Result<(), String> {
    call(
        &app,
        &host,
        "schedule",
        json!({"convId": conv_id, "resumeAt": resume_at}),
    )
    .await
    .map(|_| ())
}

#[tauri::command]
pub async fn conv_interrupt(
    app: AppHandle,
    host: State<'_, HostState>,
    conv_id: String,
) -> Result<(), String> {
    call(&app, &host, "interrupt", json!({"convId": conv_id}))
        .await
        .map(|_| ())
}

#[tauri::command]
pub async fn conv_permission(
    app: AppHandle,
    host: State<'_, HostState>,
    conv_id: String,
    request_id: String,
    option_id: String,
    message: Option<String>,
) -> Result<(), String> {
    call(
        &app,
        &host,
        "permission",
        json!({"convId": conv_id, "requestId": request_id, "optionId": option_id, "message": message}),
    )
    .await
    .map(|_| ())
}

#[tauri::command]
pub async fn conv_set_mode(
    app: AppHandle,
    host: State<'_, HostState>,
    conv_id: String,
    mode: String,
) -> Result<(), String> {
    call(
        &app,
        &host,
        "set_mode",
        json!({"convId": conv_id, "value": mode}),
    )
    .await
    .map(|_| ())
}

#[tauri::command]
pub async fn conv_set_model(
    app: AppHandle,
    host: State<'_, HostState>,
    conv_id: String,
    model: String,
) -> Result<(), String> {
    call(
        &app,
        &host,
        "set_model",
        json!({"convId": conv_id, "value": model}),
    )
    .await
    .map(|_| ())
}

#[tauri::command]
pub async fn conv_close(
    app: AppHandle,
    host: State<'_, HostState>,
    conv_id: String,
) -> Result<(), String> {
    let client = host.get(&app).await?;
    client.drop_channel(&conv_id);
    client
        .call("close", json!({"convId": conv_id}))
        .await
        .map(|_| ())
}

#[tauri::command]
pub async fn conv_control(
    app: AppHandle,
    host: State<'_, HostState>,
    conv_id: String,
    request: Value,
) -> Result<Value, String> {
    call(
        &app,
        &host,
        "control",
        json!({"convId": conv_id, "request": request}),
    )
    .await
}

#[tauri::command]
pub async fn conv_answer_question(
    app: AppHandle,
    host: State<'_, HostState>,
    conv_id: String,
    request_id: String,
    answers: Value,
) -> Result<(), String> {
    call(
        &app,
        &host,
        "answer_question",
        json!({"convId": conv_id, "requestId": request_id, "answers": answers}),
    )
    .await
    .map(|_| ())
}

#[tauri::command]
pub async fn conv_hook_runs(
    app: AppHandle,
    host: State<'_, HostState>,
    conv_id: String,
) -> Result<Value, String> {
    call(&app, &host, "hook_runs", json!({"convId": conv_id})).await
}

/// Stop the app's own Codex servers when it quits. Conversations keep
/// running in the host.
pub fn close_all(pool: &CodexPool) {
    pool.0.shutdown_blocking();
    glaude_core::procs::kill_all(std::time::Duration::from_millis(800));
}

/// Where to resume to rewind a Claude conversation to just before one of
/// its user messages: the uuid of the message before it, or `None` when it
/// was the first message (start fresh).
#[tauri::command]
pub async fn claude_rewind_target(
    account: String,
    cwd: String,
    session_id: String,
    user_uuid: String,
) -> Result<Option<String>, String> {
    let home = paths::claude_config_dir(&account)
        .unwrap_or_else(|| glaude_core::env::home_dir().join(".claude"));
    tokio::task::spawn_blocking(move || {
        let path = glaude_claude::transcript::find_transcript(&home, &cwd, &session_id)
            .ok_or("The transcript of this conversation was not found")?;
        glaude_claude::transcript::parent_of(&path, &user_uuid)
            .ok_or_else(|| "That message is not in the transcript yet".to_string())
    })
    .await
    .map_err(err)?
}

/// The full output of a finished background task. Only `.output` files in
/// Claude's task folders under the temp directory are read.
#[tauri::command]
pub async fn task_output_file(path: String) -> Result<String, String> {
    let p = std::path::PathBuf::from(&path);
    let tmp = std::env::temp_dir();
    let allowed = p.extension().is_some_and(|e| e == "output")
        && p.components().any(|c| c.as_os_str() == "tasks")
        && (p.starts_with(&tmp) || p.starts_with("/tmp") || p.starts_with("/private/tmp"));
    if !allowed {
        return Err("Not a task output file".into());
    }
    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&p).map_err(err)?;
        let tail = bytes.len().saturating_sub(512 * 1024);
        Ok(String::from_utf8_lossy(&bytes[tail..]).into_owned())
    })
    .await
    .map_err(err)?
}

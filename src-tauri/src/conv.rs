//! Conversation commands: start, send, interrupt, permissions, close.

use std::collections::HashMap;
use std::sync::Arc;

use base64::Engine;
use glaude_accounts::paths;
use glaude_claude::{ClaudeSession, ClaudeSpec};
use glaude_codex::{CodexSpec, CodexThread};
use glaude_core::model::{Attachment, ConvEvent};
use parking_lot::Mutex;
use serde::Deserialize;
use tauri::ipc::Channel;
use tauri::State;

use crate::sink::EventSink;

pub enum Agent {
    Claude(ClaudeSession),
    Codex(CodexThread),
}

/// One `codex app-server` per account home, shared by its threads.
#[derive(Default)]
pub struct CodexPool(pub glaude_codex::Pool);

pub struct Conv {
    pub agent: Agent,
    pub sink: Arc<EventSink>,
}

#[derive(Default)]
pub struct Convs(pub Mutex<HashMap<String, Arc<Conv>>>);

impl Convs {
    fn get(&self, id: &str) -> Result<Arc<Conv>, String> {
        self.0.lock().get(id).cloned().ok_or_else(|| {
            "This conversation is not running. Reopen it from the sidebar.".to_string()
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSpec {
    pub provider: String,
    pub account: String,
    pub cwd: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub permission_mode: Option<String>,
    pub resume: Option<String>,
    #[serde(default)]
    pub fork: bool,
    /// Restarting an ended session: the UI already shows its history.
    #[serde(default)]
    pub skip_history: bool,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub async fn conv_start(
    convs: State<'_, Convs>,
    pool: State<'_, CodexPool>,
    conv_id: String,
    spec: StartSpec,
    on_event: Channel<Vec<ConvEvent>>,
) -> Result<(), String> {
    let sink = EventSink::new(on_event);
    let push = {
        let sink = sink.clone();
        Arc::new(move |events: Vec<ConvEvent>| sink.push(events)) as glaude_claude::Sink
    };
    // mamadoo's launch bookkeeping; a restart of an ended session is not a new launch.
    let shared =
        crate::accounts::prepare_launch(&spec.provider, &spec.account, !spec.skip_history).await;
    if !shared.is_empty() {
        tracing::info!("shared home for {}: {}", spec.account, shared.join("; "));
        sink.push(vec![ConvEvent::Notice {
            level: "info".into(),
            text: format!("Shared home: {}", shared.join("; ")),
        }]);
    }
    let agent = match spec.provider.as_str() {
        "claude" => {
            let cs = ClaudeSpec {
                cwd: spec.cwd.clone(),
                config_dir: paths::claude_config_dir(&spec.account),
                model: spec.model.clone(),
                effort: spec.effort.clone(),
                permission_mode: spec.permission_mode.clone(),
                resume: spec.resume.clone(),
                fork: spec.fork,
                load_history: !spec.skip_history,
            };
            Agent::Claude(ClaudeSession::spawn(cs, push).await.map_err(err)?)
        }
        "codex" => {
            let server = pool
                .0
                .get(&paths::codex_home(&spec.account))
                .await
                .map_err(err)?;
            let cs = CodexSpec {
                cwd: spec.cwd.clone(),
                model: spec.model.clone(),
                effort: spec.effort.clone(),
                mode: spec.permission_mode.clone(),
                resume: spec.resume.clone(),
                load_history: !spec.skip_history,
            };
            Agent::Codex(CodexThread::open(server, cs, push).await.map_err(err)?)
        }
        other => return Err(format!("{other} conversations are not supported yet")),
    };
    convs
        .0
        .lock()
        .insert(conv_id, Arc::new(Conv { agent, sink }));
    Ok(())
}

/// Images go to Claude inline (base64); other files are referenced by path.
fn split_attachments(text: &str, attachments: &[Attachment]) -> (String, Vec<(String, String)>) {
    let mut images = Vec::new();
    let mut paths = Vec::new();
    for a in attachments {
        if a.mime.starts_with("image/") {
            match std::fs::read(&a.path) {
                Ok(bytes) => images.push((
                    a.mime.clone(),
                    base64::engine::general_purpose::STANDARD.encode(bytes),
                )),
                Err(_) => paths.push(a.path.clone()),
            }
        } else {
            paths.push(a.path.clone());
        }
    }
    let mut full = text.to_string();
    if !paths.is_empty() {
        if !full.is_empty() {
            full.push_str("\n\n");
        }
        for p in paths {
            full.push_str(&format!("@{p}\n"));
        }
    }
    (full, images)
}

#[tauri::command]
pub async fn conv_send(
    convs: State<'_, Convs>,
    conv_id: String,
    text: String,
    attachments: Vec<Attachment>,
) -> Result<(), String> {
    let conv = convs.get(&conv_id)?;
    match &conv.agent {
        Agent::Claude(s) => {
            let (full, images) = tokio::task::spawn_blocking({
                let atts = attachments.clone();
                move || split_attachments(&text, &atts)
            })
            .await
            .map_err(err)?;
            s.send(&full, images, attachments);
        }
        Agent::Codex(t) => t.send(&text, attachments).await.map_err(err)?,
    }
    Ok(())
}

#[tauri::command]
pub async fn conv_interrupt(convs: State<'_, Convs>, conv_id: String) -> Result<(), String> {
    let conv = convs.get(&conv_id)?;
    match &conv.agent {
        Agent::Claude(s) => s.interrupt(),
        Agent::Codex(t) => t.interrupt().await.map_err(err)?,
    }
    Ok(())
}

#[tauri::command]
pub fn conv_permission(
    convs: State<'_, Convs>,
    conv_id: String,
    request_id: String,
    option_id: String,
    message: Option<String>,
) -> Result<(), String> {
    match &convs.get(&conv_id)?.agent {
        Agent::Claude(s) => s.answer(&request_id, &option_id, message),
        Agent::Codex(t) => t.answer(&request_id, &option_id),
    }
    Ok(())
}

#[tauri::command]
pub fn conv_set_mode(convs: State<'_, Convs>, conv_id: String, mode: String) -> Result<(), String> {
    match &convs.get(&conv_id)?.agent {
        Agent::Claude(s) => s.set_mode(&mode),
        Agent::Codex(t) => t.set_mode(&mode),
    }
    Ok(())
}

#[tauri::command]
pub fn conv_set_model(
    convs: State<'_, Convs>,
    conv_id: String,
    model: String,
) -> Result<(), String> {
    match &convs.get(&conv_id)?.agent {
        Agent::Claude(s) => s.set_model(&model),
        Agent::Codex(t) => t.set_model(&model),
    }
    Ok(())
}

#[tauri::command]
pub fn conv_close(convs: State<'_, Convs>, conv_id: String) -> Result<(), String> {
    let conv = convs.0.lock().remove(&conv_id);
    if let Some(conv) = conv {
        match &conv.agent {
            Agent::Claude(s) => s.close(),
            Agent::Codex(t) => t.close(),
        }
        conv.sink.close();
    }
    Ok(())
}

/// Stop every agent process, e.g. when the app quits.
pub fn close_all(convs: &Convs, pool: &CodexPool) {
    for (_, conv) in convs.0.lock().drain() {
        match &conv.agent {
            Agent::Claude(s) => s.close(),
            Agent::Codex(t) => t.close(),
        }
    }
    pool.0.shutdown_blocking();
}

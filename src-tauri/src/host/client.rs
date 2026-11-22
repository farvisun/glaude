//! The app's side of the host connection: starts the host when none is
//! running, sends requests, and routes each conversation's events to the
//! webview channel that asked for them.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::{json, Value};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

use super::{Reader, PROTO};

pub type EventChannel = Channel<Box<RawValue>>;

type Waiters = Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>;

pub struct HostClient {
    tx: mpsc::UnboundedSender<String>,
    waiters: Arc<Waiters>,
    channels: Arc<Mutex<HashMap<String, EventChannel>>>,
    next: AtomicU64,
    dead: Arc<AtomicBool>,
}

#[derive(Deserialize)]
struct Incoming {
    id: Option<u64>,
    ok: Option<Value>,
    error: Option<String>,
    conv: Option<String>,
    events: Option<Box<RawValue>>,
}

impl HostClient {
    pub fn is_dead(&self) -> bool {
        self.dead.load(Ordering::Relaxed)
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        if self.is_dead() {
            return Err("The agent host stopped".into());
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.waiters.lock().insert(id, tx);
        let line = json!({"id": id, "method": method, "params": params}).to_string();
        if self.tx.send(line).is_err() {
            self.waiters.lock().remove(&id);
            return Err("The agent host stopped".into());
        }
        match rx.await {
            Ok(r) => r,
            Err(_) => Err("The agent host stopped".into()),
        }
    }

    pub fn set_channel(&self, conv: &str, ch: EventChannel) {
        self.channels.lock().insert(conv.to_string(), ch);
    }

    pub fn drop_channel(&self, conv: &str) {
        self.channels.lock().remove(conv);
    }
}

fn spawn_host() -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--host")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Own process group: Ctrl+C or the app exiting does not reach it.
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW
        cmd.creation_flags(0x0000_0008 | 0x0000_0200 | 0x0800_0000);
    }
    let mut child = cmd.spawn()?;
    // Reap it if it exits while the app runs.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

async fn open(app: &AppHandle) -> Result<(Arc<HostClient>, Value), String> {
    let (reader, mut writer) = super::connect().await.map_err(|e| e.to_string())?;
    let token = super::read_token().map_err(|e| format!("host token: {e}"))?;
    let hello = json!({"id": 0, "method": "hello", "params": {"token": token, "proto": PROTO, "stamp": super::exe_stamp()}});
    writer
        .write_all(format!("{hello}\n").as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line))
        .await
        .map_err(|_| "the agent host did not answer".to_string())?
        .map_err(|e| e.to_string())?;
    let reply: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
    if let Some(e) = reply.get("error").and_then(Value::as_str) {
        return Err(e.to_string());
    }
    let info = reply.get("ok").cloned().unwrap_or_default();

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let waiters: Arc<Waiters> = Arc::default();
    let channels: Arc<Mutex<HashMap<String, EventChannel>>> = Arc::default();
    let dead = Arc::new(AtomicBool::new(false));
    tokio::spawn(async move {
        while let Some(l) = rx.recv().await {
            if writer.write_all(l.as_bytes()).await.is_err()
                || writer.write_all(b"\n").await.is_err()
            {
                break;
            }
        }
    });
    {
        let waiters = waiters.clone();
        let channels = channels.clone();
        let dead = dead.clone();
        let app = app.clone();
        tokio::spawn(read_loop(reader, waiters, channels, dead, app));
    }
    let client = Arc::new(HostClient {
        tx,
        waiters,
        channels,
        next: AtomicU64::new(1),
        dead,
    });
    Ok((client, info))
}

async fn read_loop(
    mut reader: BufReader<Reader>,
    waiters: Arc<Waiters>,
    channels: Arc<Mutex<HashMap<String, EventChannel>>>,
    dead: Arc<AtomicBool>,
    app: AppHandle,
) {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let Ok(msg) = serde_json::from_str::<Incoming>(&line) else {
            continue;
        };
        if let (Some(conv), Some(events)) = (msg.conv, msg.events) {
            let ch = channels.lock().get(&conv).cloned();
            if let Some(ch) = ch {
                if ch.send(events).is_err() {
                    channels.lock().remove(&conv);
                }
            }
        } else if let Some(id) = msg.id {
            if let Some(w) = waiters.lock().remove(&id) {
                let _ = w.send(match msg.error {
                    Some(e) => Err(e),
                    None => Ok(msg.ok.unwrap_or(Value::Null)),
                });
            }
        }
    }
    dead.store(true, Ordering::Relaxed);
    for (_, w) in waiters.lock().drain() {
        let _ = w.send(Err("The agent host stopped".into()));
    }
    tracing::warn!("lost the agent host");
    let _ = app.emit("host-lost", ());
}

/// Connect to the running host, or start one. A host from an older build
/// is replaced when it has nothing running.
pub async fn connect(app: &AppHandle) -> Result<Arc<HostClient>, String> {
    let mine = super::exe_stamp();
    let mut replaced = false;
    for attempt in 0..3 {
        match open(app).await {
            Ok((client, info)) => {
                let stamp = info.get("stamp").and_then(Value::as_str).unwrap_or("");
                let proto = info.get("proto").and_then(Value::as_u64).unwrap_or(0);
                let busy = info.get("busy").and_then(Value::as_bool).unwrap_or(true);
                let outdated = stamp != mine || proto != u64::from(PROTO);
                if outdated && !busy && !replaced {
                    tracing::info!("replacing an outdated agent host");
                    replaced = true;
                    let _ = client.call("shutdown", json!({})).await;
                    for _ in 0..50 {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        if super::connect().await.is_err() {
                            break;
                        }
                    }
                    continue;
                }
                if proto != u64::from(PROTO) {
                    return Err("An older Glaude agent host is still running agents. Let them finish, then restart Glaude.".into());
                }
                return Ok(client);
            }
            Err(e) => {
                tracing::info!(attempt, "no agent host ({e}); starting one");
                spawn_host().map_err(|e| format!("Could not start the agent host: {e}"))?;
                for _ in 0..60 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    if super::connect().await.is_ok() {
                        break;
                    }
                }
            }
        }
    }
    Err("Could not reach the agent host".into())
}

/// Tauri state: one connection, reopened when the host goes away.
#[derive(Default)]
pub struct HostState(pub tokio::sync::Mutex<Option<Arc<HostClient>>>);

impl HostState {
    pub async fn get(&self, app: &AppHandle) -> Result<Arc<HostClient>, String> {
        let mut g = self.0.lock().await;
        if let Some(c) = g.as_ref().filter(|c| !c.is_dead()) {
            return Ok(c.clone());
        }
        let c = connect(app).await?;
        *g = Some(c.clone());
        Ok(c)
    }
}

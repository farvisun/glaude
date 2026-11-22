//! `codex app-server` over stdio: one process per CODEX_HOME (account),
//! many threads routed by `threadId`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use glaude_core::env;
use glaude_core::model::{
    Attachment, ConvEvent, Item, Limits, ModelChoice, SessionInfo, TurnState,
};
use glaude_core::procs;
use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

use crate::translate::{self, Translator};

pub type Sink = Arc<dyn Fn(Vec<ConvEvent>) + Send + Sync>;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

struct Route {
    translator: Mutex<Translator>,
    sink: Sink,
}

pub struct CodexServer {
    pub home: PathBuf,
    writer: mpsc::UnboundedSender<String>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>,
    next_id: AtomicU64,
    routes: Mutex<HashMap<String, Arc<Route>>>,
    alive: Mutex<bool>,
    kill: Mutex<Option<oneshot::Sender<()>>>,
    last_limits: Mutex<Option<Limits>>,
}

impl CodexServer {
    pub async fn start(home: &Path) -> Result<Arc<Self>> {
        let bin = env::codex_binary()
            .ok_or_else(|| anyhow!("Codex CLI not found. Install it or set CODEX_BIN."))?;
        let mut cmd = Command::new(&bin);
        cmd.arg("app-server")
            .env_clear()
            .envs(env::agent_env(&[(
                "CODEX_HOME",
                Some(home.display().to_string()),
            )]))
            .current_dir(env::home_dir())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(unix)]
        cmd.process_group(0);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000);
        let mut child = cmd
            .spawn()
            .with_context(|| format!("starting {} app-server", bin.display()))?;
        let pid = child.id();
        if let Some(pid) = pid {
            procs::register(pid);
        }
        let mut stdin = child.stdin.take().expect("stdin");
        let stdout = child.stdout.take().expect("stdout");
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        tokio::spawn(async move {
            while let Some(mut l) = rx.recv().await {
                l.push('\n');
                if stdin.write_all(l.as_bytes()).await.is_err() || stdin.flush().await.is_err() {
                    break;
                }
            }
        });
        let (kill_tx, kill_rx) = oneshot::channel::<()>();
        let server = Arc::new(Self {
            home: home.to_path_buf(),
            writer: tx,
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            routes: Mutex::new(HashMap::new()),
            alive: Mutex::new(true),
            kill: Mutex::new(Some(kill_tx)),
            last_limits: Mutex::new(None),
        });

        // Reader.
        {
            let weak = Arc::downgrade(&server);
            tokio::spawn(async move {
                let mut lines = BufReader::with_capacity(256 * 1024, stdout).lines();
                while let Ok(Some(l)) = lines.next_line().await {
                    let Some(s) = weak.upgrade() else { break };
                    if let Ok(v) = serde_json::from_str::<Value>(&l) {
                        s.dispatch(v);
                    }
                }
            });
        }
        // Exit watcher.
        {
            let weak = Arc::downgrade(&server);
            tokio::spawn(async move {
                let code = tokio::select! {
                    s = child.wait() => s.ok().and_then(|s| s.code()),
                    _ = kill_rx => {
                        #[cfg(unix)]
                        if let Some(pid) = child.id() {
                            // SAFETY: signalling our own child's process group.
                            unsafe { libc::kill(-(pid as i32), libc::SIGTERM) };
                            tokio::time::sleep(Duration::from_millis(1500)).await;
                            unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
                        }
                        let _ = child.start_kill();
                        child.wait().await.ok().and_then(|s| s.code())
                    }
                };
                if let Some(pid) = pid {
                    procs::unregister(pid);
                }
                if let Some(s) = weak.upgrade() {
                    *s.alive.lock() = false;
                    for (_, tx) in s.pending.lock().drain() {
                        let _ = tx.send(Err("Codex app-server exited".into()));
                    }
                    for (_, r) in s.routes.lock().drain() {
                        (r.sink)(vec![ConvEvent::Exited { code }]);
                    }
                }
            });
        }

        server
            .request(
                "initialize",
                json!({
                    "clientInfo": {"name": "glaude", "title": "Glaude", "version": env!("CARGO_PKG_VERSION")},
                    "capabilities": {"experimentalApi": true, "requestAttestation": false},
                }),
            )
            .await
            .context("initializing Codex app-server")?;
        server.notify("initialized", Value::Null);
        Ok(server)
    }

    pub fn is_alive(&self) -> bool {
        *self.alive.lock()
    }

    fn write(&self, v: &Value) {
        let _ = self
            .writer
            .send(serde_json::to_string(v).unwrap_or_default());
    }

    pub fn notify(&self, method: &str, params: Value) {
        let mut m = json!({"method": method});
        if !params.is_null() {
            m["params"] = params;
        }
        self.write(&m);
    }

    pub async fn request(&self, method: &str, params: Value) -> Result<Value> {
        if !self.is_alive() {
            return Err(anyhow!("Codex app-server is not running"));
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().insert(id, tx);
        self.write(&json!({"id": id, "method": method, "params": params}));
        match tokio::time::timeout(REQUEST_TIMEOUT, rx).await {
            Ok(Ok(Ok(v))) => Ok(v),
            Ok(Ok(Err(e))) => Err(anyhow!("{method}: {e}")),
            Ok(Err(_)) => Err(anyhow!("{method}: no response")),
            Err(_) => {
                self.pending.lock().remove(&id);
                Err(anyhow!("{method}: timed out"))
            }
        }
    }

    fn route(&self, thread_id: &str) -> Option<Arc<Route>> {
        self.routes.lock().get(thread_id).cloned()
    }

    fn dispatch(&self, v: Value) {
        let method = v.get("method").and_then(Value::as_str).map(str::to_string);
        match (v.get("id"), method) {
            // Response to one of our requests.
            (Some(id), None) => {
                let Some(id) = id.as_u64() else { return };
                if let Some(tx) = self.pending.lock().remove(&id) {
                    let res = match v.get("error") {
                        Some(e) => Err(e
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("error")
                            .to_string()),
                        None => Ok(v.get("result").cloned().unwrap_or(Value::Null)),
                    };
                    let _ = tx.send(res);
                }
            }
            // Server request (approvals).
            (Some(id), Some(method)) => {
                let params = v.get("params").cloned().unwrap_or(Value::Null);
                let tid = params.get("threadId").and_then(Value::as_str).unwrap_or("");
                let handled = self.route(tid).and_then(|r| {
                    let evs = r.translator.lock().server_request(id, &method, &params)?;
                    (r.sink)(evs);
                    Some(())
                });
                if handled.is_none() {
                    self.write(&json!({"id": id, "error": {"code": -32601, "message": format!("Glaude does not handle {method}")}}));
                }
            }
            // Notification.
            (None, Some(method)) => {
                let params = v.get("params").cloned().unwrap_or(Value::Null);
                if method == "account/rateLimits/updated" {
                    let limits = params
                        .get("rateLimits")
                        .map(translate::limits_from)
                        .unwrap_or_default();
                    *self.last_limits.lock() = Some(limits.clone());
                    for r in self.routes.lock().values() {
                        (r.sink)(vec![ConvEvent::Limits {
                            limits: limits.clone(),
                        }]);
                    }
                    return;
                }
                let tid = params.get("threadId").and_then(Value::as_str).unwrap_or("");
                if let Some(r) = self.route(tid) {
                    let evs = r.translator.lock().notification(&method, &params);
                    if !evs.is_empty() {
                        (r.sink)(evs);
                    }
                }
            }
            (None, None) => {}
        }
    }

    pub fn shutdown(&self) {
        if let Some(k) = self.kill.lock().take() {
            let _ = k.send(());
        }
    }

    /// Past interactive threads for the sidebar, newest first.
    pub async fn list_threads(&self, limit: usize) -> Result<Vec<ThreadSummary>> {
        let mut out = Vec::new();
        let mut cursor: Option<String> = None;
        while out.len() < limit {
            let mut params = json!({
                "limit": 100,
                "sortKey": "updated_at",
                "sourceKinds": ["cli", "vscode", "appServer"],
            });
            if let Some(c) = &cursor {
                params["cursor"] = json!(c);
            }
            let r = self.request("thread/list", params).await?;
            for t in r
                .get("data")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if t.get("ephemeral").and_then(Value::as_bool).unwrap_or(false) {
                    continue;
                }
                if let Some(s) = ThreadSummary::from_thread(t) {
                    out.push(s);
                }
            }
            cursor = r
                .get("nextCursor")
                .and_then(Value::as_str)
                .map(str::to_string);
            if cursor.is_none() {
                break;
            }
        }
        out.truncate(limit);
        Ok(out)
    }

    /// Visible models with their reasoning efforts.
    pub async fn models(&self) -> Result<Vec<ModelChoice>> {
        let list = self.request("model/list", json!({})).await?;
        Ok(list
            .get("data")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter(|m| !m.get("hidden").and_then(Value::as_bool).unwrap_or(false))
                    .filter_map(|m| {
                        Some(ModelChoice {
                            value: m.get("id")?.as_str()?.to_string(),
                            label: m
                                .get("displayName")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                            efforts: m
                                .get("supportedReasoningEfforts")
                                .and_then(Value::as_array)
                                .map(|e| {
                                    e.iter()
                                        .filter_map(|x| {
                                            x.get("reasoningEffort").and_then(Value::as_str)
                                        })
                                        .map(str::to_string)
                                        .collect()
                                })
                                .unwrap_or_default(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn account(&self) -> Result<(Value, Limits)> {
        let acct = self
            .request("account/read", json!({}))
            .await
            .unwrap_or(Value::Null);
        let limits = self
            .request("account/rateLimits/read", Value::Null)
            .await
            .ok()
            .and_then(|r| r.get("rateLimits").map(translate::limits_from))
            .unwrap_or_default();
        Ok((acct, limits))
    }
}

/// A past thread for the sidebar.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThreadSummary {
    pub id: String,
    pub cwd: String,
    pub title: String,
    /// Unix ms of the last update.
    pub updated: u64,
    pub path: String,
}

impl ThreadSummary {
    fn from_thread(t: &Value) -> Option<Self> {
        let id = t.get("id")?.as_str()?.to_string();
        let name = t
            .get("name")
            .and_then(Value::as_str)
            .filter(|n| !n.trim().is_empty());
        let preview = t.get("preview").and_then(Value::as_str).unwrap_or("");
        let title: String = name
            .unwrap_or_else(|| preview.lines().find(|l| !l.trim().is_empty()).unwrap_or(""))
            .trim()
            .chars()
            .take(140)
            .collect();
        Some(Self {
            id,
            cwd: t
                .get("cwd")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            title: if title.is_empty() {
                "Untitled".into()
            } else {
                title
            },
            updated: t.get("updatedAt").and_then(Value::as_u64).unwrap_or(0) * 1000,
            path: t
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct CodexSpec {
    pub cwd: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    /// UI mode: full | on-request | untrusted | read-only
    pub mode: Option<String>,
    pub resume: Option<String>,
    pub load_history: bool,
}

#[derive(Default)]
struct Overrides {
    model: Option<String>,
    effort: Option<String>,
    mode: Option<String>,
}

pub struct CodexThread {
    server: Arc<CodexServer>,
    pub thread_id: String,
    route: Arc<Route>,
    overrides: Mutex<Overrides>,
}

fn thread_session(thread: &Value, resp: &Value) -> SessionInfo {
    SessionInfo {
        session_id: thread.get("id").and_then(Value::as_str).map(str::to_string),
        model: resp
            .get("model")
            .or(thread.get("model"))
            .and_then(Value::as_str)
            .map(str::to_string),
        effort: resp
            .get("reasoningEffort")
            .or(thread.get("reasoningEffort"))
            .and_then(Value::as_str)
            .map(str::to_string),
        permission_mode: Some(translate::mode_from(
            resp.get("approvalPolicy").unwrap_or(&Value::Null),
            resp.get("sandbox").unwrap_or(&Value::Null),
        )),
        cwd: thread
            .get("cwd")
            .and_then(Value::as_str)
            .map(str::to_string),
        git_branch: thread
            .get("gitInfo")
            .and_then(|g| g.get("branch"))
            .and_then(Value::as_str)
            .map(str::to_string),
        version: thread
            .get("cliVersion")
            .and_then(Value::as_str)
            .map(str::to_string),
        ..Default::default()
    }
}

impl CodexThread {
    pub async fn open(server: Arc<CodexServer>, spec: CodexSpec, sink: Sink) -> Result<Self> {
        if !Path::new(&spec.cwd).is_dir() {
            return Err(anyhow!("Folder not found: {}", spec.cwd));
        }
        let mode = spec.mode.clone().unwrap_or_else(|| "full".into());
        let (approval, sandbox, _) = translate::mode_policy(&mode);
        let mut params = json!({"cwd": spec.cwd, "approvalPolicy": approval, "sandbox": sandbox});
        if let Some(m) = spec.model.as_deref().filter(|m| !m.is_empty()) {
            params["model"] = json!(m);
        }
        let resp = match spec.resume.as_deref() {
            Some(tid) => {
                params["threadId"] = json!(tid);
                params["excludeTurns"] = json!(true);
                server.request("thread/resume", params).await?
            }
            None => server.request("thread/start", params).await?,
        };
        let thread = resp.get("thread").cloned().unwrap_or(Value::Null);
        let thread_id = thread
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Codex did not return a thread id"))?
            .to_string();

        let route = Arc::new(Route {
            translator: Mutex::new(Translator::new(&spec.cwd)),
            sink: sink.clone(),
        });
        server
            .routes
            .lock()
            .insert(thread_id.clone(), route.clone());

        let mut info = thread_session(&thread, &resp);
        info.permission_mode = Some(mode.clone());
        if info.effort.is_none() {
            info.effort = spec.effort.clone();
        }
        // Models for the statusline picker.
        info.models = server.models().await.unwrap_or_default();
        let mut first = vec![ConvEvent::Session { info }];
        if let Some(l) = server.last_limits.lock().clone() {
            first.push(ConvEvent::Limits { limits: l });
        }
        sink(first);

        let t = Self {
            server: server.clone(),
            thread_id: thread_id.clone(),
            route,
            overrides: Mutex::new(Overrides {
                model: None,
                effort: spec.effort.clone(),
                mode: Some(mode),
            }),
        };

        if spec.resume.is_some() && spec.load_history {
            let items = t.load_history(&spec.cwd).await.unwrap_or_default();
            sink(vec![ConvEvent::Reset { items }]);
        }
        if server.last_limits.lock().is_none() {
            if let Ok((_, limits)) = server.account().await {
                *server.last_limits.lock() = Some(limits.clone());
                sink(vec![ConvEvent::Limits { limits }]);
            }
        }
        Ok(t)
    }

    async fn load_history(&self, cwd: &str) -> Result<Vec<Item>> {
        let mut tr = Translator::for_history(cwd);
        let mut items: Vec<Item> = Vec::new();
        let mut index: HashMap<String, usize> = HashMap::new();
        let mut cursor: Option<String> = None;
        for _ in 0..20 {
            let mut params = json!({"threadId": self.thread_id, "limit": 50, "sortDirection": "asc", "itemsView": "full"});
            if let Some(c) = &cursor {
                params["cursor"] = json!(c);
            }
            let page = self.server.request("thread/turns/list", params).await?;
            for turn in page
                .get("data")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                for it in turn
                    .get("items")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    for ev in tr.item(it, true) {
                        if let ConvEvent::Item { item } = ev {
                            let id = item.id().to_string();
                            match index.get(&id) {
                                Some(&i) => items[i] = item,
                                None => {
                                    index.insert(id, items.len());
                                    items.push(item);
                                }
                            }
                        }
                    }
                }
            }
            cursor = page
                .get("nextCursor")
                .and_then(Value::as_str)
                .map(str::to_string);
            if cursor.is_none() {
                break;
            }
        }
        Ok(items)
    }

    fn emit(&self, events: Vec<ConvEvent>) {
        if !events.is_empty() {
            (self.route.sink)(events);
        }
    }

    pub async fn send(&self, text: &str, attachments: Vec<Attachment>) -> Result<()> {
        let mut input = Vec::new();
        let mut extra_paths = Vec::new();
        for a in &attachments {
            if a.mime.starts_with("image/") {
                input.push(json!({"type": "localImage", "path": a.path}));
            } else {
                extra_paths.push(a.path.clone());
            }
        }
        let mut full = text.to_string();
        for p in extra_paths {
            full.push_str(&format!("\n\nAttached file: {p}"));
        }
        if !full.trim().is_empty() {
            input.insert(
                0,
                json!({"type": "text", "text": full, "text_elements": []}),
            );
        }
        let (client_id, events) = self.route.translator.lock().user_message(text, attachments);
        self.emit(events);
        let mut params =
            json!({"threadId": self.thread_id, "input": input, "clientUserMessageId": client_id});
        {
            let o = self.overrides.lock();
            if let Some(m) = &o.model {
                params["model"] = json!(m);
            }
            if let Some(e) = &o.effort {
                params["effort"] = json!(e);
            }
            if let Some(mode) = &o.mode {
                let (approval, _, policy) = translate::mode_policy(mode);
                params["approvalPolicy"] = json!(approval);
                params["sandboxPolicy"] = policy;
            }
        }
        if let Err(e) = self.server.request("turn/start", params).await {
            self.emit(vec![ConvEvent::TurnDone {
                duration_ms: None,
                error: Some(e.to_string()),
            }]);
            return Err(e);
        }
        Ok(())
    }

    pub async fn interrupt(&self) -> Result<()> {
        let turn = self.route.translator.lock().turn_id.clone();
        if let Some(turn_id) = turn {
            self.server
                .request(
                    "turn/interrupt",
                    json!({"threadId": self.thread_id, "turnId": turn_id}),
                )
                .await?;
        }
        Ok(())
    }

    pub fn answer(&self, key: &str, option: &str) {
        let (line, events) = self.route.translator.lock().answer(key, option);
        if let Some(l) = line {
            self.server.write(&l);
        }
        self.emit(events);
    }

    pub fn set_mode(&self, mode: &str) {
        self.overrides.lock().mode = Some(mode.to_string());
        self.emit(vec![ConvEvent::Session {
            info: SessionInfo {
                permission_mode: Some(mode.to_string()),
                ..Default::default()
            },
        }]);
    }

    pub fn set_model(&self, model: &str) {
        self.overrides.lock().model = Some(model.to_string());
        self.emit(vec![ConvEvent::Session {
            info: SessionInfo {
                model: Some(model.to_string()),
                ..Default::default()
            },
        }]);
    }

    pub fn close(&self) {
        self.server.routes.lock().remove(&self.thread_id);
        let _ = self.server.writer.send(
            serde_json::to_string(&json!({"id": 0u64, "method": "thread/unsubscribe", "params": {"threadId": self.thread_id}}))
                .unwrap_or_default(),
        );
        self.emit(vec![ConvEvent::Turn {
            state: TurnState::Idle,
            detail: None,
        }]);
    }
}

/// Pool of app-servers keyed by CODEX_HOME.
#[derive(Default)]
pub struct Pool {
    servers: tokio::sync::Mutex<HashMap<PathBuf, Arc<CodexServer>>>,
}

impl Pool {
    pub async fn get(&self, home: &Path) -> Result<Arc<CodexServer>> {
        let mut map = self.servers.lock().await;
        if let Some(s) = map.get(home).filter(|s| s.is_alive()) {
            return Ok(s.clone());
        }
        let s = CodexServer::start(home).await?;
        map.insert(home.to_path_buf(), s.clone());
        Ok(s)
    }

    /// The running app-server for `home`, if Glaude already started one.
    pub async fn existing(&self, home: &Path) -> Option<Arc<CodexServer>> {
        self.servers
            .lock()
            .await
            .get(home)
            .filter(|s| s.is_alive())
            .cloned()
    }

    pub async fn shutdown_all(&self) {
        for (_, s) in self.servers.lock().await.drain() {
            s.shutdown();
        }
    }

    /// Synchronous shutdown for app exit.
    pub fn shutdown_blocking(&self) {
        if let Ok(mut map) = self.servers.try_lock() {
            for (_, s) in map.drain() {
                s.shutdown();
            }
        }
    }
}

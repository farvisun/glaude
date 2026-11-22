//! The host process: owns agent processes, keeps a snapshot of every
//! conversation, streams events to attached clients, drains message queues
//! and scheduled resumes, and stops what is idle once no window is open.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use base64::Engine;
use glaude_accounts::paths;
use glaude_claude::{ClaudeSession, ClaudeSpec};
use glaude_codex::{CodexSpec, CodexThread};
use glaude_core::model::{Attachment, ConvEvent, QueuedMessage, SessionInfo, TurnState};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Notify};

use super::state::Snapshot;
use super::{Listener, Reader, Writer, PROTO};
use crate::sink::push_merged;

const FLUSH_MS: u64 = 24;
/// A conversation waiting for an answer keeps its process this long with
/// no window open.
const WAIT_LIMIT_MS: u64 = 6 * 3600 * 1000;
/// The host exits this long after its last client and conversation.
fn quiet_exit() -> Duration {
    let secs = std::env::var("GLAUDE_HOST_QUIET_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);
    Duration::from_secs(secs)
}
/// Requests bigger than this are refused (pasted text, attachments by path).
const MAX_LINE: usize = 64 << 20;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    /// Claude: resume only up to this message (rewind).
    pub resume_at: Option<String>,
    /// Claude: run the session as this agent.
    pub agent: Option<String>,
}

pub enum Agent {
    Claude(ClaudeSession),
    Codex(CodexThread),
}

impl Agent {
    fn close(&self) {
        match self {
            Agent::Claude(s) => s.close(),
            Agent::Codex(t) => t.close(),
        }
    }
}

struct Data {
    snap: Snapshot,
    pending: Vec<ConvEvent>,
    subs: HashSet<u64>,
    alive: bool,
}

pub struct HostConv {
    id: String,
    spec: Mutex<StartSpec>,
    agent: Mutex<Option<Arc<Agent>>>,
    data: Mutex<Data>,
    branch: Mutex<Option<String>>,
    host: Weak<Host>,
}

pub struct Host {
    convs: Mutex<HashMap<String, Arc<HostConv>>>,
    pool: glaude_codex::Pool,
    clients: Mutex<HashMap<u64, mpsc::UnboundedSender<String>>>,
    next_client: AtomicU64,
    stamp: String,
    token: String,
    quiet_since: Mutex<Option<Instant>>,
    exit: Notify,
    /// Desktop notifications while no window is open (app setting).
    notify: std::sync::atomic::AtomicBool,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn rfc3339_ms(t: &str) -> Option<u64> {
    chrono::DateTime::parse_from_rfc3339(t)
        .ok()
        .map(|d| d.timestamp_millis().max(0) as u64)
}

fn is_limit_error(e: &str) -> bool {
    let e = e.to_lowercase();
    e.contains("usage limit") || e.contains("limit reached") || e.contains("rate limit")
}

/// Images go to Claude inline (base64); other files are referenced by path.
fn split_attachments(text: &str, attachments: &[Attachment]) -> (String, Vec<(String, String)>) {
    let mut images = Vec::new();
    let mut files = Vec::new();
    for a in attachments {
        if a.mime.starts_with("image/") {
            match std::fs::read(&a.path) {
                Ok(bytes) => images.push((
                    a.mime.clone(),
                    base64::engine::general_purpose::STANDARD.encode(bytes),
                )),
                Err(_) => files.push(a.path.clone()),
            }
        } else {
            files.push(a.path.clone());
        }
    }
    let mut full = text.to_string();
    if !files.is_empty() {
        if !full.is_empty() {
            full.push_str("\n\n");
        }
        for p in files {
            full.push_str(&format!("@{p}\n"));
        }
    }
    (full, images)
}

/// A desktop notification from the host, for news while no window is open.
fn notify(title: &str, body: &str) {
    if std::env::var_os("GLAUDE_HOST_DIR").is_some() {
        return; // tests
    }
    let (title, body) = (title.to_string(), body.to_string());
    std::thread::spawn(move || {
        #[cfg(target_os = "macos")]
        {
            let q = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
            let script = format!(
                "display notification \"{}\" with title \"{}\"",
                q(&body),
                q(&title)
            );
            let _ = std::process::Command::new("osascript")
                .args(["-e", &script])
                .output();
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let _ = std::process::Command::new("notify-send")
                .args(["--app-name=Glaude", &title, &body])
                .output();
        }
        #[cfg(windows)]
        {
            let _ = (title, body);
        }
    });
}

fn str_param<'a>(p: &'a Value, k: &str) -> Result<&'a str, String> {
    p.get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing {k}"))
}

impl HostConv {
    fn sink(self: &Arc<Self>) -> glaude_claude::Sink {
        let weak = Arc::downgrade(self);
        Arc::new(move |events: Vec<ConvEvent>| {
            if let Some(c) = weak.upgrade() {
                c.on_events(events);
            }
        })
    }

    /// Record and forward events (to the snapshot and attached clients).
    fn emit(&self, events: Vec<ConvEvent>) {
        let mut d = self.data.lock();
        for e in events {
            d.snap.apply(&e);
            push_merged(&mut d.pending, e);
        }
    }

    fn branch_event(&self) -> Option<ConvEvent> {
        let cwd = self.spec.lock().cwd.clone();
        let now = glaude_core::git::branch(std::path::Path::new(&cwd));
        let mut last = self.branch.lock();
        if *last == now {
            return None;
        }
        last.clone_from(&now);
        Some(ConvEvent::Session {
            info: SessionInfo {
                git_branch: Some(now.unwrap_or_default()),
                ..Default::default()
            },
        })
    }

    fn on_events(self: &Arc<Self>, mut events: Vec<ConvEvent>) {
        let turn_end = events
            .iter()
            .any(|e| matches!(e, ConvEvent::TurnDone { .. } | ConvEvent::Reset { .. }));
        let exited = events.iter().any(|e| matches!(e, ConvEvent::Exited { .. }));
        if turn_end {
            events.extend(self.branch_event());
        }
        let mut limited: Option<Option<String>> = None;
        let mut working = false;
        let finished_ok = events
            .iter()
            .any(|e| matches!(e, ConvEvent::TurnDone { error: None, .. }));
        let prompt_now = events.iter().any(|e| {
            matches!(
                e,
                ConvEvent::Item {
                    item: glaude_core::model::Item::Permission {
                        status: glaude_core::model::Status::Pending,
                        ..
                    } | glaude_core::model::Item::Question {
                        status: glaude_core::model::Status::Pending,
                        ..
                    }
                }
            )
        });
        for e in &events {
            match e {
                ConvEvent::Limited { until } => limited = Some(until.clone()),
                ConvEvent::TurnDone {
                    error: Some(err), ..
                } if is_limit_error(err) => {
                    limited = limited.or(Some(None));
                }
                ConvEvent::Turn {
                    state: TurnState::Writing | TurnState::Tool | TurnState::Thinking,
                    ..
                } => working = true,
                _ => {}
            }
        }
        {
            let mut d = self.data.lock();
            for e in events {
                d.snap.apply(&e);
                push_merged(&mut d.pending, e);
            }
            let schedule = if let Some(until) = limited {
                let until = until.or_else(|| d.snap.limits_reset()).unwrap_or_default();
                Some(ConvEvent::Schedule {
                    limited_until: Some(until),
                    resume_at: d.snap.resume_at.clone(),
                })
            } else if working && d.snap.limited_until.is_some() {
                // A turn is running again: the limit is over.
                Some(ConvEvent::Schedule {
                    limited_until: None,
                    resume_at: d.snap.resume_at.clone(),
                })
            } else {
                None
            };
            if let Some(ev) = schedule {
                d.snap.apply(&ev);
                push_merged(&mut d.pending, ev);
            }
            if exited {
                d.alive = false;
            }
        }
        let alone = self
            .host
            .upgrade()
            .is_some_and(|h| h.clients.lock().is_empty() && h.notify.load(Ordering::Relaxed));
        if alone {
            let folder = std::path::Path::new(&self.spec.lock().cwd)
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default();
            if prompt_now {
                notify("Glaude", &format!("{folder}: an agent needs your answer"));
            } else if finished_ok {
                notify("Glaude", &format!("{folder}: the agent finished"));
            }
        }
        if turn_end || exited {
            let me = self.clone();
            tokio::spawn(async move { me.after_turn().await });
        }
    }

    /// Send the next queued message once the turn is over, then let the
    /// host stop what no longer needs to run.
    async fn after_turn(self: Arc<Self>) {
        let next = {
            let mut d = self.data.lock();
            let blocked = d.snap.turn_running()
                || d.snap.pending_prompt()
                || d.snap.limited_until.is_some()
                || d.snap.queue.is_empty();
            if blocked {
                None
            } else {
                let mut q = d.snap.queue.clone();
                let m = q.remove(0);
                let ev = ConvEvent::Queue { items: q };
                d.snap.apply(&ev);
                push_merged(&mut d.pending, ev);
                Some(m)
            }
        };
        if let Some(m) = next {
            if let Err(e) = self.deliver(&m.text, m.attachments, false).await {
                self.emit(vec![ConvEvent::Notice {
                    level: "error".into(),
                    text: format!("Queued message not sent: {e}"),
                }]);
            }
        }
        if let Some(h) = self.host.upgrade() {
            h.sweep();
        }
    }

    fn agent(&self) -> Option<Arc<Agent>> {
        let alive = self.data.lock().alive;
        if alive {
            self.agent.lock().clone()
        } else {
            None
        }
    }

    /// The running agent, restarting an ended process on the same session.
    async fn ensure_agent(self: &Arc<Self>) -> Result<Arc<Agent>, String> {
        if let Some(a) = self.agent() {
            return Ok(a);
        }
        let host = self.host.upgrade().ok_or("host is stopping")?;
        let spec = {
            let d = self.data.lock();
            let mut s = self.spec.lock().clone();
            s.resume = d.snap.session.session_id.clone().or(s.resume);
            s.fork = false;
            s.resume_at = None;
            s.skip_history = true;
            s.model = d.snap.session.model.clone().or(s.model);
            s.permission_mode = d.snap.session.permission_mode.clone().or(s.permission_mode);
            s
        };
        let agent = host.spawn_agent(self, &spec).await?;
        Ok(agent)
    }

    async fn deliver(
        self: &Arc<Self>,
        text: &str,
        attachments: Vec<Attachment>,
        steer: bool,
    ) -> Result<(), String> {
        let agent = self.ensure_agent().await?;
        match &*agent {
            Agent::Claude(s) => {
                let t = text.to_string();
                let atts = attachments.clone();
                let (full, images) =
                    tokio::task::spawn_blocking(move || split_attachments(&t, &atts))
                        .await
                        .map_err(|e| e.to_string())?;
                s.send(&full, images, attachments);
            }
            Agent::Codex(t) => {
                if steer {
                    t.steer(text, attachments).await
                } else {
                    t.send(text, attachments).await
                }
                .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    /// Attach a client: it gets everything so far, then the live stream.
    fn attach(&self, client: u64, tx: &mpsc::UnboundedSender<String>) {
        let mut d = self.data.lock();
        // Flush what the current subscribers have not seen yet first, so the
        // new client's replay and the stream do not overlap.
        let batch = std::mem::take(&mut d.pending);
        if !batch.is_empty() {
            if let Some(line) = event_line(&self.id, &batch) {
                if let Some(h) = self.host.upgrade() {
                    let clients = h.clients.lock();
                    for c in &d.subs {
                        if let Some(t) = clients.get(c) {
                            let _ = t.send(line.clone());
                        }
                    }
                }
            }
        }
        if let Some(line) = event_line(&self.id, &d.snap.replay()) {
            let _ = tx.send(line);
        }
        d.subs.insert(client);
    }

    fn flush(&self, clients: &HashMap<u64, mpsc::UnboundedSender<String>>) {
        let mut d = self.data.lock();
        if d.pending.is_empty() {
            return;
        }
        let batch = std::mem::take(&mut d.pending);
        if d.subs.is_empty() {
            return;
        }
        if let Some(line) = event_line(&self.id, &batch) {
            for c in &d.subs {
                if let Some(t) = clients.get(c) {
                    let _ = t.send(line.clone());
                }
            }
        }
    }

    fn close(&self) {
        if let Some(a) = self.agent.lock().take() {
            a.close();
        }
        self.data.lock().alive = false;
    }

    fn brief(&self) -> Value {
        let spec = self.spec.lock().clone();
        let d = self.data.lock();
        json!({
            "convId": self.id,
            "provider": spec.provider,
            "account": spec.account,
            "cwd": spec.cwd,
            "sessionId": d.snap.session.session_id,
            "alive": d.alive,
            "busy": d.snap.busy(),
            "turn": d.snap.turn.unwrap_or(TurnState::Idle),
        })
    }
}

fn event_line(conv: &str, events: &[ConvEvent]) -> Option<String> {
    serde_json::to_string(&json!({"conv": conv, "events": events})).ok()
}

/// Control requests a client may send to a Claude session.
const CONTROL_ALLOWED: &[&str] = &[
    "get_context_usage",
    "mcp_status",
    "mcp_reconnect",
    "mcp_toggle",
    "get_skills_dialog",
    "get_hooks_listing",
    "get_memory_dialog",
    "get_task_output",
    "stop_task",
    "rewind_files",
    "apply_flag_settings",
    "file_suggestions",
];

impl Host {
    fn new(token: String) -> Arc<Self> {
        Arc::new(Self {
            convs: Mutex::default(),
            pool: glaude_codex::Pool::default(),
            clients: Mutex::default(),
            next_client: AtomicU64::new(1),
            stamp: super::exe_stamp(),
            token,
            quiet_since: Mutex::new(Some(Instant::now())),
            exit: Notify::new(),
            notify: std::sync::atomic::AtomicBool::new(true),
        })
    }

    fn conv(&self, id: &str) -> Result<Arc<HostConv>, String> {
        self.convs
            .lock()
            .get(id)
            .cloned()
            .ok_or_else(|| "This conversation is not running. Send a message to restart it.".into())
    }

    fn busy(&self) -> bool {
        self.convs.lock().values().any(|c| {
            let d = c.data.lock();
            d.alive && d.snap.busy()
        })
    }

    async fn spawn_agent(
        &self,
        conv: &Arc<HostConv>,
        spec: &StartSpec,
    ) -> Result<Arc<Agent>, String> {
        let sink = conv.sink();
        let agent = match spec.provider.as_str() {
            "claude" => Agent::Claude(
                ClaudeSession::spawn(
                    ClaudeSpec {
                        cwd: spec.cwd.clone(),
                        config_dir: paths::claude_config_dir(&spec.account),
                        model: spec.model.clone(),
                        effort: spec.effort.clone(),
                        permission_mode: spec.permission_mode.clone(),
                        resume: spec.resume.clone(),
                        fork: spec.fork,
                        resume_at: spec.resume_at.clone(),
                        agent: spec.agent.clone(),
                        load_history: !spec.skip_history,
                    },
                    sink,
                )
                .await
                .map_err(|e| e.to_string())?,
            ),
            "codex" => {
                let server = self
                    .pool
                    .get(&paths::codex_home(&spec.account))
                    .await
                    .map_err(|e| e.to_string())?;
                Agent::Codex(
                    CodexThread::open(
                        server,
                        CodexSpec {
                            cwd: spec.cwd.clone(),
                            model: spec.model.clone(),
                            effort: spec.effort.clone(),
                            mode: spec.permission_mode.clone(),
                            resume: spec.resume.clone(),
                            fork: spec.fork,
                            load_history: !spec.skip_history,
                        },
                        sink,
                    )
                    .await
                    .map_err(|e| e.to_string())?,
                )
            }
            other => return Err(format!("{other} conversations are not supported")),
        };
        let agent = Arc::new(agent);
        if let Some(old) = conv.agent.lock().replace(agent.clone()) {
            old.close();
        }
        conv.data.lock().alive = true;
        if let Some(ev) = conv.branch_event() {
            conv.emit(vec![ev]);
        }
        Ok(agent)
    }

    async fn start(self: &Arc<Self>, client: u64, p: &Value) -> Result<Value, String> {
        let id = str_param(p, "convId")?.to_string();
        let spec: StartSpec = serde_json::from_value(p.get("spec").cloned().unwrap_or_default())
            .map_err(|e| e.to_string())?;
        if let Some(old) = self.convs.lock().remove(&id) {
            old.close();
        }
        let conv = Arc::new(HostConv {
            id: id.clone(),
            spec: Mutex::new(spec.clone()),
            agent: Mutex::new(None),
            data: Mutex::new(Data {
                snap: Snapshot::default(),
                pending: Vec::new(),
                subs: HashSet::from([client]),
                alive: false,
            }),
            branch: Mutex::new(None),
            host: Arc::downgrade(self),
        });
        self.convs.lock().insert(id.clone(), conv.clone());
        *self.quiet_since.lock() = None;
        // mamadoo's launch bookkeeping; a restart is not a new launch.
        let shared =
            crate::accounts::prepare_launch(&spec.provider, &spec.account, !spec.skip_history)
                .await;
        if !shared.is_empty() {
            conv.emit(vec![ConvEvent::Notice {
                level: "info".into(),
                text: format!("Shared home: {}", shared.join("; ")),
            }]);
        }
        if let Err(e) = self.spawn_agent(&conv, &spec).await {
            self.convs.lock().remove(&id);
            return Err(e);
        }
        Ok(Value::Null)
    }

    async fn handle(
        self: Arc<Self>,
        client: u64,
        method: String,
        p: Value,
    ) -> Result<Value, String> {
        let conv_id = p.get("convId").and_then(Value::as_str).unwrap_or("");
        match method.as_str() {
            "list" => Ok(Value::Array(
                self.convs.lock().values().map(|c| c.brief()).collect(),
            )),
            "start" => self.start(client, &p).await,
            "attach" => {
                let conv = self.conv(conv_id)?;
                let tx = self
                    .clients
                    .lock()
                    .get(&client)
                    .cloned()
                    .ok_or("not connected")?;
                conv.attach(client, &tx);
                Ok(conv.brief())
            }
            "detach" => {
                if let Ok(c) = self.conv(conv_id) {
                    c.data.lock().subs.remove(&client);
                }
                Ok(Value::Null)
            }
            "send" | "steer" => {
                let conv = self.conv(conv_id)?;
                let text = str_param(&p, "text")?.to_string();
                let atts: Vec<Attachment> =
                    serde_json::from_value(p.get("attachments").cloned().unwrap_or(json!([])))
                        .map_err(|e| e.to_string())?;
                conv.deliver(&text, atts, method == "steer").await?;
                Ok(Value::Null)
            }
            "queue" => {
                let conv = self.conv(conv_id)?;
                let items: Vec<QueuedMessage> =
                    serde_json::from_value(p.get("items").cloned().unwrap_or(json!([])))
                        .map_err(|e| e.to_string())?;
                conv.emit(vec![ConvEvent::Queue { items }]);
                tokio::spawn(conv.clone().after_turn());
                Ok(Value::Null)
            }
            "schedule" => {
                let conv = self.conv(conv_id)?;
                let resume_at = p
                    .get("resumeAt")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let limited = conv.data.lock().snap.limited_until.clone();
                conv.emit(vec![ConvEvent::Schedule {
                    limited_until: limited,
                    resume_at,
                }]);
                Ok(Value::Null)
            }
            "interrupt" => {
                let conv = self.conv(conv_id)?;
                if let Some(a) = conv.agent() {
                    match &*a {
                        Agent::Claude(s) => s.interrupt(),
                        Agent::Codex(t) => t.interrupt().await.map_err(|e| e.to_string())?,
                    }
                }
                Ok(Value::Null)
            }
            "permission" => {
                let a = self.conv(conv_id)?.ensure_agent().await?;
                let rid = str_param(&p, "requestId")?;
                let opt = str_param(&p, "optionId")?;
                let msg = p.get("message").and_then(Value::as_str).map(str::to_string);
                match &*a {
                    Agent::Claude(s) => s.answer(rid, opt, msg),
                    Agent::Codex(t) => t.answer(rid, opt),
                }
                Ok(Value::Null)
            }
            "answer_question" => {
                let a = self.conv(conv_id)?.ensure_agent().await?;
                let rid = str_param(&p, "requestId")?;
                let answers = serde_json::from_value(p.get("answers").cloned().unwrap_or_default())
                    .map_err(|e| e.to_string())?;
                match &*a {
                    Agent::Claude(s) => s.answer_question(rid, answers),
                    Agent::Codex(_) => return Err("Not available for Codex conversations".into()),
                }
                Ok(Value::Null)
            }
            "set_mode" | "set_model" => {
                let conv = self.conv(conv_id)?;
                let v = str_param(&p, "value")?;
                if let Some(a) = conv.agent() {
                    match (&*a, method.as_str()) {
                        (Agent::Claude(s), "set_mode") => s.set_mode(v),
                        (Agent::Claude(s), _) => s.set_model(v),
                        (Agent::Codex(t), "set_mode") => t.set_mode(v),
                        (Agent::Codex(t), _) => t.set_model(v),
                    }
                } else {
                    let mut s = conv.spec.lock();
                    if method == "set_mode" {
                        s.permission_mode = Some(v.into());
                    } else {
                        s.model = Some(v.into());
                    }
                }
                Ok(Value::Null)
            }
            "control" => {
                let req = p.get("request").cloned().unwrap_or_default();
                let sub = req.get("subtype").and_then(Value::as_str).unwrap_or("");
                if !CONTROL_ALLOWED.contains(&sub) {
                    return Err(format!("{sub} is not available"));
                }
                let a = self.conv(conv_id)?.ensure_agent().await?;
                match &*a {
                    Agent::Claude(s) => s.request(req).await.map_err(|e| e.to_string()),
                    Agent::Codex(_) => Err("Not available for Codex conversations yet".into()),
                }
            }
            "hook_runs" => {
                let conv = self.conv(conv_id)?;
                match conv.agent().as_deref() {
                    Some(Agent::Claude(s)) => {
                        serde_json::to_value(s.hook_runs()).map_err(|e| e.to_string())
                    }
                    _ => Ok(json!([])),
                }
            }
            "close" => {
                if let Some(c) = self.convs.lock().remove(conv_id) {
                    c.close();
                }
                self.sweep();
                Ok(Value::Null)
            }
            "config" => {
                if let Some(n) = p.get("notify").and_then(Value::as_bool) {
                    self.notify.store(n, Ordering::Relaxed);
                }
                Ok(Value::Null)
            }
            "close_all" => {
                let all: Vec<_> = self.convs.lock().drain().map(|(_, c)| c).collect();
                for c in all {
                    c.close();
                }
                self.sweep();
                Ok(Value::Null)
            }
            "shutdown" => {
                if self.busy() {
                    return Err("agents are still working".into());
                }
                self.exit.notify_one();
                Ok(Value::Null)
            }
            other => Err(format!("unknown request {other}")),
        }
    }

    /// With no window open: stop conversations that have nothing to do,
    /// and plan to exit once none are left.
    fn sweep(&self) {
        if !self.clients.lock().is_empty() {
            *self.quiet_since.lock() = None;
            return;
        }
        let now = now_ms();
        let mut stopped = Vec::new();
        self.convs.lock().retain(|_, c| {
            let d = c.data.lock();
            let stale_wait = d
                .snap
                .waiting_since
                .is_some_and(|w| now.saturating_sub(w) > WAIT_LIMIT_MS);
            let pending_work = !d.snap.queue.is_empty() || d.snap.resume_at.is_some();
            let keep = d.snap.busy() && (d.alive || pending_work) && !stale_wait;
            if !keep {
                stopped.push(c.clone());
            }
            keep
        });
        for c in &stopped {
            tracing::info!(conv = %c.id, "stopping idle conversation");
            c.close();
        }
        if self.convs.lock().is_empty() {
            self.quiet_since.lock().get_or_insert_with(Instant::now);
        }
    }

    /// Scheduled resumes and expired limits.
    fn tick_schedule(self: &Arc<Self>) {
        let now = now_ms();
        let convs: Vec<_> = self.convs.lock().values().cloned().collect();
        for c in convs {
            let (due, expired) = {
                let d = c.data.lock();
                let due = d
                    .snap
                    .resume_at
                    .as_deref()
                    .and_then(rfc3339_ms)
                    .is_some_and(|t| t <= now);
                let expired = d.snap.resume_at.is_none()
                    && d.snap
                        .limited_until
                        .as_deref()
                        .and_then(rfc3339_ms)
                        .is_some_and(|t| t + 60_000 <= now);
                (due, expired)
            };
            if due || expired {
                c.emit(vec![ConvEvent::Schedule {
                    limited_until: None,
                    resume_at: None,
                }]);
            }
            if due {
                let c = c.clone();
                tokio::spawn(async move {
                    tracing::info!(conv = %c.id, "resuming after the usage limit reset");
                    if let Err(e) = c.deliver("continue", Vec::new(), false).await {
                        c.emit(vec![ConvEvent::Notice {
                            level: "error".into(),
                            text: format!("Could not resume: {e}"),
                        }]);
                    }
                });
            }
        }
    }

    fn shutdown_all(&self) {
        for (_, c) in self.convs.lock().drain() {
            c.close();
        }
        self.pool.shutdown_blocking();
        glaude_core::procs::kill_all(Duration::from_millis(800));
    }
}

async fn read_line(r: &mut BufReader<Reader>, buf: &mut Vec<u8>) -> std::io::Result<bool> {
    buf.clear();
    loop {
        let chunk = r.fill_buf().await?;
        if chunk.is_empty() {
            return Ok(!buf.is_empty());
        }
        if let Some(i) = chunk.iter().position(|&b| b == b'\n') {
            buf.extend_from_slice(&chunk[..i]);
            r.consume(i + 1);
            return Ok(true);
        }
        let n = chunk.len();
        buf.extend_from_slice(chunk);
        r.consume(n);
        if buf.len() > MAX_LINE {
            return Err(std::io::Error::other("request too large"));
        }
    }
}

async fn serve_client(host: Arc<Host>, reader: Reader, mut writer: Writer) {
    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    // hello: {"id":0,"method":"hello","params":{"token","proto","stamp"}}
    let Ok(true) = read_line(&mut reader, &mut buf).await else {
        return;
    };
    let hello: Value = serde_json::from_slice(&buf).unwrap_or_default();
    let p = hello.get("params").cloned().unwrap_or_default();
    let id = hello.get("id").cloned().unwrap_or(json!(0));
    if p.get("token").and_then(Value::as_str) != Some(host.token.as_str()) {
        let _ = writer
            .write_all(format!("{}\n", json!({"id": id, "error": "bad token"})).as_bytes())
            .await;
        return;
    }
    let reply = json!({"id": id, "ok": {"proto": PROTO, "stamp": host.stamp, "busy": host.busy(), "pid": std::process::id()}});
    if writer
        .write_all(format!("{reply}\n").as_bytes())
        .await
        .is_err()
    {
        return;
    }

    let client = host.next_client.fetch_add(1, Ordering::Relaxed);
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    host.clients.lock().insert(client, tx.clone());
    *host.quiet_since.lock() = None;
    tracing::info!(client, "client connected");

    let writer_task = tokio::spawn(async move {
        while let Some(line) = rx.recv().await {
            if writer.write_all(line.as_bytes()).await.is_err()
                || writer.write_all(b"\n").await.is_err()
            {
                break;
            }
        }
    });

    while let Ok(true) = read_line(&mut reader, &mut buf).await {
        let Ok(req) = serde_json::from_slice::<Value>(&buf) else {
            continue;
        };
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let params = req.get("params").cloned().unwrap_or_default();
        let host = host.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            let res = host.handle(client, method, params).await;
            let line = match res {
                Ok(v) => json!({"id": id, "ok": v}),
                Err(e) => json!({"id": id, "error": e}),
            };
            let _ = tx.send(line.to_string());
        });
    }

    host.clients.lock().remove(&client);
    for c in host.convs.lock().values() {
        c.data.lock().subs.remove(&client);
    }
    writer_task.abort();
    tracing::info!(client, "client disconnected");
    host.sweep();
}

fn init_logging() {
    let path = super::log_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 5 << 20) {
        let _ = std::fs::rename(&path, path.with_extension("log.old"));
    }
    let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    else {
        return;
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_ansi(false)
        .with_writer(std::sync::Mutex::new(file))
        .try_init();
}

/// Entry point of `glaude --host`.
pub fn run() {
    init_logging();
    std::thread::spawn(|| {
        glaude_core::env::login_env();
    });
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("runtime: {e}");
            return;
        }
    };
    rt.block_on(async {
        // Another host already answering: nothing to do.
        if super::connect().await.is_ok() {
            tracing::info!("a host is already running");
            return;
        }
        let token = match super::new_token() {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("token: {e}");
                return;
            }
        };
        let mut listener = match Listener::bind() {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("bind: {e}");
                return;
            }
        };
        tracing::info!(pid = std::process::id(), "host started");
        let host = Host::new(token);

        // Flush batched events to attached clients.
        {
            let host = host.clone();
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(Duration::from_millis(FLUSH_MS));
                tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                loop {
                    tick.tick().await;
                    let clients = host.clients.lock().clone();
                    let convs: Vec<_> = host.convs.lock().values().cloned().collect();
                    for c in convs {
                        c.flush(&clients);
                    }
                }
            });
        }
        // Housekeeping: schedules, idle stop, exit when quiet.
        {
            let host = host.clone();
            tokio::spawn(async move {
                let secs = if quiet_exit() < Duration::from_secs(10) {
                    1
                } else {
                    10
                };
                let mut tick = tokio::time::interval(Duration::from_secs(secs));
                loop {
                    tick.tick().await;
                    host.tick_schedule();
                    host.sweep();
                    let quiet = *host.quiet_since.lock();
                    if quiet.is_some_and(|q| q.elapsed() >= quiet_exit()) {
                        host.exit.notify_one();
                    }
                }
            });
        }
        #[cfg(unix)]
        let mut term =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();

        loop {
            #[cfg(unix)]
            let term_recv = async {
                match term.as_mut() {
                    Some(t) => {
                        t.recv().await;
                    }
                    None => std::future::pending::<()>().await,
                }
            };
            #[cfg(not(unix))]
            let term_recv = std::future::pending::<()>();
            tokio::select! {
                accepted = listener.accept() => match accepted {
                    Ok((r, w)) => {
                        tokio::spawn(serve_client(host.clone(), r, w));
                    }
                    Err(e) => {
                        tracing::warn!("accept: {e}");
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                },
                _ = host.exit.notified() => break,
                _ = term_recv => break,
            }
        }
        tracing::info!("host exiting");
        drop(listener);
        host.shutdown_all();
    });
}

//! Runs one `claude -p --input-format stream-json` process per conversation.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use glaude_core::env;
use glaude_core::model::{Attachment, ConvEvent, SessionInfo, TurnState};
use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

use crate::transcript;
use crate::translate::Translator;

pub type Sink = Arc<dyn Fn(Vec<ConvEvent>) + Send + Sync>;

#[derive(Debug, Clone, Default)]
pub struct ClaudeSpec {
    pub cwd: String,
    /// Account home for `CLAUDE_CONFIG_DIR`; `None` for the default account.
    pub config_dir: Option<PathBuf>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub permission_mode: Option<String>,
    pub resume: Option<String>,
    pub fork: bool,
    /// When resuming, replay the transcript into the UI first.
    pub load_history: bool,
}

impl ClaudeSpec {
    /// Where this account keeps `projects/`.
    pub fn home(&self) -> PathBuf {
        self.config_dir
            .clone()
            .unwrap_or_else(|| env::home_dir().join(".claude"))
    }

    pub fn args(&self) -> Vec<String> {
        let mut a: Vec<String> = [
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--replay-user-messages",
            "--permission-prompt-tool",
            "stdio",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        if let Some(m) = self.permission_mode.as_deref().filter(|m| !m.is_empty()) {
            a.extend(["--permission-mode".into(), m.into()]);
        }
        if let Some(m) = self.model.as_deref().filter(|m| !m.is_empty()) {
            a.extend(["--model".into(), m.into()]);
        }
        if let Some(e) = self.effort.as_deref().filter(|e| !e.is_empty()) {
            a.extend(["--effort".into(), e.into()]);
        }
        if let Some(r) = self.resume.as_deref().filter(|r| !r.is_empty()) {
            a.extend(["--resume".into(), r.into()]);
            if self.fork {
                a.push("--fork-session".into());
            }
        }
        a
    }

    pub fn env(&self) -> env::EnvMap {
        // The keychain item name hashes the exact path: no trailing slash.
        let dir = self
            .config_dir
            .as_ref()
            .map(|d| d.display().to_string().trim_end_matches('/').to_string());
        env::agent_env(&[("CLAUDE_CONFIG_DIR", dir)])
    }
}

pub struct ClaudeSession {
    stdin: mpsc::UnboundedSender<String>,
    translator: Arc<Mutex<Translator>>,
    sink: Sink,
    kill: Mutex<Option<oneshot::Sender<()>>>,
    pub pid: Option<u32>,
}

fn line(v: &Value) -> String {
    serde_json::to_string(v).unwrap_or_default()
}

impl ClaudeSession {
    pub async fn spawn(spec: ClaudeSpec, sink: Sink) -> Result<Self> {
        let bin = env::claude_binary()
            .ok_or_else(|| anyhow!("Claude Code CLI not found. Install it or set CLAUDE_BIN."))?;
        if !std::path::Path::new(&spec.cwd).is_dir() {
            return Err(anyhow!("Folder not found: {}", spec.cwd));
        }

        // Show history first when resuming.
        if let Some(sid) = spec.resume.clone().filter(|_| spec.load_history) {
            let home = spec.home();
            let cwd = spec.cwd.clone();
            let items = tokio::task::spawn_blocking(move || {
                transcript::find_transcript(&home, &cwd, &sid)
                    .map(|p| transcript::load_items(&p, &cwd))
                    .transpose()
            })
            .await??
            .unwrap_or_default();
            sink(vec![ConvEvent::Reset { items }]);
        }

        let mut cmd = Command::new(&bin);
        cmd.args(spec.args())
            .current_dir(&spec.cwd)
            .env_clear()
            .envs(spec.env())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        cmd.process_group(0);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let mut child = cmd
            .spawn()
            .with_context(|| format!("starting {}", bin.display()))?;
        let pid = child.id();
        let mut stdin = child.stdin.take().expect("stdin");
        let stdout = child.stdout.take().expect("stdout");
        let mut stderr = child.stderr.take().expect("stderr");

        let translator = Arc::new(Mutex::new(Translator::new(&spec.cwd)));
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        // Writer: one JSON object per line, never blocks the UI thread.
        tokio::spawn(async move {
            while let Some(mut l) = rx.recv().await {
                l.push('\n');
                if stdin.write_all(l.as_bytes()).await.is_err() || stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        // stderr: keep the tail for error reports.
        let err_tail = Arc::new(Mutex::new(String::new()));
        {
            let err_tail = err_tail.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                while let Ok(n) = stderr.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    let mut t = err_tail.lock();
                    t.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if t.len() > 8192 {
                        let cut = t.len() - 4096;
                        let cut = (cut..t.len())
                            .find(|i| t.is_char_boundary(*i))
                            .unwrap_or(t.len());
                        t.drain(..cut);
                    }
                }
            });
        }

        // stdout: parse, translate, forward.
        let (kill_tx, kill_rx) = oneshot::channel::<()>();
        {
            let translator = translator.clone();
            let sink = sink.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::with_capacity(256 * 1024, stdout).lines();
                while let Ok(Some(l)) = lines.next_line().await {
                    if l.is_empty() {
                        continue;
                    }
                    let Ok(v) = serde_json::from_str::<Value>(&l) else {
                        tracing::debug!(line = %l.chars().take(200).collect::<String>(), "non-JSON stdout line");
                        continue;
                    };
                    let out = translator.lock().feed(&v);
                    for r in &out.requests {
                        let _ = tx.send(line(r));
                    }
                    if !out.events.is_empty() {
                        sink(out.events);
                    }
                }
            });
        }

        // Wait for exit or a kill request.
        {
            let sink = sink.clone();
            tokio::spawn(async move {
                let status = tokio::select! {
                    s = child.wait() => s.ok(),
                    _ = kill_rx => {
                        #[cfg(unix)]
                        if let Some(pid) = child.id() {
                            // SAFETY: signalling our own child's process group.
                            unsafe { libc::kill(-(pid as i32), libc::SIGTERM) };
                            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                            unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
                        }
                        let _ = child.start_kill();
                        child.wait().await.ok()
                    }
                };
                let code = status.and_then(|s| s.code());
                let mut events = Vec::new();
                let tail = err_tail.lock().trim().to_string();
                if code.is_some_and(|c| c != 0) && !tail.is_empty() {
                    let last: Vec<&str> = tail.lines().rev().take(6).collect();
                    let text = last.into_iter().rev().collect::<Vec<_>>().join("\n");
                    events.push(ConvEvent::Notice {
                        level: "error".into(),
                        text,
                    });
                }
                events.push(ConvEvent::Exited { code });
                sink(events);
            });
        }

        let session = Self {
            stdin: tx,
            translator,
            sink,
            kill: Mutex::new(Some(kill_tx)),
            pid,
        };
        let init = session
            .translator
            .lock()
            .control("init", json!({"subtype": "initialize"}));
        session.write(&init);
        Ok(session)
    }

    fn write(&self, v: &Value) {
        let _ = self.stdin.send(line(v));
    }

    pub fn send(&self, text: &str, images: Vec<(String, String)>, attachments: Vec<Attachment>) {
        let (msg, out) = self
            .translator
            .lock()
            .user_message(text, images, attachments);
        (self.sink)(out.events);
        self.write(&msg);
    }

    pub fn interrupt(&self) {
        let req = self
            .translator
            .lock()
            .control("int", json!({"subtype": "interrupt"}));
        self.write(&req);
    }

    pub fn answer(&self, request_id: &str, option: &str, message: Option<String>) {
        let (line, out) = self
            .translator
            .lock()
            .answer_permission(request_id, option, message);
        if let Some(l) = line {
            self.write(&l);
        }
        (self.sink)(out.events);
    }

    pub fn set_mode(&self, mode: &str) {
        let req = self.translator.lock().control(
            "mode",
            json!({"subtype": "set_permission_mode", "mode": mode}),
        );
        self.write(&req);
        (self.sink)(vec![ConvEvent::Session {
            info: SessionInfo {
                permission_mode: Some(mode.to_string()),
                ..Default::default()
            },
        }]);
    }

    pub fn set_model(&self, model: &str) {
        let req = self
            .translator
            .lock()
            .control("model", json!({"subtype": "set_model", "model": model}));
        self.write(&req);
        (self.sink)(vec![ConvEvent::Session {
            info: SessionInfo {
                model: Some(model.to_string()),
                ..Default::default()
            },
        }]);
    }

    pub fn close(&self) {
        if let Some(k) = self.kill.lock().take() {
            let _ = k.send(());
        }
        (self.sink)(vec![ConvEvent::Turn {
            state: TurnState::Idle,
            detail: None,
        }]);
    }
}

impl Drop for ClaudeSession {
    fn drop(&mut self) {
        if let Some(k) = self.kill.lock().take() {
            let _ = k.send(());
        }
    }
}

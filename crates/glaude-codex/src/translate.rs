//! Translate Codex app-server notifications for one thread into
//! provider-neutral events. Pure; history turns go through the same code.

use std::collections::{HashMap, HashSet};

use glaude_core::diff;
use glaude_core::model::{
    Attachment, ConvEvent, FileDiff, Item, LimitWindow, Limits, PermissionOption, PlanStep,
    SessionInfo, Status, TurnState,
};
use glaude_core::render::{self, StreamRenderer};
use serde_json::{json, Value};

const MAX_OUTPUT: usize = 64 * 1024;

fn s<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

fn u(v: &Value, k: &str) -> Option<u64> {
    v.get(k).and_then(Value::as_u64)
}

/// Approval policy, sandbox mode (thread/start) and sandbox policy
/// (turn/start) for a UI permission mode.
pub fn mode_policy(mode: &str) -> (&'static str, &'static str, Value) {
    let ww = json!({"type": "workspaceWrite", "writableRoots": [], "networkAccess": false, "excludeTmpdirEnvVar": false, "excludeSlashTmp": false});
    match mode {
        "full" => (
            "never",
            "danger-full-access",
            json!({"type": "dangerFullAccess"}),
        ),
        "untrusted" => ("untrusted", "workspace-write", ww),
        "read-only" => (
            "on-request",
            "read-only",
            json!({"type": "readOnly", "networkAccess": false}),
        ),
        _ => ("on-request", "workspace-write", ww),
    }
}

/// Best guess of the UI mode from a thread's approval policy and sandbox.
pub fn mode_from(approval: &Value, sandbox: &Value) -> String {
    let a = approval.as_str().unwrap_or("on-request");
    let sb = s(sandbox, "type").or(sandbox.as_str()).unwrap_or("");
    match (a, sb) {
        (_, "dangerFullAccess" | "danger-full-access") => "full",
        (_, "readOnly" | "read-only") => "read-only",
        ("untrusted", _) => "untrusted",
        _ => "on-request",
    }
    .to_string()
}

fn rfc3339(epoch_s: u64) -> Option<String> {
    chrono::DateTime::from_timestamp(epoch_s as i64, 0).map(|d| d.to_rfc3339())
}

/// Rate limit snapshot into Limits: windows of a day or more are weekly.
pub fn limits_from(snapshot: &Value) -> Limits {
    let mut l = Limits {
        plan: s(snapshot, "planType").map(str::to_string),
        ..Default::default()
    };
    for key in ["primary", "secondary"] {
        let Some(w) = snapshot.get(key).filter(|w| w.is_object()) else {
            continue;
        };
        let Some(used) = w.get("usedPercent").and_then(Value::as_f64) else {
            continue;
        };
        let win = LimitWindow {
            used: used.clamp(0.0, 100.0),
            resets_at: u(w, "resetsAt").and_then(rfc3339),
        };
        if u(w, "windowDurationMins").unwrap_or(0) >= 1440 {
            l.week = Some(win);
        } else {
            l.five_hour = Some(win);
        }
    }
    l
}

fn rel(path: &str, cwd: &str) -> String {
    let cwd = cwd.trim_end_matches('/');
    path.strip_prefix(cwd)
        .and_then(|r| r.strip_prefix('/'))
        .unwrap_or(path)
        .to_string()
}

/// `/bin/zsh -lc "cmd"` → `cmd`.
pub fn display_command(item: &Value) -> String {
    if let Some(actions) = item.get("commandActions").and_then(Value::as_array) {
        if actions.len() == 1 {
            if let Some(c) = s(&actions[0], "command") {
                return c.to_string();
            }
        }
    }
    let cmd = s(item, "command").unwrap_or("");
    for prefix in [
        "/bin/zsh -lc ",
        "/bin/bash -lc ",
        "bash -lc ",
        "zsh -lc ",
        "/bin/sh -c ",
        "sh -c ",
    ] {
        if let Some(rest) = cmd.strip_prefix(prefix) {
            let r = rest.trim();
            if r.len() >= 2
                && (r.starts_with('"') && r.ends_with('"')
                    || r.starts_with('\'') && r.ends_with('\''))
            {
                return r[1..r.len() - 1].to_string();
            }
            return r.to_string();
        }
    }
    cmd.to_string()
}

fn status_of(v: &Value) -> Status {
    match s(v, "status").unwrap_or("") {
        "inProgress" => Status::Running,
        "completed" => Status::Done,
        "failed" => Status::Error,
        "declined" => Status::Declined,
        "interrupted" => Status::Interrupted,
        _ => Status::Running,
    }
}

fn file_diffs(item: &Value, cwd: &str) -> Vec<FileDiff> {
    item.get("changes")
        .and_then(Value::as_array)
        .map(|changes| {
            changes
                .iter()
                .map(|c| {
                    let path = rel(s(c, "path").unwrap_or(""), cwd);
                    let kind = c
                        .get("kind")
                        .and_then(|k| s(k, "type"))
                        .unwrap_or("update")
                        .to_string();
                    let raw = s(c, "diff").unwrap_or("");
                    let is_unified = raw.lines().any(|l| l.starts_with("@@"));
                    let text = if is_unified {
                        raw.to_string()
                    } else if kind == "delete" {
                        diff::unified(&path, raw, "").diff
                    } else {
                        diff::unified(&path, "", raw).diff
                    };
                    let (added, removed) = diff::count(&text);
                    FileDiff {
                        path,
                        kind,
                        diff: text,
                        added,
                        removed,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn truncate(mut t: String) -> String {
    if t.len() > MAX_OUTPUT {
        let mut cut = t.len() - MAX_OUTPUT;
        while !t.is_char_boundary(cut) {
            cut += 1;
        }
        t = format!("… (earlier output truncated)\n{}", &t[cut..]);
    }
    t
}

struct Perm {
    rpc_id: Value,
    item: Item,
}

pub struct Translator {
    cwd: String,
    history: bool,
    pub turn_id: Option<String>,
    streams: HashMap<String, (bool, StreamRenderer)>,
    file_changes: HashMap<String, Vec<FileDiff>>,
    commands: HashMap<String, Item>,
    perms: HashMap<String, Perm>,
    client_ids: HashSet<String>,
    seq: u64,
    turn: TurnState,
}

impl Translator {
    pub fn new(cwd: &str) -> Self {
        Self {
            cwd: cwd.to_string(),
            history: false,
            turn_id: None,
            streams: HashMap::new(),
            file_changes: HashMap::new(),
            commands: HashMap::new(),
            perms: HashMap::new(),
            client_ids: HashSet::new(),
            seq: 0,
            turn: TurnState::Idle,
        }
    }

    pub fn for_history(cwd: &str) -> Self {
        let mut t = Self::new(cwd);
        t.history = true;
        t
    }

    fn next_id(&mut self, p: &str) -> String {
        self.seq += 1;
        format!("{p}:{}", self.seq)
    }

    fn set_turn(&mut self, out: &mut Vec<ConvEvent>, state: TurnState, detail: Option<String>) {
        if self.history {
            return;
        }
        if self.turn != state || detail.is_some() {
            self.turn = state;
            out.push(ConvEvent::Turn { state, detail });
        }
    }

    /// Optimistic user item for a message we send; its client id makes the
    /// server's echo a no-op.
    pub fn user_message(
        &mut self,
        text: &str,
        attachments: Vec<Attachment>,
    ) -> (String, Vec<ConvEvent>) {
        let client_id = uuid::Uuid::new_v4().to_string();
        self.client_ids.insert(client_id.clone());
        let mut out = vec![ConvEvent::Item {
            item: Item::User {
                id: client_id.clone(),
                blocks: render::render_plain(text),
                attachments,
                ts: Some(chrono::Utc::now().to_rfc3339()),
            },
        }];
        self.set_turn(&mut out, TurnState::Requesting, None);
        (client_id, out)
    }

    /// One thread item, started (`completed == false`) or completed.
    pub fn item(&mut self, item: &Value, completed: bool) -> Vec<ConvEvent> {
        let mut out = Vec::new();
        let id = s(item, "id").unwrap_or("").to_string();
        match s(item, "type").unwrap_or("") {
            "userMessage" => {
                if s(item, "clientId").is_some_and(|c| self.client_ids.contains(c))
                    || self.client_ids.contains(&id)
                {
                    return out;
                }
                if !completed && !self.history {
                    return out;
                }
                let text = item
                    .get("content")
                    .and_then(Value::as_array)
                    .map(|parts| {
                        parts
                            .iter()
                            .filter_map(|p| match s(p, "type") {
                                Some("text") => s(p, "text").map(str::to_string),
                                Some("localImage") | Some("image") => Some("[image]".into()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n\n")
                    })
                    .unwrap_or_default();
                if text.trim().is_empty() {
                    return out;
                }
                out.push(ConvEvent::Item {
                    item: Item::User {
                        id,
                        blocks: render::render_plain(&text),
                        attachments: Vec::new(),
                        ts: None,
                    },
                });
            }
            "agentMessage" | "plan" => {
                let text = s(item, "text").unwrap_or("").to_string();
                if completed {
                    self.streams.remove(&id);
                    if text.trim().is_empty() {
                        return out;
                    }
                    out.push(ConvEvent::Item {
                        item: Item::Assistant {
                            id,
                            blocks: render::render(&text),
                            parent: None,
                            streaming: false,
                        },
                    });
                } else {
                    self.streams
                        .insert(id.clone(), (true, StreamRenderer::new()));
                    out.push(ConvEvent::Item {
                        item: Item::Assistant {
                            id,
                            blocks: Vec::new(),
                            parent: None,
                            streaming: true,
                        },
                    });
                    self.set_turn(&mut out, TurnState::Writing, None);
                }
            }
            "reasoning" => {
                let summary: Vec<String> = item
                    .get("summary")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();
                if completed {
                    self.streams.remove(&id);
                    let text = summary.join("\n\n");
                    out.push(ConvEvent::Item {
                        item: Item::Reasoning {
                            id,
                            blocks: render::render(&text),
                            streaming: false,
                        },
                    });
                } else {
                    self.streams
                        .insert(id.clone(), (false, StreamRenderer::new()));
                    out.push(ConvEvent::Item {
                        item: Item::Reasoning {
                            id,
                            blocks: Vec::new(),
                            streaming: true,
                        },
                    });
                    self.set_turn(&mut out, TurnState::Thinking, None);
                }
            }
            "commandExecution" => {
                let command = display_command(item);
                let prev_output = match self.commands.get(&id) {
                    Some(Item::Command { output, .. }) => output.clone(),
                    _ => String::new(),
                };
                let output = s(item, "aggregatedOutput")
                    .map(|o| truncate(o.to_string()))
                    .unwrap_or(prev_output);
                let cmd = Item::Command {
                    id: id.clone(),
                    command: command.clone(),
                    cwd: s(item, "cwd").unwrap_or("").to_string(),
                    status: if completed {
                        status_of(item)
                    } else {
                        Status::Running
                    },
                    output,
                    exit_code: item
                        .get("exitCode")
                        .and_then(Value::as_i64)
                        .map(|c| c as i32),
                    duration_ms: u(item, "durationMs"),
                };
                self.commands.insert(id, cmd.clone());
                out.push(ConvEvent::Item { item: cmd });
                if !completed {
                    self.set_turn(&mut out, TurnState::Tool, Some(command));
                }
            }
            "fileChange" => {
                let diffs = file_diffs(item, &self.cwd);
                self.file_changes.insert(id.clone(), diffs.clone());
                out.push(ConvEvent::Item {
                    item: Item::FileChange {
                        id,
                        status: if completed {
                            status_of(item)
                        } else {
                            Status::Running
                        },
                        diffs,
                    },
                });
            }
            "mcpToolCall" | "dynamicToolCall" => {
                let server = s(item, "server").or(s(item, "namespace")).unwrap_or("");
                let tool = s(item, "tool").unwrap_or("tool");
                let name = if server.is_empty() {
                    tool.to_string()
                } else {
                    format!("mcp__{server}__{tool}")
                };
                let input = item.get("arguments").cloned().unwrap_or(Value::Null);
                let output = item
                    .get("result")
                    .and_then(|r| r.get("content"))
                    .or_else(|| item.get("contentItems"))
                    .and_then(Value::as_array)
                    .map(|parts| {
                        parts
                            .iter()
                            .filter_map(|p| s(p, "text"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .or_else(|| {
                        item.get("error")
                            .and_then(|e| s(e, "message").map(str::to_string))
                    });
                let title = input
                    .as_object()
                    .and_then(|o| o.values().find_map(Value::as_str))
                    .map(|t| t.chars().take(120).collect())
                    .unwrap_or_default();
                out.push(ConvEvent::Item {
                    item: Item::Tool {
                        id,
                        name,
                        title,
                        input,
                        status: if completed {
                            status_of(item)
                        } else {
                            Status::Running
                        },
                        output: output.map(truncate),
                        diffs: Vec::new(),
                        parent: None,
                    },
                });
            }
            "webSearch" => {
                let q = s(item, "query").unwrap_or("").to_string();
                out.push(ConvEvent::Item {
                    item: Item::Tool {
                        id,
                        name: "WebSearch".into(),
                        title: q.clone(),
                        input: json!({"query": q}),
                        status: if completed {
                            Status::Done
                        } else {
                            Status::Running
                        },
                        output: None,
                        diffs: Vec::new(),
                        parent: None,
                    },
                });
            }
            "collabAgentToolCall" | "subAgentActivity" => {
                let title = s(item, "prompt")
                    .or(s(item, "description"))
                    .unwrap_or("Subagent")
                    .chars()
                    .take(120)
                    .collect();
                out.push(ConvEvent::Item {
                    item: Item::Tool {
                        id,
                        name: "Agent".into(),
                        title,
                        input: Value::Null,
                        status: if completed {
                            Status::Done
                        } else {
                            Status::Running
                        },
                        output: None,
                        diffs: Vec::new(),
                        parent: None,
                    },
                });
            }
            "contextCompaction" if completed => {
                out.push(ConvEvent::Item {
                    item: Item::System {
                        id,
                        level: "compact".into(),
                        text: "Context compacted".into(),
                    },
                });
            }
            "imageView" if completed => {
                let path = rel(s(item, "path").unwrap_or(""), &self.cwd);
                out.push(ConvEvent::Item {
                    item: Item::System {
                        id,
                        level: "info".into(),
                        text: format!("Viewed image {path}"),
                    },
                });
            }
            "enteredReviewMode" | "exitedReviewMode" if completed => {
                let text = if s(item, "type") == Some("enteredReviewMode") {
                    "Review started"
                } else {
                    "Review finished"
                };
                out.push(ConvEvent::Item {
                    item: Item::System {
                        id,
                        level: "info".into(),
                        text: text.into(),
                    },
                });
            }
            _ => {}
        }
        out
    }

    /// A notification already routed to this thread.
    pub fn notification(&mut self, method: &str, p: &Value) -> Vec<ConvEvent> {
        let mut out = Vec::new();
        match method {
            "turn/started" => {
                self.turn_id = p.get("turn").and_then(|t| s(t, "id")).map(str::to_string);
                self.set_turn(&mut out, TurnState::Thinking, None);
            }
            "item/started" => out = self.item(p.get("item").unwrap_or(&Value::Null), false),
            "item/completed" => out = self.item(p.get("item").unwrap_or(&Value::Null), true),
            "item/agentMessage/delta"
            | "item/plan/delta"
            | "item/reasoning/summaryTextDelta"
            | "item/reasoning/textDelta" => {
                let id = s(p, "itemId").unwrap_or("").to_string();
                let delta = s(p, "delta").unwrap_or("");
                if let Some((is_text, r)) = self.streams.get_mut(&id) {
                    let is_text = *is_text;
                    let (from, blocks) = r.push(delta);
                    out.push(ConvEvent::Blocks { id, from, blocks });
                    if is_text {
                        self.set_turn(&mut out, TurnState::Writing, None);
                    }
                }
            }
            "item/commandExecution/outputDelta" => {
                let id = s(p, "itemId").unwrap_or("").to_string();
                let chunk = s(p, "delta").unwrap_or("").to_string();
                if let Some(Item::Command { output, .. }) = self.commands.get_mut(&id) {
                    output.push_str(&chunk);
                }
                out.push(ConvEvent::Output { id, chunk });
            }
            "turn/plan/updated" => {
                let steps: Vec<PlanStep> = p
                    .get("plan")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .map(|st| PlanStep {
                                text: s(st, "step").unwrap_or("").to_string(),
                                status: s(st, "status").unwrap_or("pending").to_string(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let turn = s(p, "turnId").unwrap_or("turn");
                out.push(ConvEvent::Item {
                    item: Item::Plan {
                        id: format!("plan:{turn}"),
                        steps,
                        explanation: s(p, "explanation").map(str::to_string),
                    },
                });
            }
            "thread/tokenUsage/updated" => {
                if let Some(tu) = p.get("tokenUsage") {
                    let last = tu.get("last").and_then(|l| u(l, "totalTokens"));
                    out.push(ConvEvent::Session {
                        info: SessionInfo {
                            context_used: last,
                            context_window: u(tu, "modelContextWindow"),
                            ..Default::default()
                        },
                    });
                }
            }
            "account/rateLimits/updated" => {
                if let Some(r) = p.get("rateLimits") {
                    out.push(ConvEvent::Limits {
                        limits: limits_from(r),
                    });
                }
            }
            "thread/status/changed" => {
                let flags = p
                    .get("status")
                    .and_then(|st| st.get("activeFlags"))
                    .and_then(Value::as_array);
                if flags.is_some_and(|f| f.iter().any(|x| x.as_str() == Some("waitingOnApproval")))
                {
                    self.set_turn(&mut out, TurnState::WaitingPermission, None);
                }
            }
            "serverRequest/resolved" => {
                let key = p.get("requestId").map(Value::to_string).unwrap_or_default();
                if let Some(perm) = self.perms.remove(&key) {
                    out.push(ConvEvent::Item {
                        item: with_status(perm.item, Status::Interrupted),
                    });
                }
            }
            "error" => {
                let msg = p
                    .get("error")
                    .and_then(|e| s(e, "message"))
                    .unwrap_or("Codex error")
                    .to_string();
                if p.get("willRetry").and_then(Value::as_bool).unwrap_or(false) {
                    self.set_turn(&mut out, TurnState::Retrying, Some(msg));
                } else {
                    let id = self.next_id("sys");
                    out.push(ConvEvent::Item {
                        item: Item::System {
                            id,
                            level: "error".into(),
                            text: msg,
                        },
                    });
                }
            }
            "thread/compacted" => {
                let id = self.next_id("sys");
                out.push(ConvEvent::Item {
                    item: Item::System {
                        id,
                        level: "compact".into(),
                        text: "Context compacted".into(),
                    },
                });
            }
            "model/rerouted" => {
                if let Some(m) = s(p, "toModel").or(s(p, "model")) {
                    out.push(ConvEvent::Session {
                        info: SessionInfo {
                            model: Some(m.to_string()),
                            ..Default::default()
                        },
                    });
                }
            }
            "turn/completed" => {
                let turn = p.get("turn").cloned().unwrap_or(Value::Null);
                // Close anything left open.
                let open: Vec<String> = self.streams.keys().cloned().collect();
                for id in open {
                    if let Some((is_text, r)) = self.streams.remove(&id) {
                        let blocks = r.finish();
                        let item = if is_text {
                            Item::Assistant {
                                id,
                                blocks,
                                parent: None,
                                streaming: false,
                            }
                        } else {
                            Item::Reasoning {
                                id,
                                blocks,
                                streaming: false,
                            }
                        };
                        out.push(ConvEvent::Item { item });
                    }
                }
                let stale: Vec<String> = self.perms.keys().cloned().collect();
                for k in stale {
                    if let Some(perm) = self.perms.remove(&k) {
                        out.push(ConvEvent::Item {
                            item: with_status(perm.item, Status::Interrupted),
                        });
                    }
                }
                let error = match s(&turn, "status") {
                    Some("failed") => Some(
                        turn.get("error")
                            .and_then(|e| s(e, "message"))
                            .unwrap_or("The turn failed")
                            .to_string(),
                    ),
                    _ => None,
                };
                self.turn = TurnState::Idle;
                self.turn_id = None;
                if !self.history {
                    out.push(ConvEvent::TurnDone {
                        duration_ms: u(&turn, "durationMs"),
                        error,
                    });
                }
            }
            _ => {}
        }
        out
    }

    /// A server request (approval). Returns the events; the request stays
    /// pending until `answer`.
    pub fn server_request(
        &mut self,
        rpc_id: &Value,
        method: &str,
        p: &Value,
    ) -> Option<Vec<ConvEvent>> {
        let key = rpc_id.to_string();
        let options = |always: &str| {
            vec![
                PermissionOption {
                    id: "allow".into(),
                    label: "Allow".into(),
                    kind: "allow".into(),
                },
                PermissionOption {
                    id: "allow-always".into(),
                    label: always.into(),
                    kind: "allow".into(),
                },
                PermissionOption {
                    id: "deny".into(),
                    label: "Deny".into(),
                    kind: "deny".into(),
                },
            ]
        };
        let item = match method {
            "item/commandExecution/requestApproval" => {
                let command = display_command(p);
                Item::Permission {
                    id: format!("perm:{key}"),
                    request_id: key.clone(),
                    tool: "Shell".into(),
                    title: command.clone(),
                    input: json!({"command": command}),
                    diffs: Vec::new(),
                    options: options("Allow for this session"),
                    status: Status::Pending,
                    reason: s(p, "reason").map(str::to_string),
                    blocks: Vec::new(),
                    tool_use_id: String::new(),
                }
            }
            "item/fileChange/requestApproval" => {
                let item_id = s(p, "itemId").unwrap_or("");
                let diffs = self.file_changes.get(item_id).cloned().unwrap_or_default();
                let title = diffs
                    .iter()
                    .map(|d| d.path.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                Item::Permission {
                    id: format!("perm:{key}"),
                    request_id: key.clone(),
                    tool: "Edit".into(),
                    title,
                    input: Value::Null,
                    diffs,
                    options: options("Allow edits for this session"),
                    status: Status::Pending,
                    reason: s(p, "reason").map(str::to_string),
                    blocks: Vec::new(),
                    tool_use_id: String::new(),
                }
            }
            _ => return None,
        };
        let mut out = vec![ConvEvent::Item { item: item.clone() }];
        self.perms.insert(
            key,
            Perm {
                rpc_id: rpc_id.clone(),
                item,
            },
        );
        self.set_turn(&mut out, TurnState::WaitingPermission, None);
        Some(out)
    }

    /// Answer a pending approval: returns the JSON-RPC response line.
    pub fn answer(&mut self, key: &str, option: &str) -> (Option<Value>, Vec<ConvEvent>) {
        let Some(perm) = self.perms.remove(key) else {
            return (None, Vec::new());
        };
        let decision = match option {
            "allow" => "accept",
            "allow-always" => "acceptForSession",
            _ => "decline",
        };
        let line = json!({"id": perm.rpc_id, "result": {"decision": decision}});
        let status = if decision == "decline" {
            Status::Declined
        } else {
            Status::Done
        };
        let mut out = vec![ConvEvent::Item {
            item: with_status(perm.item, status),
        }];
        let next = if self.perms.is_empty() {
            TurnState::Tool
        } else {
            TurnState::WaitingPermission
        };
        self.set_turn(&mut out, next, None);
        (Some(line), out)
    }
}

fn with_status(item: Item, st: Status) -> Item {
    match item {
        Item::Permission {
            id,
            request_id,
            tool,
            title,
            input,
            diffs,
            options,
            reason,
            blocks,
            tool_use_id,
            ..
        } => Item::Permission {
            id,
            request_id,
            tool,
            title,
            input,
            diffs,
            options,
            status: st,
            reason,
            blocks,
            tool_use_id,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replays_command_approval_fixture() {
        let path = format!(
            "{}/../../fixtures/codex/command-approval.ndjson",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read_to_string(path).unwrap();
        let mut t = Translator::new("/tmp/fixproj");
        let mut events = Vec::new();
        let mut answered = false;
        for line in text.lines() {
            let v: Value = serde_json::from_str(line).unwrap();
            let Some(method) = s(&v, "method") else {
                continue;
            };
            let params = v.get("params").cloned().unwrap_or(Value::Null);
            if let Some(id) = v.get("id") {
                let evs = t
                    .server_request(id, method, &params)
                    .expect("approval request");
                let key = id.to_string();
                assert!(evs.iter().any(|e| matches!(e, ConvEvent::Item { item: Item::Permission { title, .. } } if title.contains("cat note.txt"))));
                events.extend(evs);
                let (line, evs) = t.answer(&key, "allow");
                assert_eq!(line.unwrap()["result"]["decision"], "accept");
                events.extend(evs);
                answered = true;
                continue;
            }
            events.extend(t.notification(method, &params));
        }
        assert!(answered);
        let cmd = events.iter().rev().find_map(|e| match e {
            ConvEvent::Item {
                item:
                    Item::Command {
                        command,
                        status,
                        output,
                        exit_code,
                        ..
                    },
            } => Some((command.clone(), *status, output.clone(), *exit_code)),
            _ => None,
        });
        let (command, status, output, code) = cmd.expect("command item");
        assert_eq!(command, "printf 'سلام' > out.txt && cat note.txt");
        assert_eq!(status, Status::Done);
        assert_eq!(output.trim(), "hello fixture");
        assert_eq!(code, Some(0));
        let answer = events.iter().rev().find_map(|e| match e {
            ConvEvent::Item {
                item:
                    Item::Assistant {
                        blocks,
                        streaming: false,
                        ..
                    },
            } => Some(blocks.clone()),
            _ => None,
        });
        assert_eq!(answer.unwrap()[0].dir, glaude_core::bidi::Dir::Rtl);
        assert!(
            events.iter().any(|e| matches!(e, ConvEvent::Blocks { .. })),
            "streamed"
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, ConvEvent::Limits { limits } if limits.week.is_some())));
        assert!(events
            .iter()
            .any(|e| matches!(e, ConvEvent::TurnDone { error: None, .. })));
    }

    #[test]
    fn strips_shell_wrapper() {
        assert_eq!(
            display_command(&json!({"command": "/bin/zsh -lc \"ls -la\""})),
            "ls -la"
        );
        assert_eq!(
            display_command(&json!({"command": "git status"})),
            "git status"
        );
    }

    #[test]
    fn new_file_change_becomes_unified_diff() {
        let mut t = Translator::new("/p");
        let ev = t.item(&json!({"type": "fileChange", "id": "f1", "status": "completed", "changes": [{"path": "/p/a.txt", "kind": {"type": "add"}, "diff": "hello\nworld\n"}]}), true);
        match &ev[0] {
            ConvEvent::Item {
                item: Item::FileChange { diffs, .. },
            } => {
                assert_eq!(diffs[0].path, "a.txt");
                assert_eq!(diffs[0].added, 2);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn mode_mapping_round_trips() {
        for m in ["full", "untrusted", "read-only", "on-request"] {
            let (a, sb, _) = mode_policy(m);
            assert_eq!(mode_from(&json!(a), &json!(sb)), m);
        }
    }
}

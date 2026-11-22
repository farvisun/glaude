//! Translate Claude Code stream-json lines into provider-neutral events.
//!
//! The translator is pure: it takes parsed JSON lines and returns UI
//! events plus control requests the driver should send. Transcript lines
//! from disk go through the same path, so history and live sessions render
//! identically.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use glaude_core::model::{
    Block, ConvEvent, FileDiff, Item, LimitWindow, Limits, ModelChoice, PermissionOption,
    SessionInfo, Status, TurnState, TurnTokens,
};
use glaude_core::render::{self, StreamRenderer};
use serde_json::{json, Value};

use crate::tasks;
use crate::tools;

/// Tool output kept for display; the full output stays in the transcript.
const MAX_OUTPUT: usize = 64 * 1024;

#[derive(Default)]
pub struct Output {
    pub events: Vec<ConvEvent>,
    /// Control requests to write to the CLI's stdin.
    pub requests: Vec<Value>,
}

enum StreamKind {
    Text,
    Thinking,
    Tool {
        id: String,
        name: String,
        json: String,
    },
}

struct StreamBlock {
    kind: StreamKind,
    item_id: String,
    renderer: StreamRenderer,
    parent: Option<String>,
}

struct ToolState {
    name: String,
    title: String,
    input: Value,
    status: Status,
    output: Option<String>,
    diffs: Vec<FileDiff>,
    parent: Option<String>,
}

struct PermState {
    tool_use_id: String,
    /// Tool name (AskUserQuestion, ExitPlanMode, Bash, ...).
    name: String,
    tool: String,
    title: String,
    input: Value,
    diffs: Vec<FileDiff>,
    options: Vec<PermissionOption>,
    suggestions: Value,
    reason: Option<String>,
    blocks: Vec<glaude_core::model::Block>,
    answers: BTreeMap<String, String>,
}

/// One hook run, kept for the Hooks panel.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookRun {
    pub name: String,
    pub event: String,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i64>,
    pub output: String,
    pub at: u64,
}

const HOOK_RUNS: usize = 200;

pub struct Translator {
    cwd: String,
    history: bool,
    seq: u64,
    /// Open streamed content blocks keyed by (parent, message id, index).
    blocks: HashMap<(Option<String>, String, u64), StreamBlock>,
    /// Current message id per parent (subagents stream interleaved).
    current_msg: HashMap<Option<String>, String>,
    /// Streamed text/thinking items per message, in order, waiting for the
    /// authoritative `assistant` event.
    unconfirmed: HashMap<String, VecDeque<(bool, String)>>,
    /// Final assistant text per item id (for rendering final blocks once).
    finalized: HashSet<String>,
    tools: HashMap<String, ToolState>,
    perms: HashMap<String, PermState>,
    sent_uuids: HashSet<String>,
    turn: TurnState,
    req_seq: u64,
    model_window: Option<u64>,
    tasks: tasks::Tasks,
    /// Characters streamed this turn (text + tool input) and thinking.
    turn_chars: u64,
    thinking_chars: u64,
    thinking_est: u64,
    last_tokens: u64,
    hook_runs: VecDeque<HookRun>,
}

fn s<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

fn u(v: &Value, k: &str) -> u64 {
    v.get(k).and_then(Value::as_u64).unwrap_or(0)
}

fn truncate(mut text: String) -> String {
    if text.len() > MAX_OUTPUT {
        let mut cut = MAX_OUTPUT;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text.truncate(cut);
        text.push_str("\n… (output truncated)");
    }
    text
}

/// Epoch seconds/ms or RFC 3339 into RFC 3339.
fn to_rfc3339(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => {
            let mut x = n.as_f64()?;
            if x > 1e10 {
                x /= 1000.0;
            }
            chrono::DateTime::from_timestamp(x as i64, 0).map(|d| d.to_rfc3339())
        }
        _ => None,
    }
}

impl Translator {
    pub fn new(cwd: &str) -> Self {
        Self {
            cwd: cwd.to_string(),
            history: false,
            seq: 0,
            blocks: HashMap::new(),
            current_msg: HashMap::new(),
            unconfirmed: HashMap::new(),
            finalized: HashSet::new(),
            tools: HashMap::new(),
            perms: HashMap::new(),
            sent_uuids: HashSet::new(),
            turn: TurnState::Idle,
            req_seq: 0,
            model_window: None,
            tasks: tasks::Tasks::default(),
            turn_chars: 0,
            thinking_chars: 0,
            thinking_est: 0,
            last_tokens: 0,
            hook_runs: VecDeque::new(),
        }
    }

    /// A translator for replaying transcript lines (no turn state).
    pub fn for_history(cwd: &str) -> Self {
        let mut t = Self::new(cwd);
        t.history = true;
        t
    }

    fn next_id(&mut self, prefix: &str) -> String {
        self.seq += 1;
        format!("{prefix}:{}", self.seq)
    }

    pub fn next_request_id(&mut self, prefix: &str) -> String {
        self.req_seq += 1;
        format!("{prefix}-{}", self.req_seq)
    }

    pub fn control(&mut self, prefix: &str, request: Value) -> Value {
        let id = self.next_request_id(prefix);
        json!({"type": "control_request", "request_id": id, "request": request})
    }

    fn set_turn(&mut self, out: &mut Output, state: TurnState, detail: Option<String>) {
        if self.history {
            return;
        }
        if self.turn != state || detail.is_some() {
            if self.turn == TurnState::Idle && state != TurnState::Idle {
                self.turn_chars = 0;
                self.thinking_chars = 0;
                self.thinking_est = 0;
                self.last_tokens = 0;
            }
            self.turn = state;
            out.events.push(ConvEvent::Turn { state, detail });
        }
    }

    /// The UI sent a user message. Returns the stdin line and the item.
    pub fn user_message(
        &mut self,
        text: &str,
        images: Vec<(String, String)>,
        attachments: Vec<glaude_core::model::Attachment>,
    ) -> (Value, Output) {
        let uuid = uuid::Uuid::new_v4().to_string();
        self.sent_uuids.insert(uuid.clone());
        let mut content = Vec::new();
        if !text.is_empty() {
            content.push(json!({"type": "text", "text": text}));
        }
        for (mime, data) in images {
            content.push(json!({"type": "image", "source": {"type": "base64", "media_type": mime, "data": data}}));
        }
        let line = json!({
            "type": "user",
            "session_id": "",
            "parent_tool_use_id": null,
            "uuid": uuid,
            "message": {"role": "user", "content": content},
        });
        let mut out = Output::default();
        out.events.push(ConvEvent::Item {
            item: Item::User {
                id: uuid,
                blocks: render::render_plain(text),
                attachments,
                ts: Some(chrono::Utc::now().to_rfc3339()),
            },
        });
        self.set_turn(&mut out, TurnState::Requesting, None);
        (line, out)
    }

    /// Answer a permission prompt. Returns the control_response line.
    pub fn answer_permission(
        &mut self,
        request_id: &str,
        option: &str,
        message: Option<String>,
    ) -> (Option<Value>, Output) {
        let mut out = Output::default();
        let Some(p) = self.perms.remove(request_id) else {
            return (None, out);
        };
        let allow = option.starts_with("allow");
        let response = if allow {
            let mut r =
                json!({"behavior": "allow", "updatedInput": p.input, "toolUseID": p.tool_use_id});
            if option == "allow-always" {
                if let Some(arr) = p.suggestions.as_array().filter(|a| !a.is_empty()) {
                    r["updatedPermissions"] = Value::Array(arr.clone());
                }
            }
            // Approving a plan leaves plan mode for the chosen mode.
            if p.name == "ExitPlanMode" {
                let mode = if option == "allow-accept-edits" {
                    "acceptEdits"
                } else {
                    "default"
                };
                r["updatedPermissions"] =
                    json!([{"type": "setMode", "mode": mode, "destination": "session"}]);
                out.events.push(ConvEvent::Session {
                    info: SessionInfo {
                        permission_mode: Some(mode.into()),
                        ..Default::default()
                    },
                });
            }
            r
        } else {
            json!({
                "behavior": "deny",
                "message": message.unwrap_or_else(|| "The user declined this action.".into()),
                "interrupt": false,
                "toolUseID": p.tool_use_id,
            })
        };
        let line = json!({
            "type": "control_response",
            "response": {"subtype": "success", "request_id": request_id, "response": response},
        });
        out.events.push(ConvEvent::Item {
            item: self.perm_item(
                request_id,
                &p,
                if allow {
                    Status::Done
                } else {
                    Status::Declined
                },
            ),
        });
        if !allow {
            if let Some(t) = self.tools.get_mut(&p.tool_use_id) {
                t.status = Status::Declined;
            }
            if let Some(item) = self.tool_item(&p.tool_use_id) {
                out.events.push(ConvEvent::Item { item });
            }
        }
        let state = if self.perms.is_empty() {
            TurnState::Tool
        } else {
            TurnState::WaitingPermission
        };
        self.set_turn(&mut out, state, None);
        (Some(line), out)
    }

    /// Answer an AskUserQuestion prompt: question text -> chosen label(s).
    pub fn answer_question(
        &mut self,
        request_id: &str,
        answers: BTreeMap<String, String>,
    ) -> (Option<Value>, Output) {
        let mut out = Output::default();
        let Some(mut p) = self.perms.remove(request_id) else {
            return (None, out);
        };
        let mut input = p.input.clone();
        if let Value::Object(m) = &mut input {
            m.insert("answers".into(), json!(answers));
        }
        let line = json!({
            "type": "control_response",
            "response": {"subtype": "success", "request_id": request_id, "response": {
                "behavior": "allow", "updatedInput": input, "toolUseID": p.tool_use_id,
            }},
        });
        p.answers = answers;
        out.events.push(ConvEvent::Item {
            item: self.perm_item(request_id, &p, Status::Done),
        });
        let state = if self.perms.is_empty() {
            TurnState::Tool
        } else {
            TurnState::WaitingPermission
        };
        self.set_turn(&mut out, state, None);
        (Some(line), out)
    }

    /// Recent hook runs, newest last.
    pub fn hook_runs(&self) -> Vec<HookRun> {
        self.hook_runs.iter().cloned().collect()
    }

    fn perm_item(&self, request_id: &str, p: &PermState, status: Status) -> Item {
        if p.name == "AskUserQuestion" {
            return Item::Question {
                id: format!("perm:{request_id}"),
                request_id: request_id.to_string(),
                questions: tools::questions(&p.input),
                status,
                answers: p.answers.clone(),
                tool_use_id: p.tool_use_id.clone(),
            };
        }
        Item::Permission {
            id: format!("perm:{request_id}"),
            request_id: request_id.to_string(),
            tool: p.tool.clone(),
            title: p.title.clone(),
            input: p.input.clone(),
            diffs: p.diffs.clone(),
            options: p.options.clone(),
            status,
            reason: p.reason.clone(),
            blocks: p.blocks.clone(),
            tool_use_id: p.tool_use_id.clone(),
        }
    }

    fn live_notification(&mut self, text: &str, out: &mut Output) {
        let Some((tool_use_id, _, _, Some(result))) = tasks::parse_notification(text) else {
            return;
        };
        if let Some(task) = self.tasks.set_report(&tool_use_id, &result) {
            out.events.push(ConvEvent::Task { task });
        }
        if let Some(t) = self.tools.get_mut(&tool_use_id) {
            t.output = Some(truncate(result));
        }
        if let Some(item) = self.tool_item(&tool_use_id) {
            out.events.push(ConvEvent::Item { item });
        }
    }

    fn tool_item(&self, id: &str) -> Option<Item> {
        let t = self.tools.get(id)?;
        Some(Item::Tool {
            id: id.to_string(),
            name: t.name.clone(),
            title: t.title.clone(),
            input: t.input.clone(),
            status: t.status,
            output: t.output.clone(),
            diffs: t.diffs.clone(),
            parent: t.parent.clone(),
        })
    }

    fn upsert_tool(
        &mut self,
        out: &mut Output,
        id: &str,
        name: &str,
        input: Value,
        parent: Option<String>,
        status: Status,
    ) {
        let title = tools::title(name, &input, &self.cwd);
        let diffs = if matches!(name, "Edit" | "MultiEdit" | "Write") && !self.history {
            tools::diff_preview(name, &input, &self.cwd)
        } else if matches!(name, "Edit" | "MultiEdit") {
            // History: no file on disk to diff against reliably; use strings.
            tools::diff_preview(name, &input, "")
        } else {
            Vec::new()
        };
        let entry = self
            .tools
            .entry(id.to_string())
            .or_insert_with(|| ToolState {
                name: name.to_string(),
                title: String::new(),
                input: Value::Null,
                status,
                output: None,
                diffs: Vec::new(),
                parent: parent.clone(),
            });
        entry.name = name.to_string();
        entry.title = title;
        entry.input = input;
        if entry.diffs.is_empty() {
            entry.diffs = diffs;
        }
        if matches!(entry.status, Status::Pending) {
            entry.status = status;
        }
        if let Some(item) = self.tool_item(id) {
            out.events.push(ConvEvent::Item { item });
        }
    }

    /// Feed one parsed line from stdout (or a transcript).
    pub fn feed(&mut self, v: &Value) -> Output {
        let mut out = Output::default();
        match s(v, "type").unwrap_or("") {
            "stream_event" => self.on_stream(v, &mut out),
            "assistant" => self.on_assistant(v, &mut out),
            "user" => self.on_user(v, &mut out),
            "system" => self.on_system(v, &mut out),
            "result" => self.on_result(v, &mut out),
            "rate_limit_event" => self.on_rate_limit(v, &mut out),
            "control_request" => self.on_control_request(v, &mut out),
            "control_cancel_request" => self.on_cancel(v, &mut out),
            "control_response" => self.on_control_response(v, &mut out),
            "conversation_reset" => {
                if let Some(id) = s(v, "new_conversation_id") {
                    out.events.push(ConvEvent::Session {
                        info: SessionInfo {
                            session_id: Some(id.to_string()),
                            ..Default::default()
                        },
                    });
                }
                out.events.push(ConvEvent::Reset { items: Vec::new() });
            }
            _ => {}
        }
        out
    }

    fn parent_of(v: &Value) -> Option<String> {
        s(v, "parent_tool_use_id").map(str::to_string)
    }

    fn on_stream(&mut self, v: &Value, out: &mut Output) {
        let parent = Self::parent_of(v);
        let Some(ev) = v.get("event") else { return };
        let index = u(ev, "index");
        match s(ev, "type").unwrap_or("") {
            "message_start" => {
                if let Some(id) = ev.get("message").and_then(|m| s(m, "id")) {
                    self.current_msg.insert(parent.clone(), id.to_string());
                }
                if parent.is_none() {
                    self.set_turn(out, TurnState::Thinking, None);
                }
            }
            "content_block_start" => {
                let Some(msg) = self.current_msg.get(&parent).cloned() else {
                    return;
                };
                let Some(cb) = ev.get("content_block") else {
                    return;
                };
                let item_id = format!("{msg}:{index}");
                let kind = match s(cb, "type").unwrap_or("") {
                    "text" => {
                        out.events.push(ConvEvent::Item {
                            item: Item::Assistant {
                                id: item_id.clone(),
                                blocks: Vec::new(),
                                parent: parent.clone(),
                                streaming: true,
                            },
                        });
                        self.unconfirmed
                            .entry(msg.clone())
                            .or_default()
                            .push_back((true, item_id.clone()));
                        if parent.is_none() {
                            self.set_turn(out, TurnState::Writing, None);
                        }
                        StreamKind::Text
                    }
                    "thinking" => {
                        out.events.push(ConvEvent::Item {
                            item: Item::Reasoning {
                                id: item_id.clone(),
                                blocks: Vec::new(),
                                streaming: true,
                            },
                        });
                        self.unconfirmed
                            .entry(msg.clone())
                            .or_default()
                            .push_back((false, item_id.clone()));
                        if parent.is_none() {
                            self.set_turn(out, TurnState::Thinking, None);
                        }
                        StreamKind::Thinking
                    }
                    "tool_use" => {
                        let id = s(cb, "id").unwrap_or_default().to_string();
                        let name = s(cb, "name").unwrap_or_default().to_string();
                        self.upsert_tool(
                            out,
                            &id,
                            &name,
                            json!({}),
                            parent.clone(),
                            Status::Pending,
                        );
                        if parent.is_none() {
                            self.set_turn(out, TurnState::Tool, Some(name.clone()));
                        }
                        StreamKind::Tool {
                            id,
                            name,
                            json: String::new(),
                        }
                    }
                    _ => return,
                };
                self.blocks.insert(
                    (parent.clone(), msg, index),
                    StreamBlock {
                        kind,
                        item_id,
                        renderer: StreamRenderer::new(),
                        parent,
                    },
                );
            }
            "content_block_delta" => {
                let Some(msg) = self.current_msg.get(&parent).cloned() else {
                    return;
                };
                let Some(b) = self.blocks.get_mut(&(parent.clone(), msg, index)) else {
                    return;
                };
                let Some(d) = ev.get("delta") else { return };
                match (&mut b.kind, s(d, "type").unwrap_or("")) {
                    (StreamKind::Text, "text_delta") | (StreamKind::Thinking, "thinking_delta") => {
                        let text = s(d, "text").or(s(d, "thinking")).unwrap_or("");
                        if text.is_empty() {
                            return;
                        }
                        if b.parent.is_none() {
                            if matches!(b.kind, StreamKind::Thinking) {
                                self.thinking_chars += text.len() as u64;
                            } else {
                                self.turn_chars += text.len() as u64;
                            }
                        }
                        let (from, blocks) = b.renderer.push(text);
                        out.events.push(ConvEvent::Blocks {
                            id: b.item_id.clone(),
                            from,
                            blocks,
                        });
                    }
                    (StreamKind::Tool { json, .. }, "input_json_delta") => {
                        let part = s(d, "partial_json").unwrap_or("");
                        json.push_str(part);
                        if b.parent.is_none() {
                            self.turn_chars += part.len() as u64;
                        }
                    }
                    _ => {}
                }
                self.emit_tokens(out);
            }
            "content_block_stop" => {
                let Some(msg) = self.current_msg.get(&parent).cloned() else {
                    return;
                };
                let Some(b) = self.blocks.remove(&(parent.clone(), msg, index)) else {
                    return;
                };
                match b.kind {
                    StreamKind::Text | StreamKind::Thinking => {
                        if self.finalized.contains(&b.item_id) {
                            return;
                        }
                        let blocks = b.renderer.finish();
                        let item = if matches!(b.kind, StreamKind::Text) {
                            Item::Assistant {
                                id: b.item_id,
                                blocks,
                                parent: b.parent,
                                streaming: false,
                            }
                        } else {
                            Item::Reasoning {
                                id: b.item_id,
                                blocks,
                                streaming: false,
                            }
                        };
                        out.events.push(ConvEvent::Item { item });
                    }
                    StreamKind::Tool { id, name, json } => {
                        if let Ok(input) = serde_json::from_str::<Value>(&json) {
                            self.upsert_tool(out, &id, &name, input, b.parent, Status::Running);
                        }
                    }
                }
            }
            "message_delta" if parent.is_none() => {
                if let Some(usage) = ev.get("usage") {
                    self.context_from_usage(usage, out);
                }
            }
            _ => {}
        }
    }

    /// Tokens produced this turn, estimated like Claude Code's spinner.
    fn emit_tokens(&mut self, out: &mut Output) {
        if self.history || self.turn == TurnState::Idle {
            return;
        }
        let tokens = self.turn_chars / 4 + self.thinking_est.max(self.thinking_chars / 4);
        if tokens >= self.last_tokens + 10 {
            self.last_tokens = tokens;
            out.events.push(ConvEvent::Tokens { tokens });
        }
    }

    fn context_from_usage(&self, usage: &Value, out: &mut Output) {
        let used = u(usage, "input_tokens")
            + u(usage, "cache_read_input_tokens")
            + u(usage, "cache_creation_input_tokens")
            + u(usage, "output_tokens");
        if used > 0 {
            out.events.push(ConvEvent::Session {
                info: SessionInfo {
                    context_used: Some(used),
                    context_window: self.model_window,
                    ..Default::default()
                },
            });
        }
    }

    fn on_assistant(&mut self, v: &Value, out: &mut Output) {
        let parent = Self::parent_of(v);
        let Some(msg) = v.get("message") else { return };
        let msg_id = s(msg, "id").unwrap_or("").to_string();
        let content = msg
            .get("content")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for c in content {
            match s(&c, "type").unwrap_or("") {
                kind @ ("text" | "thinking") => {
                    let is_text = kind == "text";
                    let text = s(&c, "text")
                        .or(s(&c, "thinking"))
                        .unwrap_or("")
                        .to_string();
                    let queue = self.unconfirmed.entry(msg_id.clone()).or_default();
                    let pos = queue.iter().position(|(t, _)| *t == is_text);
                    let id = match pos.and_then(|p| queue.remove(p)) {
                        Some((_, id)) => id,
                        None => {
                            self.next_id(&format!("{msg_id}:{}", if is_text { "t" } else { "r" }))
                        }
                    };
                    if is_text && text.trim().is_empty() {
                        continue;
                    }
                    self.finalized.insert(id.clone());
                    let blocks: Vec<Block> = if text.is_empty() {
                        Vec::new()
                    } else {
                        render::render(&text)
                    };
                    let item = if is_text {
                        Item::Assistant {
                            id,
                            blocks,
                            parent: parent.clone(),
                            streaming: false,
                        }
                    } else {
                        Item::Reasoning {
                            id,
                            blocks,
                            streaming: false,
                        }
                    };
                    out.events.push(ConvEvent::Item { item });
                }
                "tool_use" => {
                    let id = s(&c, "id").unwrap_or_default().to_string();
                    let name = s(&c, "name").unwrap_or_default().to_string();
                    let input = c.get("input").cloned().unwrap_or(Value::Null);
                    self.upsert_tool(out, &id, &name, input, parent.clone(), Status::Running);
                }
                _ => {}
            }
        }
        if parent.is_none() {
            if let Some(usage) = msg.get("usage") {
                self.context_from_usage(usage, out);
            }
        }
        if let Some(err) = s(v, "error") {
            let id = self.next_id("sys");
            out.events.push(ConvEvent::Item {
                item: Item::System {
                    id,
                    level: "error".into(),
                    text: format!("API error: {err}"),
                },
            });
        }
    }

    fn on_user(&mut self, v: &Value, out: &mut Output) {
        let Some(msg) = v.get("message") else { return };
        if v.get("isMeta").and_then(Value::as_bool).unwrap_or(false) {
            return;
        }
        let uuid = s(v, "uuid").unwrap_or("").to_string();
        let content = msg.get("content").cloned().unwrap_or(Value::Null);
        let tool_use_result = v.get("tool_use_result").or_else(|| v.get("toolUseResult"));
        match &content {
            Value::Array(parts) if parts.iter().any(|p| s(p, "type") == Some("tool_result")) => {
                for p in parts.iter().filter(|p| s(p, "type") == Some("tool_result")) {
                    let id = s(p, "tool_use_id").unwrap_or("").to_string();
                    let is_error = p.get("is_error").and_then(Value::as_bool).unwrap_or(false);
                    let text =
                        truncate(tools::result_text(p.get("content").unwrap_or(&Value::Null)));
                    if let Some(t) = self.tools.get_mut(&id) {
                        if t.status != Status::Declined {
                            t.status = if is_error {
                                Status::Error
                            } else {
                                Status::Done
                            };
                        }
                        t.output = Some(text);
                        // A Write to a file that did not exist: the result tells us.
                        if t.name == "Write" {
                            if let Some(r) = tool_use_result {
                                if r.get("originalFile").is_some_and(Value::is_null)
                                    && !t.diffs.is_empty()
                                {
                                    t.diffs[0].kind = "add".into();
                                }
                            }
                        }
                    }
                    if let Some(item) = self.tool_item(&id) {
                        out.events.push(ConvEvent::Item { item });
                    }
                }
                if !self.history && self.perms.is_empty() {
                    self.set_turn(out, TurnState::Requesting, None);
                }
            }
            _ => {
                if v.get("isReplay").and_then(Value::as_bool).unwrap_or(false)
                    || self.sent_uuids.contains(&uuid)
                {
                    // A finished background task is fed to the model as a
                    // replayed message; its <result> is the agent's report.
                    if let Some(text) = content.as_str() {
                        self.live_notification(text, out);
                    }
                    return;
                }
                if v.get("isCompactSummary")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    let id = self.next_id("sys");
                    out.events.push(ConvEvent::Item {
                        item: Item::System {
                            id,
                            level: "compact".into(),
                            text: "Earlier conversation was summarized".into(),
                        },
                    });
                    return;
                }
                let text = match &content {
                    Value::String(t) => t.clone(),
                    Value::Array(parts) => parts
                        .iter()
                        .filter(|p| s(p, "type") == Some("text"))
                        .map(|p| s(p, "text").unwrap_or("").to_string())
                        .collect::<Vec<_>>()
                        .join("\n\n"),
                    _ => String::new(),
                };
                if let Some((tool_use_id, status, summary, result)) =
                    tasks::parse_notification(&text)
                {
                    if let Some(t) = self.tools.get_mut(&tool_use_id) {
                        if let Some(r) = result.clone() {
                            t.output = Some(truncate(r));
                        }
                        t.status = if status == "completed" {
                            Status::Done
                        } else {
                            Status::Error
                        };
                    }
                    if let Some(item) = self.tool_item(&tool_use_id) {
                        out.events.push(ConvEvent::Item { item });
                    }
                    if !summary.is_empty() {
                        let id = self.next_id("sys");
                        out.events.push(ConvEvent::Item {
                            item: Item::System {
                                id,
                                level: "task".into(),
                                text: summary,
                            },
                        });
                    }
                    return;
                }
                if let Some(inner) = local_command(&text) {
                    if !inner.trim().is_empty() {
                        let id = self.next_id("sys");
                        out.events.push(ConvEvent::Item {
                            item: Item::System {
                                id,
                                level: "info".into(),
                                text: inner,
                            },
                        });
                    }
                    return;
                }
                if text.trim().is_empty()
                    || text.starts_with("<command-")
                    || text.starts_with("Caveat:")
                {
                    return;
                }
                let id = if uuid.is_empty() {
                    self.next_id("user")
                } else {
                    uuid
                };
                out.events.push(ConvEvent::Item {
                    item: Item::User {
                        id,
                        blocks: render::render_plain(&text),
                        attachments: Vec::new(),
                        ts: s(v, "timestamp").map(str::to_string),
                    },
                });
            }
        }
    }

    fn on_system(&mut self, v: &Value, out: &mut Output) {
        match s(v, "subtype").unwrap_or("") {
            "init" => {
                let slash = v
                    .get("slash_commands")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();
                out.events.push(ConvEvent::Session {
                    info: SessionInfo {
                        session_id: s(v, "session_id").map(str::to_string),
                        model: s(v, "model").map(str::to_string),
                        permission_mode: s(v, "permissionMode").map(str::to_string),
                        cwd: s(v, "cwd").map(str::to_string),
                        version: s(v, "claude_code_version").map(str::to_string),
                        slash_commands: slash,
                        mcp_servers: v.get("mcp_servers").and_then(Value::as_array).map(|a| {
                            a.iter()
                                .filter_map(|m| {
                                    Some(glaude_core::model::McpBrief {
                                        name: s(m, "name")?.to_string(),
                                        status: s(m, "status").unwrap_or("").to_string(),
                                    })
                                })
                                .collect()
                        }),
                        ..Default::default()
                    },
                });
                if let Some(src) = s(v, "apiKeySource").filter(|s| *s != "none") {
                    out.events.push(ConvEvent::Notice {
                        level: "warn".into(),
                        text: format!("Claude is billing an API key ({src}) instead of your subscription login."),
                    });
                }
            }
            "status" => {
                if let Some(mode) = s(v, "permissionMode") {
                    out.events.push(ConvEvent::Session {
                        info: SessionInfo {
                            permission_mode: Some(mode.to_string()),
                            ..Default::default()
                        },
                    });
                }
                match s(v, "status") {
                    Some("compacting") => self.set_turn(out, TurnState::Compacting, None),
                    Some("requesting") => self.set_turn(out, TurnState::Requesting, None),
                    _ => {}
                }
            }
            "task_started"
            | "task_progress"
            | "task_updated"
            | "task_notification"
            | "background_tasks_changed" => {
                for change in self.tasks.apply(v) {
                    if let (Some(line), false) = (&change.ended, self.history) {
                        // Steps of a finished subagent that never reported back.
                        if let Some(parent) = change.task.tool_use_id.as_deref() {
                            self.settle_children(parent, out);
                        }
                        let id = self.next_id("sys");
                        out.events.push(ConvEvent::Item {
                            item: Item::System {
                                id,
                                level: "task".into(),
                                text: line.clone(),
                            },
                        });
                    }
                    out.events.push(ConvEvent::Task { task: change.task });
                }
            }
            "thinking_tokens" => {
                self.thinking_est = self.thinking_est.max(u(v, "estimated_tokens"));
                self.emit_tokens(out);
            }
            "hook_response" => {
                if !self.history {
                    let text = s(v, "output")
                        .filter(|t| !t.is_empty())
                        .or(s(v, "stderr"))
                        .unwrap_or("");
                    let mut output: String = text.chars().take(4000).collect();
                    if text.len() > output.len() {
                        output.push('…');
                    }
                    self.hook_runs.push_back(HookRun {
                        name: s(v, "hook_name").unwrap_or("").to_string(),
                        event: s(v, "hook_event").unwrap_or("").to_string(),
                        outcome: s(v, "outcome").unwrap_or("").to_string(),
                        exit_code: v.get("exit_code").and_then(Value::as_i64),
                        output,
                        at: tasks::now_ms(),
                    });
                    while self.hook_runs.len() > HOOK_RUNS {
                        self.hook_runs.pop_front();
                    }
                }
            }
            "compact_boundary" => {
                let id = self.next_id("sys");
                out.events.push(ConvEvent::Item {
                    item: Item::System {
                        id,
                        level: "compact".into(),
                        text: "Context compacted".into(),
                    },
                });
            }
            "api_retry" => {
                let detail = format!(
                    "Retry {}/{} in {}s",
                    u(v, "attempt"),
                    u(v, "max_retries"),
                    u(v, "retry_delay_ms") / 1000
                );
                self.set_turn(out, TurnState::Retrying, Some(detail));
            }
            "local_command_output" => {
                if let Some(c) = s(v, "content") {
                    let id = self.next_id("sys");
                    out.events.push(ConvEvent::Item {
                        item: Item::System {
                            id,
                            level: "info".into(),
                            text: c.to_string(),
                        },
                    });
                }
            }
            "permission_denied" | "informational" | "notification" => {
                let text = s(v, "message")
                    .or(s(v, "content"))
                    .or(s(v, "text"))
                    .unwrap_or("")
                    .to_string();
                if !text.is_empty() {
                    let id = self.next_id("sys");
                    out.events.push(ConvEvent::Item {
                        item: Item::System {
                            id,
                            level: "info".into(),
                            text,
                        },
                    });
                }
            }
            _ => {}
        }
    }

    /// Mark a finished subagent's leftover running steps as done.
    fn settle_children(&mut self, parent: &str, out: &mut Output) {
        let open: Vec<String> = self
            .tools
            .iter()
            .filter(|(_, t)| {
                t.parent.as_deref() == Some(parent)
                    && matches!(t.status, Status::Running | Status::Pending)
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in open {
            if let Some(t) = self.tools.get_mut(&id) {
                t.status = Status::Done;
            }
            if let Some(item) = self.tool_item(&id) {
                out.events.push(ConvEvent::Item { item });
            }
        }
    }

    fn on_result(&mut self, v: &Value, out: &mut Output) {
        if let Some(mu) = v.get("modelUsage").and_then(Value::as_object) {
            if let Some(w) = mu
                .values()
                .filter_map(|m| m.get("contextWindow").and_then(Value::as_u64))
                .max()
            {
                self.model_window.get_or_insert(w);
            }
        }
        out.events.push(ConvEvent::Session {
            info: SessionInfo {
                cost_usd: v.get("total_cost_usd").and_then(Value::as_f64),
                ..Default::default()
            },
        });
        // Anything still running when the turn ends did not finish.
        // Anything of the main thread still running when the turn ends did
        // not finish. Subagent steps go on: background agents outlive turns.
        let open: Vec<String> = self
            .tools
            .iter()
            .filter(|(_, t)| {
                t.parent.is_none() && matches!(t.status, Status::Running | Status::Pending)
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in open {
            if let Some(t) = self.tools.get_mut(&id) {
                t.status = Status::Interrupted;
            }
            if let Some(item) = self.tool_item(&id) {
                out.events.push(ConvEvent::Item { item });
            }
        }
        let perms: Vec<String> = self
            .perms
            .iter()
            .filter(|(_, p)| {
                self.tools
                    .get(&p.tool_use_id)
                    .is_none_or(|t| t.parent.is_none())
            })
            .map(|(rid, _)| rid.clone())
            .collect();
        for rid in perms {
            if let Some(p) = self.perms.remove(&rid) {
                out.events.push(ConvEvent::Item {
                    item: self.perm_item(&rid, &p, Status::Interrupted),
                });
            }
        }
        let is_error = v.get("is_error").and_then(Value::as_bool).unwrap_or(false);
        let sub = s(v, "subtype").unwrap_or("success");
        let error = if sub == "error_during_execution" {
            None // usually an interrupt; not worth an error banner
        } else if is_error || sub != "success" {
            Some(s(v, "result").unwrap_or(sub).to_string())
        } else {
            None
        };
        self.turn = TurnState::Idle;
        if !self.history {
            let tokens = v.get("usage").map(|us| TurnTokens {
                input: u(us, "input_tokens") + u(us, "cache_creation_input_tokens"),
                cached: u(us, "cache_read_input_tokens"),
                output: u(us, "output_tokens"),
            });
            out.events.push(ConvEvent::TurnDone {
                duration_ms: v.get("duration_ms").and_then(Value::as_u64),
                error,
                tokens,
            });
            let usage = self.control("usage", json!({"subtype": "get_usage"}));
            let ctx = self.control("ctx", json!({"subtype": "get_context_usage"}));
            out.requests.push(usage);
            out.requests.push(ctx);
        }
    }

    fn on_rate_limit(&mut self, v: &Value, out: &mut Output) {
        let Some(info) = v.get("rate_limit_info") else {
            return;
        };
        let mut limits = Limits::default();
        let window = |w: &Value| -> Option<LimitWindow> {
            let used = w.get("utilization").and_then(Value::as_f64)?;
            Some(LimitWindow {
                used: (used * 100.0).clamp(0.0, 100.0),
                resets_at: w.get("resetsAt").and_then(to_rfc3339),
            })
        };
        if let Some(uw) = info.get("unifiedWindows") {
            limits.five_hour = uw.get("five_hour").and_then(window);
            limits.week = uw.get("seven_day").and_then(window);
        } else {
            match s(info, "rateLimitType") {
                Some("five_hour") => limits.five_hour = window(info),
                Some("seven_day") => limits.week = window(info),
                _ => {}
            }
        }
        if limits.five_hour.is_some() || limits.week.is_some() {
            out.events.push(ConvEvent::Limits { limits });
        }
        if s(info, "status") == Some("rejected") {
            out.events.push(ConvEvent::Notice {
                level: "error".into(),
                text: "Usage limit reached for this account.".into(),
            });
            out.events.push(ConvEvent::Limited {
                until: info.get("resetsAt").and_then(to_rfc3339),
            });
        }
    }

    fn on_control_request(&mut self, v: &Value, out: &mut Output) {
        let rid = s(v, "request_id").unwrap_or("").to_string();
        let Some(req) = v.get("request") else { return };
        match s(req, "subtype").unwrap_or("") {
            "can_use_tool" => {
                let tool = s(req, "display_name")
                    .or(s(req, "tool_name"))
                    .unwrap_or("tool")
                    .to_string();
                let name = s(req, "tool_name").unwrap_or("").to_string();
                let input = req.get("input").cloned().unwrap_or(Value::Null);
                let tool_use_id = s(req, "tool_use_id").unwrap_or("").to_string();
                let title = tools::title(&name, &input, &self.cwd);
                let diffs = tools::diff_preview(&name, &input, &self.cwd);
                let suggestions = req
                    .get("permission_suggestions")
                    .cloned()
                    .unwrap_or(Value::Null);
                let opt = |id: &str, label: &str, kind: &str| PermissionOption {
                    id: id.into(),
                    label: label.into(),
                    kind: kind.into(),
                };
                let mut blocks = Vec::new();
                let options = if name == "ExitPlanMode" {
                    blocks = render::render(s(&input, "plan").unwrap_or(""));
                    vec![
                        opt("allow-accept-edits", "Yes, and auto-accept edits", "allow"),
                        opt("allow-default", "Yes, and ask before each edit", "allow"),
                        opt("deny", "No, keep planning", "deny"),
                    ]
                } else if name == "AskUserQuestion" {
                    vec![opt("deny", "Skip", "deny")]
                } else {
                    let mut o = vec![opt("allow", "Allow", "allow")];
                    if let Some(label) = always_label(&suggestions) {
                        o.push(opt("allow-always", &label, "allow"));
                    }
                    o.push(opt("deny", "Deny", "deny"));
                    o
                };
                let reason = s(req, "decision_reason")
                    .or(s(req, "blocked_path"))
                    .map(str::to_string);
                let p = PermState {
                    tool_use_id,
                    name,
                    tool,
                    title,
                    input,
                    diffs,
                    options,
                    suggestions,
                    reason,
                    blocks,
                    answers: BTreeMap::new(),
                };
                out.events.push(ConvEvent::Item {
                    item: self.perm_item(&rid, &p, Status::Pending),
                });
                self.perms.insert(rid, p);
                self.set_turn(out, TurnState::WaitingPermission, None);
            }
            other => {
                // Hooks/MCP callbacks we did not register: answer with an error
                // so the CLI never blocks waiting on us.
                tracing::debug!(subtype = other, "unhandled control request");
                out.requests.push(json!({
                    "type": "control_response",
                    "response": {"subtype": "error", "request_id": rid, "error": format!("Glaude does not handle {other}")},
                }));
            }
        }
    }

    fn on_cancel(&mut self, v: &Value, out: &mut Output) {
        let rid = s(v, "request_id").unwrap_or("").to_string();
        if let Some(p) = self.perms.remove(&rid) {
            out.events.push(ConvEvent::Item {
                item: self.perm_item(&rid, &p, Status::Interrupted),
            });
        }
    }

    fn on_control_response(&mut self, v: &Value, out: &mut Output) {
        let Some(resp) = v.get("response") else {
            return;
        };
        let rid = s(resp, "request_id").unwrap_or("");
        if s(resp, "subtype") == Some("error") {
            tracing::warn!(
                rid,
                error = s(resp, "error").unwrap_or(""),
                "control request failed"
            );
            return;
        }
        let Some(r) = resp.get("response") else {
            return;
        };
        if rid.starts_with("init-") {
            let models = r
                .get("models")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|m| {
                            Some(ModelChoice {
                                value: s(m, "value")?.to_string(),
                                label: s(m, "displayName").or(s(m, "value"))?.to_string(),
                                efforts: m
                                    .get("supportedEffortLevels")
                                    .and_then(Value::as_array)
                                    .map(|e| {
                                        e.iter()
                                            .filter_map(Value::as_str)
                                            .map(str::to_string)
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let commands: Vec<String> = r
                .get("commands")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|c| s(c, "name"))
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            let agents = r
                .get("agents")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|x| {
                            Some(glaude_core::model::AgentInfo {
                                name: s(x, "name")?.to_string(),
                                description: s(x, "description").unwrap_or("").to_string(),
                                model: s(x, "model").map(str::to_string),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let output_styles = r
                .get("available_output_styles")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            out.events.push(ConvEvent::Session {
                info: SessionInfo {
                    models,
                    agents,
                    output_style: s(r, "output_style").map(str::to_string),
                    output_styles,
                    slash_commands: commands,
                    account_email: r
                        .get("account")
                        .and_then(|a| s(a, "email"))
                        .map(str::to_string),
                    version: s(r, "claude_code_version").map(str::to_string),
                    ..Default::default()
                },
            });
        } else if rid.starts_with("usage-") {
            if let Some(sess) = r.get("session") {
                out.events.push(ConvEvent::Session {
                    info: SessionInfo {
                        cost_usd: sess.get("total_cost_usd").and_then(Value::as_f64),
                        lines_added: sess.get("total_lines_added").and_then(Value::as_u64),
                        lines_removed: sess.get("total_lines_removed").and_then(Value::as_u64),
                        ..Default::default()
                    },
                });
            }
            let window = |w: &Value| -> Option<LimitWindow> {
                Some(LimitWindow {
                    used: w
                        .get("utilization")
                        .and_then(Value::as_f64)?
                        .clamp(0.0, 100.0),
                    resets_at: w.get("resets_at").and_then(to_rfc3339),
                })
            };
            if let Some(rl) = r.get("rate_limits") {
                out.events.push(ConvEvent::Limits {
                    limits: Limits {
                        five_hour: rl.get("five_hour").and_then(window),
                        week: rl.get("seven_day").and_then(window),
                        plan: s(r, "subscription_type").map(str::to_string),
                    },
                });
            }
        } else if rid.starts_with("ctx-") {
            let total = u(r, "totalTokens");
            let max = u(r, "maxTokens");
            if max > 0 {
                out.events.push(ConvEvent::Session {
                    info: SessionInfo {
                        context_used: Some(total),
                        context_window: Some(max),
                        ..Default::default()
                    },
                });
            }
        }
    }
}

fn always_label(suggestions: &Value) -> Option<String> {
    let first = suggestions.as_array()?.first()?;
    Some(match s(first, "type") {
        Some("setMode") => match s(first, "mode") {
            Some("acceptEdits") => "Allow all edits this session".into(),
            Some(m) => format!("Switch to {m}"),
            None => "Always allow".into(),
        },
        Some("addRules") => {
            let rule = first
                .get("rules")
                .and_then(Value::as_array)
                .and_then(|r| r.first())
                .map(|r| match s(r, "ruleContent") {
                    Some(c) => format!("{}({c})", s(r, "toolName").unwrap_or("")),
                    None => s(r, "toolName").unwrap_or("").to_string(),
                })
                .unwrap_or_default();
            if rule.is_empty() {
                "Always allow".into()
            } else {
                format!("Always allow {rule}")
            }
        }
        Some("addDirectories") => "Allow this folder".into(),
        _ => "Always allow".into(),
    })
}

/// Output of a local slash command, wrapped in pseudo-XML by the CLI.
fn local_command(text: &str) -> Option<String> {
    for tag in ["local-command-stdout", "local-command-stderr"] {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        if let Some(start) = text.find(&open) {
            let rest = &text[start + open.len()..];
            let end = rest.find(&close).unwrap_or(rest.len());
            return Some(rest[..end].trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use glaude_core::model::TaskInfo;

    fn replay(fixture: &str) -> (Translator, Vec<ConvEvent>, Vec<Value>) {
        let path = format!(
            "{}/../../fixtures/claude/{fixture}",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read_to_string(path).unwrap();
        let mut t = Translator::new("/tmp/fixproj");
        let mut events = Vec::new();
        let mut requests = Vec::new();
        for line in text.lines() {
            let v: Value = serde_json::from_str(line).unwrap();
            let o = t.feed(&v);
            events.extend(o.events);
            requests.extend(o.requests);
        }
        (t, events, requests)
    }

    fn final_items(events: &[ConvEvent]) -> Vec<Item> {
        let mut order: Vec<String> = Vec::new();
        let mut map: HashMap<String, Item> = HashMap::new();
        for e in events {
            if let ConvEvent::Item { item } = e {
                let id = item.id().to_string();
                if !map.contains_key(&id) {
                    order.push(id.clone());
                }
                map.insert(id, item.clone());
            }
        }
        order
            .into_iter()
            .map(|id| map.remove(&id).unwrap())
            .collect()
    }

    #[test]
    fn bash_turn_produces_tool_and_persian_answer() {
        let (_, events, requests) = replay("bash-turn.ndjson");
        let items = final_items(&events);
        let tool = items
            .iter()
            .find(|i| matches!(i, Item::Tool { .. }))
            .expect("tool item");
        match tool {
            Item::Tool {
                name,
                title,
                status,
                output,
                ..
            } => {
                assert_eq!(name, "Bash");
                assert_eq!(title, "cat note.txt");
                assert_eq!(*status, Status::Done);
                assert_eq!(output.as_deref(), Some("hello fixture"));
            }
            _ => unreachable!(),
        }
        let answer = items
            .iter()
            .find_map(|i| match i {
                Item::Assistant {
                    blocks, streaming, ..
                } => Some((blocks.clone(), *streaming)),
                _ => None,
            })
            .expect("assistant text");
        assert!(!answer.1, "final answer is not streaming");
        assert_eq!(answer.0[0].dir, glaude_core::bidi::Dir::Rtl);
        assert!(answer.0[0].html.contains("<bdi>"), "{}", answer.0[0].html);
        assert!(events
            .iter()
            .any(|e| matches!(e, ConvEvent::TurnDone { error: None, .. })));
        // The turn's own tokens: new input with cache writes, cache reads, output.
        let tokens = events.iter().find_map(|e| match e {
            ConvEvent::TurnDone { tokens, .. } => *tokens,
            _ => None,
        });
        assert_eq!(
            tokens,
            Some(TurnTokens {
                input: 34677,
                cached: 52932,
                output: 219
            })
        );
        assert_eq!(
            requests.len(),
            2,
            "asks for usage and context after the turn"
        );
        // usage + context control responses in the fixture fill the statusline
        assert!(events
            .iter()
            .any(|e| matches!(e, ConvEvent::Limits { limits } if limits.week.is_some())));
        assert!(events
            .iter()
            .any(|e| matches!(e, ConvEvent::Session { info } if info.context_window.is_some())));
    }

    #[test]
    fn streamed_text_and_final_text_share_one_item() {
        let (_, events, _) = replay("bash-turn.ndjson");
        let assistant_ids: HashSet<String> = events
            .iter()
            .filter_map(|e| match e {
                ConvEvent::Item {
                    item: Item::Assistant { id, .. },
                } => Some(id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(assistant_ids.len(), 1, "{assistant_ids:?}");
        assert!(
            events.iter().any(|e| matches!(e, ConvEvent::Blocks { .. })),
            "text streamed"
        );
    }

    #[test]
    fn permission_prompt_and_answer() {
        let path = format!(
            "{}/../../fixtures/claude/write-permission.ndjson",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read_to_string(path).unwrap();
        let mut t = Translator::new("/tmp/fixproj");
        let mut perm_rid = None;
        for line in text.lines() {
            let v: Value = serde_json::from_str(line).unwrap();
            let o = t.feed(&v);
            for e in &o.events {
                if let ConvEvent::Item {
                    item:
                        Item::Permission {
                            request_id,
                            status,
                            options,
                            diffs,
                            title,
                            ..
                        },
                } = e
                {
                    if *status == Status::Pending {
                        assert_eq!(title, "out.txt");
                        assert_eq!(diffs.len(), 1);
                        assert!(options
                            .iter()
                            .any(|o| o.label == "Allow all edits this session"));
                        perm_rid = Some(request_id.clone());
                    }
                }
            }
            if let Some(rid) = perm_rid.take() {
                let (line, out) = t.answer_permission(&rid, "allow", None);
                let line = line.expect("response line");
                assert_eq!(line["response"]["response"]["behavior"], "allow");
                assert!(out.events.iter().any(|e| matches!(
                    e,
                    ConvEvent::Item {
                        item: Item::Permission {
                            status: Status::Done,
                            ..
                        }
                    }
                )));
            }
        }
    }

    #[test]
    fn deny_sends_message() {
        let mut t = Translator::new("/p");
        let req = json!({"type": "control_request", "request_id": "r1", "request": {"subtype": "can_use_tool", "tool_name": "Bash", "input": {"command": "rm -rf x"}, "tool_use_id": "tu1", "permission_suggestions": []}});
        t.feed(&req);
        let (line, _) = t.answer_permission("r1", "deny", Some("use trash instead".into()));
        let r = &line.unwrap()["response"]["response"];
        assert_eq!(r["behavior"], "deny");
        assert_eq!(r["message"], "use trash instead");
    }

    #[test]
    fn unknown_control_requests_get_an_error_reply() {
        let mut t = Translator::new("/p");
        let o = t.feed(&json!({"type": "control_request", "request_id": "h1", "request": {"subtype": "hook_callback"}}));
        assert_eq!(o.requests[0]["response"]["subtype"], "error");
    }

    #[test]
    fn local_command_output_becomes_system_item() {
        let mut t = Translator::for_history("/p");
        let o = t.feed(&json!({"type": "user", "message": {"role": "user", "content": "<local-command-stdout>Total cost: $0.10</local-command-stdout>"}}));
        assert!(
            matches!(&o.events[0], ConvEvent::Item { item: Item::System { text, .. } } if text == "Total cost: $0.10")
        );
    }

    #[test]
    fn ask_user_question_becomes_a_form() {
        let (_, events, _) = replay("ask-question.ndjson");
        let q = final_items(&events)
            .into_iter()
            .find_map(|i| match i {
                Item::Question { questions, .. } => Some(questions),
                _ => None,
            })
            .expect("question item");
        assert_eq!(q[0].question, "Which colour do you prefer?");
        assert_eq!(q[0].options.len(), 2);
        assert!(!q[0].multi_select);
        // Answering builds updatedInput.answers.
        let mut t2 = Translator::new("/tmp/fixproj");
        let req = json!({"type": "control_request", "request_id": "r1", "request": {
            "subtype": "can_use_tool", "tool_name": "AskUserQuestion",
            "input": {"questions": [{"question": "Q?", "header": "H", "options": [{"label": "A"}, {"label": "B"}], "multiSelect": true}]},
            "tool_use_id": "tu1"}});
        let o = t2.feed(&req);
        assert!(matches!(
            &o.events[0],
            ConvEvent::Item {
                item: Item::Question {
                    status: Status::Pending,
                    ..
                }
            }
        ));
        let mut answers = BTreeMap::new();
        answers.insert("Q?".to_string(), "A, B".to_string());
        let (line, out) = t2.answer_question("r1", answers);
        let line = line.unwrap();
        assert_eq!(line["response"]["response"]["behavior"], "allow");
        assert_eq!(
            line["response"]["response"]["updatedInput"]["answers"]["Q?"],
            "A, B"
        );
        assert!(
            matches!(&out.events[0], ConvEvent::Item { item: Item::Question { status: Status::Done, answers, .. } } if answers["Q?"] == "A, B")
        );
    }

    #[test]
    fn plan_approval_sets_the_mode() {
        let mut t = Translator::new("/tmp/fixproj");
        let req = json!({"type": "control_request", "request_id": "p1", "request": {
            "subtype": "can_use_tool", "tool_name": "ExitPlanMode",
            "input": {"plan": "# Plan\n\n1. Do it", "planFilePath": "/tmp/p.md"}, "tool_use_id": "tu2"}});
        let o = t.feed(&req);
        let ConvEvent::Item {
            item: Item::Permission {
                options, blocks, ..
            },
        } = &o.events[0]
        else {
            panic!()
        };
        assert_eq!(
            options.iter().map(|o| o.id.as_str()).collect::<Vec<_>>(),
            ["allow-accept-edits", "allow-default", "deny"]
        );
        assert!(blocks.iter().any(|b| b.html.contains("<h1")));
        let (line, out) = t.answer_permission("p1", "allow-accept-edits", None);
        let r = &line.unwrap()["response"]["response"];
        assert_eq!(r["updatedPermissions"][0]["mode"], "acceptEdits");
        assert!(out.events.iter().any(|e| matches!(e, ConvEvent::Session { info } if info.permission_mode.as_deref() == Some("acceptEdits"))));
        // Feedback keeps planning.
        t.feed(&req);
        let (line, _) = t.answer_permission("p1", "deny", Some("Add tests first".into()));
        assert_eq!(
            line.unwrap()["response"]["response"]["message"],
            "Add tests first"
        );
    }

    #[test]
    fn background_tasks_survive_the_turn() {
        let (_, events, _) = replay("agents-background.ndjson");
        let tasks: Vec<&TaskInfo> = events
            .iter()
            .filter_map(|e| match e {
                ConvEvent::Task { task } => Some(task),
                _ => None,
            })
            .collect();
        assert!(tasks
            .iter()
            .any(|t| t.kind == "agent" && t.status == "completed"));
        assert!(tasks
            .iter()
            .any(|t| t.kind == "shell" && t.status == "completed"));
        // The report comes from the notification fed to the model.
        let last_agent = tasks.iter().rev().find(|t| t.kind == "agent").unwrap();
        assert!(
            last_agent.report.as_deref().is_some_and(|r| !r.is_empty()),
            "{last_agent:?}"
        );
        let items = final_items(&events);
        // Subagent steps are linked to the Agent call and were not cut off.
        let agent_call = items
            .iter()
            .find_map(|i| match i {
                Item::Tool { id, name, .. } if name == "Agent" => Some(id.clone()),
                _ => None,
            })
            .unwrap();
        let children: Vec<&Item> = items
            .iter()
            .filter(|i| matches!(i, Item::Tool { parent: Some(p), .. } if *p == agent_call))
            .collect();
        assert!(!children.is_empty());
        assert!(children.iter().all(|i| !matches!(
            i,
            Item::Tool {
                status: Status::Interrupted,
                ..
            }
        )));
        let lines: Vec<&str> = items
            .iter()
            .filter_map(|i| match i {
                Item::System { level, text, .. } if level == "task" => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 2, "{lines:?}");
    }

    #[test]
    fn workflow_run_reports_agents() {
        let (_, events, _) = replay("workflow.ndjson");
        let last = events
            .iter()
            .rev()
            .find_map(|e| match e {
                ConvEvent::Task { task } => Some(task),
                _ => None,
            })
            .unwrap();
        assert_eq!(last.kind, "workflow");
        assert_eq!(last.status, "completed");
        assert_eq!(last.agents.len(), 2);
        // Two turns: the CLI starts the second itself when the run ends.
        let done = events
            .iter()
            .filter(|e| matches!(e, ConvEvent::TurnDone { .. }))
            .count();
        assert_eq!(done, 2);
    }

    #[test]
    fn transcript_task_notification_fills_the_agent_report() {
        let mut t = Translator::for_history("/tmp/fixproj");
        t.feed(&json!({"type": "assistant", "message": {"id": "m1", "role": "assistant", "content": [
            {"type": "tool_use", "id": "toolu_9", "name": "Agent", "input": {"description": "List files", "subagent_type": "Explore", "prompt": "x"}}]}}));
        let o = t.feed(&json!({"type": "user", "uuid": "u9", "message": {"role": "user", "content":
            "<task-notification>\n<task-id>a1</task-id>\n<tool-use-id>toolu_9</tool-use-id>\n<status>completed</status>\n<summary>Agent \"List files\" finished</summary>\n<result>a.rs, b.rs</result>\n</task-notification>"}}));
        assert!(o.events.iter().any(|e| matches!(e, ConvEvent::Item { item: Item::Tool { output: Some(out), .. } } if out == "a.rs, b.rs")));
        assert!(o.events.iter().any(
            |e| matches!(e, ConvEvent::Item { item: Item::System { level, .. } } if level == "task")
        ));
    }
}

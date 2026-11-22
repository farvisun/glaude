//! Provider-neutral conversation model. Both the Claude and the Codex
//! drivers translate their wire protocols into these types; the UI only
//! knows about these.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::bidi::Dir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    Codex,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Claude => "claude",
            Provider::Codex => "codex",
        }
    }
}

/// A rendered markdown block: sanitized HTML plus its own direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub html: String,
    pub dir: Dir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Pending,
    Running,
    Done,
    Error,
    Declined,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub name: String,
    pub mime: String,
    /// Absolute path on disk (images are cached by the app).
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub path: String,
    /// add | delete | update
    pub kind: String,
    /// Unified diff text.
    pub diff: String,
    pub added: u32,
    pub removed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanStep {
    pub text: String,
    /// pending | inProgress | completed
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionOption {
    pub id: String,
    pub label: String,
    /// allow | deny
    pub kind: String,
}

/// One entry in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Item {
    #[serde(rename_all = "camelCase")]
    User {
        id: String,
        blocks: Vec<Block>,
        attachments: Vec<Attachment>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ts: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Assistant {
        id: String,
        blocks: Vec<Block>,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
        streaming: bool,
    },
    #[serde(rename_all = "camelCase")]
    Reasoning {
        id: String,
        blocks: Vec<Block>,
        streaming: bool,
    },
    #[serde(rename_all = "camelCase")]
    Tool {
        id: String,
        name: String,
        /// Short human summary, e.g. the file path or the command.
        title: String,
        input: Value,
        status: Status,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        diffs: Vec<FileDiff>,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Command {
        id: String,
        command: String,
        cwd: String,
        status: Status,
        output: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
    },
    #[serde(rename_all = "camelCase")]
    FileChange {
        id: String,
        status: Status,
        diffs: Vec<FileDiff>,
    },
    #[serde(rename_all = "camelCase")]
    Plan {
        id: String,
        steps: Vec<PlanStep>,
        #[serde(skip_serializing_if = "Option::is_none")]
        explanation: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Permission {
        id: String,
        /// Provider request id to answer.
        request_id: String,
        tool: String,
        title: String,
        input: Value,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        diffs: Vec<FileDiff>,
        options: Vec<PermissionOption>,
        status: Status,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    System {
        id: String,
        /// info | warn | error | compact
        level: String,
        text: String,
    },
}

impl Item {
    pub fn id(&self) -> &str {
        match self {
            Item::User { id, .. }
            | Item::Assistant { id, .. }
            | Item::Reasoning { id, .. }
            | Item::Tool { id, .. }
            | Item::Command { id, .. }
            | Item::FileChange { id, .. }
            | Item::Plan { id, .. }
            | Item::Permission { id, .. }
            | Item::System { id, .. } => id,
        }
    }
}

/// What the agent is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TurnState {
    Idle,
    Starting,
    Requesting,
    Thinking,
    Writing,
    Tool,
    Compacting,
    Retrying,
    WaitingPermission,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitWindow {
    /// Used percent, 0-100.
    pub used: f64,
    /// RFC 3339 reset time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub five_hour: Option<LimitWindow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub week: Option<LimitWindow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
}

/// Session facts shown in the statusline.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_used: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_added: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_removed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_email: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub slash_commands: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub models: Vec<ModelChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelChoice {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub efforts: Vec<String>,
}

/// Events pushed to the UI for one conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ConvEvent {
    /// Insert or replace a whole item.
    #[serde(rename_all = "camelCase")]
    Item { item: Item },
    /// Replace blocks of a streaming item from `from` onwards.
    #[serde(rename_all = "camelCase")]
    Blocks {
        id: String,
        from: usize,
        blocks: Vec<Block>,
    },
    /// Append raw output to a command or tool item.
    #[serde(rename_all = "camelCase")]
    Output { id: String, chunk: String },
    #[serde(rename_all = "camelCase")]
    Turn {
        state: TurnState,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    TurnDone {
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Session { info: SessionInfo },
    #[serde(rename_all = "camelCase")]
    Limits { limits: Limits },
    /// Many items at once, e.g. history load. Replaces everything.
    #[serde(rename_all = "camelCase")]
    Reset { items: Vec<Item> },
    #[serde(rename_all = "camelCase")]
    Notice { level: String, text: String },
    #[serde(rename_all = "camelCase")]
    Exited {
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<i32>,
    },
}

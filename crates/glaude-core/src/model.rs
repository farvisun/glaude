//! Provider-neutral conversation model. Both the Claude and the Codex
//! drivers translate their wire protocols into these types; the UI only
//! knows about these.

use std::collections::BTreeMap;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionOption {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional preview (code, mockup) shown when the option is focused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub question: String,
    #[serde(default)]
    pub header: String,
    pub options: Vec<QuestionOption>,
    #[serde(default)]
    pub multi_select: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskUsage {
    pub tokens: u64,
    pub tool_uses: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowPhase {
    pub index: u32,
    pub title: String,
}

/// One agent of a Workflow run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowAgent {
    pub index: u32,
    pub label: String,
    /// start | progress | done | error
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_tool_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_preview: Option<String>,
}

/// Work Claude runs outside the main turn: subagents, background shells,
/// monitors and Workflow runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInfo {
    pub id: String,
    /// The tool call that started it (Agent/Task, Bash, Workflow, Monitor).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    /// agent | shell | workflow | monitor | other
    pub kind: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_type: Option<String>,
    /// running | completed | failed | stopped | killed | paused | pending
    pub status: String,
    pub background: bool,
    /// Unix ms.
    pub started_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TaskUsage>,
    /// Latest progress line, e.g. "Running List files".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_tool: Option<String>,
    /// Final report or completion summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub phases: Vec<WorkflowPhase>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub agents: Vec<WorkflowAgent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpBrief {
    pub name: String,
    pub status: String,
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
        /// Rendered body, e.g. the plan of an ExitPlanMode request.
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        blocks: Vec<Block>,
    },
    /// Claude asks the user (AskUserQuestion); answered from the UI.
    #[serde(rename_all = "camelCase")]
    Question {
        id: String,
        /// Provider request id to answer.
        request_id: String,
        questions: Vec<Question>,
        status: Status,
        /// Question text -> chosen label(s), once answered.
        #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
        answers: BTreeMap<String, String>,
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
            | Item::Question { id, .. }
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
    /// Subagents this session can use (built-in, user, project, plugins).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub agents: Vec<AgentInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_style: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub output_styles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<Vec<McpBrief>>,
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
    /// A subagent, background shell, monitor or workflow changed.
    #[serde(rename_all = "camelCase")]
    Task { task: TaskInfo },
    /// Estimated tokens produced so far in this turn.
    #[serde(rename_all = "camelCase")]
    Tokens { tokens: u64 },
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

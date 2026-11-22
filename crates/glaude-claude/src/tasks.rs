//! Subagents, background shells, monitors and Workflow runs. Claude Code
//! reports them with `task_started`, `task_progress`, `task_updated`,
//! `task_notification` and `background_tasks_changed` system events; this
//! merges them into one [`TaskInfo`] per task id.

use std::collections::HashMap;

use glaude_core::model::{TaskInfo, TaskUsage, WorkflowAgent, WorkflowPhase};
use serde_json::Value;

pub fn now_ms() -> u64 {
    chrono::Utc::now().timestamp_millis().max(0) as u64
}

fn s<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

fn opt_u64(v: &Value, k: &str) -> Option<u64> {
    v.get(k).and_then(Value::as_u64)
}

fn kind_of(task_type: Option<&str>, subagent: bool) -> &'static str {
    match task_type {
        Some("local_agent" | "remote_agent" | "in_process_teammate") => "agent",
        Some("local_bash") => "shell",
        Some("local_workflow") => "workflow",
        Some(t) if t.starts_with("monitor") => "monitor",
        None if subagent => "agent",
        _ => "other",
    }
}

fn status_of(s: &str) -> &'static str {
    match s {
        "completed" => "completed",
        "failed" => "failed",
        "stopped" => "stopped",
        "killed" => "killed",
        "paused" => "paused",
        "pending" => "pending",
        _ => "running",
    }
}

fn usage(v: &Value) -> Option<TaskUsage> {
    let u = v.get("usage")?;
    Some(TaskUsage {
        tokens: opt_u64(u, "total_tokens").unwrap_or(0),
        tool_uses: opt_u64(u, "tool_uses").unwrap_or(0),
        duration_ms: opt_u64(u, "duration_ms").unwrap_or(0),
    })
}

fn workflow(t: &mut TaskInfo, v: &Value) {
    let Some(items) = v.get("workflow_progress").and_then(Value::as_array) else {
        return;
    };
    for it in items {
        let index = opt_u64(it, "index").unwrap_or(0) as u32;
        match s(it, "type") {
            Some("workflow_phase") => {
                let title = s(it, "title").unwrap_or("").to_string();
                match t.phases.iter_mut().find(|p| p.index == index) {
                    Some(p) => p.title = title,
                    None => t.phases.push(WorkflowPhase { index, title }),
                }
            }
            Some("workflow_agent") => {
                let str_of = |k: &str| s(it, k).map(str::to_string);
                let a = WorkflowAgent {
                    index,
                    label: s(it, "label").unwrap_or("").to_string(),
                    state: s(it, "state").unwrap_or("start").to_string(),
                    phase_index: opt_u64(it, "phaseIndex").map(|n| n as u32),
                    agent_type: str_of("agentType"),
                    model: str_of("model"),
                    tokens: opt_u64(it, "tokens"),
                    tool_calls: opt_u64(it, "toolCalls"),
                    duration_ms: opt_u64(it, "durationMs"),
                    started_at: opt_u64(it, "startedAt"),
                    error: str_of("error"),
                    last_tool_name: str_of("lastToolName"),
                    last_tool_summary: str_of("lastToolSummary"),
                    result_preview: str_of("resultPreview"),
                    prompt_preview: str_of("promptPreview"),
                };
                match t.agents.iter_mut().find(|x| x.index == index) {
                    Some(x) => *x = a,
                    None => t.agents.push(a),
                }
            }
            _ => {}
        }
    }
    t.phases.sort_by_key(|p| p.index);
    t.agents.sort_by_key(|a| a.index);
}

/// What the transcript should say when a task ends.
fn ended_line(t: &TaskInfo, summary: Option<&str>) -> String {
    let word = match t.status.as_str() {
        "completed" => "finished",
        "failed" => "failed",
        _ => "was stopped",
    };
    match t.kind.as_str() {
        "agent" => format!(
            "{} agent “{}” {word}",
            t.subagent_type.as_deref().unwrap_or("Sub"),
            t.description
        ),
        "workflow" => format!(
            "Workflow “{}” {word}",
            t.workflow_name.as_deref().unwrap_or(&t.description)
        ),
        _ => summary
            .map(str::to_string)
            .unwrap_or_else(|| format!("“{}” {word}", t.description)),
    }
}

#[derive(Default)]
pub struct Tasks {
    map: HashMap<String, TaskInfo>,
}

pub struct Change {
    pub task: TaskInfo,
    /// A line for the transcript when the task ended.
    pub ended: Option<String>,
}

impl Tasks {
    pub fn by_tool_use(&self, tool_use_id: &str) -> Option<&TaskInfo> {
        self.map
            .values()
            .find(|t| t.tool_use_id.as_deref() == Some(tool_use_id))
    }

    fn entry(&mut self, v: &Value) -> Option<&mut TaskInfo> {
        let id = s(v, "task_id")?.to_string();
        Some(self.map.entry(id.clone()).or_insert_with(|| TaskInfo {
            id,
            kind: "other".into(),
            status: "running".into(),
            started_at: now_ms(),
            ..Default::default()
        }))
    }

    /// Apply one `task_*` / `background_tasks_changed` system event.
    pub fn apply(&mut self, v: &Value) -> Vec<Change> {
        let sub = s(v, "subtype").unwrap_or("");
        if sub == "background_tasks_changed" {
            let mut out = Vec::new();
            for t in v
                .get("tasks")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let fresh = s(t, "task_id").is_some_and(|id| !self.map.contains_key(id));
                let Some(e) = self.entry(t) else { continue };
                e.background = true;
                if fresh {
                    e.kind = kind_of(s(t, "task_type"), t.get("subagent_type").is_some()).into();
                    e.description = s(t, "description").unwrap_or("").to_string();
                    e.subagent_type = s(t, "subagent_type").map(str::to_string);
                    out.push(Change {
                        task: e.clone(),
                        ended: None,
                    });
                }
            }
            return out;
        }
        let Some(t) = self.entry(v) else {
            return Vec::new();
        };
        if let Some(id) = s(v, "tool_use_id") {
            t.tool_use_id = Some(id.to_string());
        }
        if let Some(st) = s(v, "subagent_type") {
            t.subagent_type = Some(st.to_string());
        }
        let mut ended = None;
        match sub {
            "task_started" => {
                t.kind = kind_of(s(v, "task_type"), t.subagent_type.is_some()).into();
                t.description = s(v, "description").unwrap_or("").to_string();
                t.background = v
                    .get("is_backgrounded")
                    .and_then(Value::as_bool)
                    .unwrap_or(t.background);
                t.workflow_name = s(v, "workflow_name").map(str::to_string);
                t.parent_task_id = s(v, "parent_task_id").map(str::to_string);
            }
            "task_progress" => {
                let desc = s(v, "description").map(str::to_string);
                let summary = s(v, "summary").map(str::to_string);
                t.activity = if t.kind == "workflow" {
                    desc
                } else {
                    summary.or(desc)
                };
                if let Some(u) = usage(v) {
                    t.usage = Some(u);
                }
                if let Some(n) = s(v, "last_tool_name") {
                    t.last_tool = Some(n.to_string());
                }
                workflow(t, v);
            }
            "task_updated" => {
                if let Some(p) = v.get("patch") {
                    if let Some(st) = s(p, "status") {
                        t.status = status_of(st).into();
                    }
                    if let Some(e) = opt_u64(p, "end_time") {
                        t.ended_at = Some(e);
                    }
                    if let Some(d) = s(p, "description") {
                        t.description = d.to_string();
                    }
                    if let Some(b) = p.get("is_backgrounded").and_then(Value::as_bool) {
                        t.background = b;
                    }
                    if let Some(e) = s(p, "error") {
                        t.report = Some(e.to_string());
                    }
                }
            }
            "task_notification" => {
                t.status = status_of(s(v, "status").unwrap_or("completed")).into();
                t.ended_at.get_or_insert_with(now_ms);
                if let Some(u) = usage(v) {
                    t.usage = Some(u);
                }
                t.output_file = s(v, "output_file").map(str::to_string);
                let summary = s(v, "summary");
                let is_report = v
                    .get("summary_is_report")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if is_report || t.kind != "agent" {
                    t.report = summary.map(str::to_string);
                }
                workflow(t, v);
                ended = Some(ended_line(t, if is_report { None } else { summary }));
            }
            _ => return Vec::new(),
        }
        vec![Change {
            task: t.clone(),
            ended,
        }]
    }
}

/// `<task-notification>` text stored in transcripts, for history replay:
/// (tool_use_id, status, summary, result).
pub fn parse_notification(text: &str) -> Option<(String, String, String, Option<String>)> {
    let t = text.trim_start();
    if !t.starts_with("<task-notification>") {
        return None;
    }
    let tag = |name: &str| -> Option<String> {
        let open = format!("<{name}>");
        let close = format!("</{name}>");
        let a = t.find(&open)? + open.len();
        let b = t[a..].find(&close)? + a;
        Some(t[a..b].trim().to_string())
    };
    Some((
        tag("tool-use-id").unwrap_or_default(),
        tag("status").unwrap_or_else(|| "completed".into()),
        tag("summary").unwrap_or_default(),
        tag("result"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(lines: &str) -> (Tasks, Vec<Change>) {
        let mut t = Tasks::default();
        let mut all = Vec::new();
        for l in lines.lines() {
            let v: Value = serde_json::from_str(l).unwrap();
            if s(&v, "type") == Some("system") {
                all.extend(t.apply(&v));
            }
        }
        (t, all)
    }

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/../../fixtures/claude/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    #[test]
    fn background_agent_and_shell() {
        let (t, changes) = feed(&fixture("agents-background.ndjson"));
        let agent = t.map.values().find(|x| x.kind == "agent").unwrap();
        assert_eq!(agent.subagent_type.as_deref(), Some("Explore"));
        assert_eq!(agent.status, "completed");
        assert!(agent.background);
        assert_eq!(agent.report.as_deref(), Some("NOTES.md, main.rs"));
        assert_eq!(agent.usage.as_ref().unwrap().tool_uses, 1);
        let shell = t.map.values().find(|x| x.kind == "shell").unwrap();
        assert_eq!(shell.status, "completed");
        assert!(shell.report.as_deref().unwrap().contains("exit code 0"));
        let lines: Vec<String> = changes.into_iter().filter_map(|c| c.ended).collect();
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines
            .iter()
            .any(|l| l == "Explore agent “List folder files” finished"));
    }

    #[test]
    fn workflow_phases_and_agents() {
        let (t, _) = feed(&fixture("workflow.ndjson"));
        let wf = t.map.values().next().unwrap();
        assert_eq!(wf.kind, "workflow");
        assert_eq!(wf.workflow_name.as_deref(), Some("probe"));
        assert_eq!(wf.status, "completed");
        assert_eq!(wf.phases.len(), 1);
        assert_eq!(wf.phases[0].title, "Ask");
        assert_eq!(wf.agents.len(), 2);
        assert!(wf.agents.iter().all(|a| a.state == "done"));
        assert_eq!(wf.agents[0].result_preview.as_deref(), Some("alpha"));
    }

    #[test]
    fn stopped_shell() {
        let (t, _) = feed(&fixture("task-stop.ndjson"));
        let sh = t.map.values().next().unwrap();
        assert_eq!(sh.kind, "shell");
        assert_eq!(sh.status, "stopped");
        assert!(sh.ended_at.is_some());
    }

    #[test]
    fn parses_transcript_notification() {
        let text = "<task-notification>\n<task-id>a1</task-id>\n<tool-use-id>toolu_1</tool-use-id>\n<status>completed</status>\n<summary>Agent \"x\" finished</summary>\n<result>NOTES.md, main.rs</result>\n</task-notification>";
        let (id, status, summary, result) = parse_notification(text).unwrap();
        assert_eq!((id.as_str(), status.as_str()), ("toolu_1", "completed"));
        assert_eq!(summary, "Agent \"x\" finished");
        assert_eq!(result.as_deref(), Some("NOTES.md, main.rs"));
        assert!(parse_notification("hello").is_none());
    }
}

// Mirrors crates/glaude-core/src/model.rs. Keep in sync.

export type Provider = "claude" | "codex";
export type Dir = "ltr" | "rtl";

export interface Block {
	html: string;
	dir: Dir;
}

export type Status = "pending" | "running" | "done" | "error" | "declined" | "interrupted";

export interface Attachment {
	name: string;
	mime: string;
	path: string;
}

export interface FileDiff {
	path: string;
	kind: "add" | "delete" | "update" | string;
	diff: string;
	added: number;
	removed: number;
}

export interface PlanStep {
	text: string;
	status: "pending" | "inProgress" | "completed" | string;
}

export interface PermissionOption {
	id: string;
	label: string;
	kind: "allow" | "deny" | string;
}

export interface QuestionOption {
	label: string;
	description?: string;
	preview?: string;
}

export interface Question {
	question: string;
	header: string;
	options: QuestionOption[];
	multiSelect: boolean;
}

export interface TaskUsage {
	tokens: number;
	toolUses: number;
	durationMs: number;
}

export interface WorkflowPhase {
	index: number;
	title: string;
}

export interface WorkflowAgent {
	index: number;
	label: string;
	state: "start" | "progress" | "done" | "error" | string;
	phaseIndex?: number;
	agentType?: string;
	model?: string;
	tokens?: number;
	toolCalls?: number;
	durationMs?: number;
	startedAt?: number;
	error?: string;
	lastToolName?: string;
	lastToolSummary?: string;
	resultPreview?: string;
	promptPreview?: string;
}

/** A subagent, background shell, monitor or Workflow run. */
export interface TaskInfo {
	id: string;
	toolUseId?: string;
	kind: "agent" | "shell" | "workflow" | "monitor" | "other" | string;
	description: string;
	subagentType?: string;
	status: "running" | "completed" | "failed" | "stopped" | "killed" | "paused" | "pending" | string;
	background: boolean;
	startedAt: number;
	endedAt?: number;
	usage?: TaskUsage;
	activity?: string;
	lastTool?: string;
	report?: string;
	outputFile?: string;
	workflowName?: string;
	phases?: WorkflowPhase[];
	agents?: WorkflowAgent[];
	parentTaskId?: string;
}

/** A review note on one diff line ("a" = old side, "b" = new side). */
export interface ReviewNote {
	id: string;
	path: string;
	side: "a" | "b";
	n: number;
	text: string;
	code: string;
}

/** A message typed while a turn ran; sent when it ends. */
export interface QueuedMessage {
	id: string;
	text: string;
	attachments?: Attachment[];
}

export interface AgentInfo {
	name: string;
	description: string;
	model?: string;
}

export type Item =
	| { kind: "user"; id: string; blocks: Block[]; attachments: Attachment[]; ts?: string }
	| { kind: "assistant"; id: string; blocks: Block[]; parent?: string; streaming: boolean }
	| { kind: "reasoning"; id: string; blocks: Block[]; streaming: boolean }
	| {
			kind: "tool";
			id: string;
			name: string;
			title: string;
			input: unknown;
			status: Status;
			output?: string;
			diffs?: FileDiff[];
			parent?: string;
	  }
	| {
			kind: "command";
			id: string;
			command: string;
			cwd: string;
			status: Status;
			output: string;
			exitCode?: number;
			durationMs?: number;
	  }
	| { kind: "fileChange"; id: string; status: Status; diffs: FileDiff[] }
	| { kind: "plan"; id: string; steps: PlanStep[]; explanation?: string }
	| {
			kind: "permission";
			id: string;
			requestId: string;
			tool: string;
			title: string;
			input: unknown;
			diffs?: FileDiff[];
			options: PermissionOption[];
			status: Status;
			reason?: string;
			/** Rendered body, e.g. the plan of ExitPlanMode. */
			blocks?: Block[];
			toolUseId?: string;
	  }
	| {
			kind: "question";
			id: string;
			requestId: string;
			questions: Question[];
			status: Status;
			answers?: Record<string, string>;
			toolUseId?: string;
	  }
	| { kind: "system"; id: string; level: "info" | "warn" | "error" | "compact" | string; text: string };

export type TurnState =
	| "idle"
	| "starting"
	| "requesting"
	| "thinking"
	| "writing"
	| "tool"
	| "compacting"
	| "retrying"
	| "waitingPermission";

export interface LimitWindow {
	used: number;
	resetsAt?: string;
}

export interface Limits {
	fiveHour?: LimitWindow;
	week?: LimitWindow;
	plan?: string;
}

export interface ModelChoice {
	value: string;
	label: string;
	efforts?: string[];
}

export interface SessionInfo {
	sessionId?: string;
	model?: string;
	effort?: string;
	permissionMode?: string;
	cwd?: string;
	gitBranch?: string;
	contextUsed?: number;
	contextWindow?: number;
	costUsd?: number;
	linesAdded?: number;
	linesRemoved?: number;
	version?: string;
	accountEmail?: string;
	slashCommands?: string[];
	models?: ModelChoice[];
	agents?: AgentInfo[];
	outputStyle?: string;
	outputStyles?: string[];
	mcpServers?: { name: string; status: string }[];
}

export type ConvEvent =
	| { type: "item"; item: Item }
	| { type: "blocks"; id: string; from: number; blocks: Block[] }
	| { type: "output"; id: string; chunk: string }
	| { type: "turn"; state: TurnState; detail?: string }
	| { type: "turnDone"; durationMs?: number; error?: string }
	| { type: "session"; info: SessionInfo }
	| { type: "task"; task: TaskInfo }
	| { type: "queue"; items: QueuedMessage[] }
	| { type: "schedule"; limitedUntil?: string; resumeAt?: string }
	| { type: "limited"; until?: string }
	| { type: "tokens"; tokens: number }
	| { type: "limits"; limits: Limits }
	| { type: "reset"; items: Item[] }
	| { type: "notice"; level: string; text: string }
	| { type: "exited"; code?: number };

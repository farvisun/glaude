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
}

export type ConvEvent =
	| { type: "item"; item: Item }
	| { type: "blocks"; id: string; from: number; blocks: Block[] }
	| { type: "output"; id: string; chunk: string }
	| { type: "turn"; state: TurnState; detail?: string }
	| { type: "turnDone"; durationMs?: number; error?: string }
	| { type: "session"; info: SessionInfo }
	| { type: "limits"; limits: Limits }
	| { type: "reset"; items: Item[] }
	| { type: "notice"; level: string; text: string }
	| { type: "exited"; code?: number };

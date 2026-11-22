// What each agent is doing right now, in a few words, and since when.
// Shown in the sidebar, on the agents board, in tab tooltips, in the menu
// bar status icon and in notifications.
import type { Item } from "$lib/types";
import type { Conversation } from "./conversations.svelte";
import { basename } from "$lib/format";

export type Tone = "waiting" | "working" | "done" | "error" | "idle";

export interface Step {
	tone: Tone;
	text: string;
	/** Epoch ms the step started; null when unknown. */
	since: number | null;
}

const short = (s: string, n = 48) => {
	const one = s.replace(/\s+/g, " ").trim();
	return one.length > n ? one.slice(0, n - 1) + "…" : one;
};

/** "Editing App.svelte", "Running bun test", "Reading README.md". */
export function toolStep(it: Item): string {
	if (it.kind === "command") return `Running ${short(it.command, 40)}`;
	if (it.kind === "fileChange") return `Editing ${it.diffs.map((d) => basename(d.path)).join(", ") || "files"}`;
	if (it.kind !== "tool") return "Working";
	const t = it.title;
	switch (it.name) {
		case "Read":
		case "NotebookRead":
			return `Reading ${basename(t) || "a file"}`;
		case "Edit":
		case "MultiEdit":
		case "NotebookEdit":
			return `Editing ${basename(t) || "a file"}`;
		case "Write":
			return `Writing ${basename(t) || "a file"}`;
		case "Bash":
			return `Running ${short(t, 40)}`;
		case "Grep":
		case "Glob":
			return `Searching ${short(t, 32)}`;
		case "WebSearch":
			return `Searching the web`;
		case "WebFetch":
			try {
				return `Fetching ${new URL(t).host}`;
			} catch {
				return "Fetching a page";
			}
		case "Task":
		case "Agent":
			return `Agent: ${short(t, 40)}`;
		case "TodoWrite":
			return "Updating the plan";
		case "Skill":
			return `Using ${t || "a skill"}`;
	}
	if (it.name.startsWith("mcp__")) {
		const [, server, tool] = it.name.split("__");
		return `${server}: ${tool ?? ""}`.trim();
	}
	return short(`${it.name} ${t}`, 48);
}

/** The pending approval or question, if any. */
export function pendingOf(conv: Conversation): Extract<Item, { kind: "permission" | "question" }> | undefined {
	for (let i = conv.items.length - 1; i >= 0; i--) {
		const it = conv.items[i];
		if ((it.kind === "permission" || it.kind === "question") && it.status === "pending") return it;
	}
	return undefined;
}

/** The tool call running now (main thread first). */
export function currentTool(conv: Conversation): Item | undefined {
	let nested: Item | undefined;
	for (let i = conv.items.length - 1; i >= 0; i--) {
		const it = conv.items[i];
		if ((it.kind === "tool" || it.kind === "command" || it.kind === "fileChange") && (it.status === "running" || it.status === "pending")) {
			if (it.kind === "tool" && it.parent) nested ??= it;
			else return it;
		}
	}
	return nested;
}

export function waitingText(it: Extract<Item, { kind: "permission" | "question" }>): string {
	if (it.kind === "question") return "Has a question";
	if (it.tool === "ExitPlanMode") return "Plan ready for review";
	const what = it.tool === "Bash" ? short(it.title, 36) : basename(it.title) || it.tool;
	return `Allow ${it.tool === "Bash" ? "" : it.tool + " "}${what}?`.replace(/\s+\?$/, "?");
}

export function stepOf(conv: Conversation): Step {
	const pending = pendingOf(conv);
	if (pending) return { tone: "waiting", text: waitingText(pending), since: conv.timeOf(pending.id)?.start ?? null };
	if (conv.turn !== "idle") {
		const cur = conv.turn === "tool" || conv.turn === "waitingPermission" ? currentTool(conv) : undefined;
		if (cur) return { tone: "working", text: toolStep(cur), since: conv.timeOf(cur.id)?.start ?? conv.turnStartedAt };
		const text =
			conv.turn === "thinking"
				? "Thinking"
				: conv.turn === "writing"
					? "Writing a reply"
					: conv.turn === "compacting"
						? "Compacting context"
						: conv.turn === "retrying"
							? `Retrying${conv.turnDetail ? ` · ${short(conv.turnDetail, 30)}` : ""}`
							: conv.turn === "tool"
								? "Running a tool"
								: "Waiting for the model";
		return { tone: "working", text, since: conv.turnStartedAt };
	}
	const bg = conv.runningTasks;
	if (bg.length) {
		const first = bg[0];
		const text = bg.length === 1 ? short(first.activity ?? first.description ?? "Background task", 44) : `${bg.length} background tasks`;
		return { tone: "working", text, since: Math.min(...bg.map((t) => t.startedAt || Date.now())) };
	}
	if (conv.lastError && conv.unread) return { tone: "error", text: "Stopped with an error", since: conv.lastTurnEndedAt };
	if (conv.unread) return { tone: "done", text: "Finished", since: conv.lastTurnEndedAt };
	return { tone: "idle", text: conv.alive ? "Idle" : "Paused", since: conv.lastTurnEndedAt };
}

/** Compact elapsed time: "8s", "1m 20s", "2h 5m". */
export function fmtElapsed(ms: number): string {
	const s = Math.max(0, Math.floor(ms / 1000));
	if (s < 60) return `${s}s`;
	const m = Math.floor(s / 60);
	if (m < 60) return `${m}m ${String(s % 60).padStart(2, "0")}s`;
	const h = Math.floor(m / 60);
	return `${h}h ${m % 60}m`;
}

/** Order for lists of agents: waiting (longest first), working, finished, idle. */
export function byUrgency(a: { step: Step }, b: { step: Step }): number {
	const rank: Record<Tone, number> = { waiting: 0, error: 1, working: 2, done: 3, idle: 4 };
	const r = rank[a.step.tone] - rank[b.step.tone];
	if (r) return r;
	return (a.step.since ?? Infinity) - (b.step.since ?? Infinity);
}

const textCache = new WeakMap<object, string>();
/** Plain text of rendered blocks (cached per blocks array). */
export function plainText(blocks: { html: string }[]): string {
	const hit = textCache.get(blocks);
	if (hit !== undefined) return hit;
	const div = document.createElement("div");
	div.innerHTML = blocks.map((b) => b.html).join("\n");
	const text = (div.textContent ?? "").replace(/\s+/g, " ").trim();
	textCache.set(blocks, text);
	return text;
}

/** The agent's latest reply on the main thread, as plain text. */
export function lastReply(conv: Conversation): string {
	for (let i = conv.items.length - 1; i >= 0; i--) {
		const it = conv.items[i];
		if (it.kind === "assistant" && !it.parent && it.blocks.length) return plainText(it.blocks);
	}
	return "";
}

/** The latest user prompt, as plain text. */
export function lastPrompt(conv: Conversation): string {
	for (let i = conv.items.length - 1; i >= 0; i--) {
		const it = conv.items[i];
		if (it.kind === "user" && it.blocks.length) return plainText(it.blocks);
	}
	return "";
}

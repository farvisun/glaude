// Client-side conversation state. Rust pushes ConvEvents over a channel;
// this store applies them. Items live in a $state.raw array that is
// replaced (not mutated) so only changed rows re-render.

import type { ConvEvent, Item, Limits, Provider, QueuedMessage, SessionInfo, TaskInfo, TurnState, TurnTokens } from "$lib/types";
import { badges } from "./badges.svelte";
import { groupItems } from "./group";

/** What one turn took: time, cost, tokens and the files it changed. */
export interface TurnStat {
	durationMs?: number;
	costUsd?: number;
	tokens?: TurnTokens;
	files: string[];
	error?: string;
	endedAt: number;
}

const EDIT_TOOLS = new Set(["Edit", "MultiEdit", "Write", "NotebookEdit"]);

/** Files changed by these items (edit tools and Codex file changes). */
export function filesChanged(items: Item[]): string[] {
	const out = new Set<string>();
	for (const it of items) {
		if (it.kind === "fileChange") it.diffs.forEach((d) => out.add(d.path));
		else if (it.kind === "tool" && EDIT_TOOLS.has(it.name) && it.status !== "declined" && it.status !== "error") {
			if (it.diffs?.length) it.diffs.forEach((d) => out.add(d.path));
			else if (it.title) out.add(it.title);
		}
	}
	return [...out];
}

const ACTIVE = new Set(["running", "pending"]);
function statusOf(it: Item | undefined): string | undefined {
	if (!it) return undefined;
	if (it.kind === "assistant" || it.kind === "reasoning") return it.streaming ? "running" : "done";
	return "status" in it ? it.status : undefined;
}

export interface ConvSpec {
	id: string;
	provider: Provider;
	account: string;
	cwd: string;
	title: string;
	sessionId?: string;
}

export class Conversation {
	readonly id: string;
	provider: Provider;
	account = $state("");
	cwd = $state("");
	title = $state("");
	items = $state.raw<Item[]>([]);
	turn = $state<TurnState>("idle");
	turnDetail = $state<string | undefined>(undefined);
	turnStartedAt = $state<number | null>(null);
	lastTurnMs = $state<number | null>(null);
	session = $state<SessionInfo>({});
	limits = $state<Limits>({});
	alive = $state(false);
	/** Last event or message (ms); used to stop idle processes. Not reactive. */
	lastActivity = Date.now();
	loading = $state(false);
	/** Subagents, background shells, monitors and workflows, by task id. */
	tasks = $state<Record<string, TaskInfo>>({});
	/** Estimated tokens produced in the current turn. */
	turnTokens = $state(0);
	/** Text to put back into the composer (after a rewind). */
	draft = $state<string | null>(null);
	/** Messages to send when the turn ends (kept by the host). */
	queue = $state<QueuedMessage[]>([]);
	/** Usage limit hit: when it resets ("" = unknown). */
	limitedUntil = $state<string | null>(null);
	/** A "continue" is scheduled at this time. */
	resumeAt = $state<string | null>(null);
	/** The terminal drawer under the conversation is shown. */
	drawer = $state(false);
	/** Side chat: the conversation it was forked from. */
	sideOf = $state<string | null>(null);
	/** Cross-review: the conversation whose changes this one reviews. */
	reviewOf = $state<string | null>(null);
	/** Text to quote into the composer (from a side chat). */
	quote = $state<string | null>(null);
	/** Restored tab: its history loads from the transcript when shown. */
	needsHistory = $state(false);

	private index = new Map<string, number>();

	constructor(spec: ConvSpec) {
		this.id = spec.id;
		this.provider = spec.provider;
		this.account = spec.account;
		this.cwd = spec.cwd;
		this.title = spec.title;
		if (spec.sessionId) this.session = { sessionId: spec.sessionId };
	}

	get pendingPermissions(): number {
		return this.items.filter((i) => (i.kind === "permission" || i.kind === "question") && i.status === "pending").length;
	}

	/** The task a tool call started (Agent, Bash in background, Workflow). */
	taskFor(toolUseId: string): TaskInfo | undefined {
		for (const t of Object.values(this.tasks)) if (t.toolUseId === toolUseId) return t;
		return undefined;
	}

	get runningTasks(): TaskInfo[] {
		return Object.values(this.tasks).filter((t) => t.status === "running" || t.status === "pending");
	}

	get running(): boolean {
		return this.turn !== "idle";
	}

	/** Drop a message and everything after it (conversation rewind). */
	truncateBefore(itemId: string) {
		const at = this.items.findIndex((i) => i.id === itemId);
		if (at < 0) return;
		this.items = this.items.slice(0, at);
		this.reindex();
	}

	private reindex() {
		this.index.clear();
		this.items.forEach((it, i) => this.index.set(it.id, i));
	}

	/** When items started and ended (ms), for items seen live. Not reactive:
	 *  the items array changes whenever a status does. */
	private times = new Map<string, { start: number; end?: number }>();

	timeOf(id: string): { start: number; end?: number } | undefined {
		return this.times.get(id);
	}

	/** Stats of finished turns, keyed by the turn's last item. */
	turnStats = $state<Record<string, TurnStat>>({});
	/** The stats of the turn that just ended. */
	lastStat: TurnStat | null = null;
	lastTurnEndedAt = $state<number | null>(null);
	lastError = $state<string | null>(null);
	private turnFrom = 0;
	private costAtStart = 0;

	private track(item: Item, prev: Item | undefined) {
		const now = Date.now();
		const st = statusOf(item);
		const t = this.times.get(item.id);
		if (!t) {
			// History arrives finished while idle: no times for it.
			if (st && ACTIVE.has(st)) this.times.set(item.id, { start: now });
			else if (!prev && this.turn !== "idle") this.times.set(item.id, { start: now, end: now });
		} else if (t.end === undefined && st && !ACTIVE.has(st) && ACTIVE.has(statusOf(prev) ?? "")) t.end = now;
	}

	private upsert(item: Item) {
		if (item.kind === "reasoning") this.timeThinking(item.id, item.streaming);
		const at = this.index.get(item.id);
		this.track(item, at === undefined ? undefined : this.items[at]);
		const next = this.items.slice();
		if (at === undefined) {
			this.index.set(item.id, next.length);
			next.push(item);
		} else next[at] = item;
		this.items = next;
	}

	/** When each live reasoning item started and stopped (ms); not reactive. */
	private thinkTimes = new Map<string, { start: number; end?: number }>();

	private timeThinking(id: string, streaming: boolean) {
		const t = this.thinkTimes.get(id);
		if (streaming && !t) this.thinkTimes.set(id, { start: Date.now() });
		else if (!streaming && t && t.end === undefined) t.end = Date.now();
	}

	/** How long the model thought, if this session watched it happen. */
	thoughtFor(id: string): number | null {
		const t = this.thinkTimes.get(id);
		return t?.end !== undefined ? t.end - t.start : null;
	}

	apply(ev: ConvEvent) {
		this.lastActivity = Date.now();
		switch (ev.type) {
			case "item":
				this.upsert(ev.item);
				break;
			case "blocks": {
				const at = this.index.get(ev.id);
				if (at === undefined) break;
				const it = this.items[at];
				if (it.kind !== "assistant" && it.kind !== "reasoning" && it.kind !== "user") break;
				this.upsert({ ...it, blocks: [...it.blocks.slice(0, ev.from), ...ev.blocks] } as Item);
				break;
			}
			case "output": {
				const at = this.index.get(ev.id);
				if (at === undefined) break;
				const it = this.items[at];
				if (it.kind === "command") this.upsert({ ...it, output: it.output + ev.chunk });
				else if (it.kind === "tool") this.upsert({ ...it, output: (it.output ?? "") + ev.chunk });
				break;
			}
			case "turn":
				if (this.turn === "idle" && ev.state !== "idle") {
					this.turnStartedAt = Date.now();
					this.turnTokens = 0;
					this.turnFrom = this.items.length;
					this.costAtStart = this.session.costUsd ?? 0;
				}
				this.turn = ev.state;
				this.turnDetail = ev.detail;
				break;
			case "turnDone":
				this.finishTurn(ev);
				this.turn = "idle";
				this.turnDetail = undefined;
				this.lastTurnMs = ev.durationMs ?? (this.turnStartedAt ? Date.now() - this.turnStartedAt : null);
				this.turnStartedAt = null;
				break;
			case "session":
				this.session = { ...this.session, ...ev.info };
				break;
			case "task":
				this.tasks = { ...this.tasks, [ev.task.id]: ev.task };
				break;
			case "tokens":
				this.turnTokens = ev.tokens;
				break;
			case "queue":
				this.queue = ev.items;
				break;
			case "schedule":
				this.limitedUntil = ev.limitedUntil ?? null;
				this.resumeAt = ev.resumeAt ?? null;
				break;
			case "limited":
				break;
			case "limits":
				this.limits = { ...this.limits, ...ev.limits };
				break;
			case "reset":
				this.items = ev.items;
				this.reindex();
				if (ev.items.length === 0) this.tasks = {};
				break;
			case "exited": {
				this.alive = false;
				this.turn = "idle";
				// Background work dies with the process.
				const now = Date.now();
				const next = { ...this.tasks };
				for (const t of Object.values(next))
					if (t.status === "running" || t.status === "pending") next[t.id] = { ...t, status: "stopped", endedAt: now };
				this.tasks = next;
				break;
			}
			case "notice":
				break;
		}
		this.updateBadge();
	}

	private finishTurn(ev: Extract<ConvEvent, { type: "turnDone" }>) {
		const turnItems = this.items.slice(this.turnFrom);
		// Same anchor as the view: the last row of the turn on the main thread.
		const anchor = [...groupItems(turnItems).main].reverse().find((i) => i.kind !== "user");
		const cost = this.session.costUsd;
		// The total is per process: a restarted process starts from zero.
		const costUsd = cost == null ? undefined : cost >= this.costAtStart ? cost - this.costAtStart : cost;
		const stat: TurnStat = {
			durationMs: ev.durationMs ?? (this.turnStartedAt ? Date.now() - this.turnStartedAt : undefined),
			costUsd,
			tokens: ev.tokens,
			files: filesChanged(turnItems),
			error: ev.error,
			endedAt: Date.now(),
		};
		this.lastStat = stat;
		this.lastTurnEndedAt = stat.endedAt;
		this.lastError = ev.error ?? null;
		if (anchor) this.turnStats = { ...this.turnStats, [anchor.id]: stat };
		this.costAtStart = cost ?? 0;
	}

	/** Finished while its tab was not in view. */
	unread = $state(false);

	markUnread(v: boolean) {
		if (this.unread === v) return;
		this.unread = v;
		this.updateBadge();
	}

	private updateBadge() {
		badges.set(
			this.id,
			this.pendingPermissions > 0
				? "attention"
				: this.running || this.runningTasks.length > 0
					? "running"
					: this.unread
						? "unread"
						: null,
		);
	}
}

class Conversations {
	byId = $state<Record<string, Conversation>>({});

	get(id: string): Conversation | undefined {
		return this.byId[id];
	}

	add(spec: ConvSpec): Conversation {
		const c = new Conversation(spec);
		this.byId[spec.id] = c;
		return c;
	}

	remove(id: string) {
		delete this.byId[id];
	}
}

export const conversations = new Conversations();

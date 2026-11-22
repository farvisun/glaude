// Client-side conversation state. Rust pushes ConvEvents over a channel;
// this store applies them. Items live in a $state.raw array that is
// replaced (not mutated) so only changed rows re-render.

import type { ConvEvent, Item, Limits, Provider, QueuedMessage, SessionInfo, TaskInfo, TurnState } from "$lib/types";
import { badges } from "./badges.svelte";

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

	private upsert(item: Item) {
		if (item.kind === "reasoning") this.timeThinking(item.id, item.streaming);
		const at = this.index.get(item.id);
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
				}
				this.turn = ev.state;
				this.turnDetail = ev.detail;
				break;
			case "turnDone":
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

	private updateBadge() {
		badges.set(
			this.id,
			this.pendingPermissions > 0 ? "attention" : this.running || this.runningTasks.length > 0 ? "running" : null,
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

// Client-side conversation state. Rust pushes ConvEvents over a channel;
// this store applies them. Items live in a $state.raw array that is
// replaced (not mutated) so only changed rows re-render.

import type { ConvEvent, Item, Limits, Provider, SessionInfo, TurnState } from "$lib/types";
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
		return this.items.filter((i) => i.kind === "permission" && i.status === "pending").length;
	}

	get running(): boolean {
		return this.turn !== "idle";
	}

	private reindex() {
		this.index.clear();
		this.items.forEach((it, i) => this.index.set(it.id, i));
	}

	private upsert(item: Item) {
		const at = this.index.get(item.id);
		const next = this.items.slice();
		if (at === undefined) {
			this.index.set(item.id, next.length);
			next.push(item);
		} else next[at] = item;
		this.items = next;
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
				if (this.turn === "idle" && ev.state !== "idle") this.turnStartedAt = Date.now();
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
			case "limits":
				this.limits = { ...this.limits, ...ev.limits };
				break;
			case "reset":
				this.items = ev.items;
				this.reindex();
				break;
			case "exited":
				this.alive = false;
				this.turn = "idle";
				break;
			case "notice":
				break;
		}
		this.updateBadge();
	}

	private updateBadge() {
		badges.set(this.id, this.pendingPermissions > 0 ? "attention" : this.running ? "running" : null);
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

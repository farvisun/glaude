// Accounts page state (mamadoo port). The list comes from disk quickly;
// usage scans run one account at a time in the background, active first.

import { call, channel } from "$lib/ipc";
import type { LimitWindow, Provider } from "$lib/types";
import { conversations } from "./conversations.svelte";
import { showError } from "./sessions";

export interface AccountView {
	provider: Provider;
	name: string;
	path: string;
	archived: boolean;
	protected: boolean;
	active: boolean;
	running: boolean;
	email: string;
	org: string;
	plan: string;
	fiveHour: LimitWindow | null;
	week: LimitWindow | null;
	limitsAsOf: string;
	tokens: number;
	tokensToday: number;
	tokens7d: number;
	sessions: number;
	launches: number;
	lastUsed: string | null;
	createdAt: string | null;
	usageUpdated: string | null;
	sharedLinked: number;
	sharedPending: number;
	sharedTotal: number;
	details: { label: string; text: string }[];
}

export type SortKey = "name" | "recent" | "tokens";

export const accountKey = (a: { provider: Provider; name: string }) => `${a.provider}:${a.name}`;

class Accounts {
	list = $state.raw<AccountView[]>([]);
	loaded = $state(false);
	/** Scan progress (0–1) per account key while it is being refreshed. */
	progress = $state<Record<string, number>>({});
	private queue: AccountView[] = [];
	private draining = false;

	async load() {
		try {
			this.list = await call<AccountView[]>("accounts_list");
		} catch (e) {
			showError("Could not read accounts", e);
		} finally {
			this.loaded = true;
		}
	}

	/** Live limits from open conversations beat what is cached on disk. */
	withLive(a: AccountView): AccountView {
		let five = a.fiveHour;
		let week = a.week;
		for (const c of Object.values(conversations.byId)) {
			if (c.provider !== a.provider || (c.account || "default") !== a.name) continue;
			if (c.limits.fiveHour) five = c.limits.fiveHour;
			if (c.limits.week) week = c.limits.week;
		}
		return five === a.fiveHour && week === a.week ? a : { ...a, fiveHour: five, week };
	}

	private replace(v: AccountView) {
		this.list = this.list.map((a) => (accountKey(a) === accountKey(v) ? v : a));
	}

	/** Queue usage scans (deduplicated), active accounts first. */
	refresh(items: AccountView[]) {
		const queued = new Set(this.queue.map(accountKey));
		const sorted = [...items].sort((a, b) => Number(b.active) - Number(a.active));
		for (const a of sorted) {
			const k = accountKey(a);
			if (queued.has(k) || k in this.progress) continue;
			queued.add(k);
			this.queue.push(a);
		}
		void this.drain();
	}

	refreshAll() {
		this.refresh(this.list.filter((a) => !a.archived));
	}

	private async drain() {
		if (this.draining) return;
		this.draining = true;
		try {
			while (this.queue.length) {
				const a = this.queue.shift()!;
				const k = accountKey(a);
				this.progress = { ...this.progress, [k]: 0 };
				try {
					const ch = await channel<number>((f) => (this.progress = { ...this.progress, [k]: f }));
					const v = await call<AccountView>("accounts_refresh", { provider: a.provider, name: a.name, onProgress: ch.raw });
					this.replace(v);
				} catch (e) {
					showError(`Could not scan ${a.name}`, e);
				} finally {
					const { [k]: _, ...rest } = this.progress;
					this.progress = rest;
				}
			}
		} finally {
			this.draining = false;
		}
	}

	async create(provider: Provider, name: string): Promise<string | null> {
		try {
			const created = await call<string>("accounts_create", { provider, name });
			await this.load();
			return created;
		} catch (e) {
			showError("Could not add the account", e);
			return null;
		}
	}

	async rename(a: AccountView, newName: string): Promise<boolean> {
		try {
			await call("accounts_rename", { provider: a.provider, name: a.name, newName });
			await this.load();
			return true;
		} catch (e) {
			showError("Could not rename the account", e);
			return false;
		}
	}

	async remove(a: AccountView): Promise<boolean> {
		try {
			await call("accounts_delete", { provider: a.provider, name: a.name });
			await this.load();
			return true;
		} catch (e) {
			showError("Could not delete the account", e);
			return false;
		}
	}

	async archive(a: AccountView, archived: boolean) {
		try {
			await call("accounts_archive", { provider: a.provider, name: a.name, archived });
			this.replace({ ...a, archived });
		} catch (e) {
			showError("Could not update the account", e);
		}
	}

	async makeActive(a: AccountView) {
		try {
			await call("accounts_set_active", { provider: a.provider, name: a.name });
			this.list = this.list.map((x) => (x.provider === a.provider ? { ...x, active: x.name === a.name } : x));
		} catch (e) {
			showError("Could not switch the active account", e);
		}
	}

	async share(a: AccountView, dryRun: boolean): Promise<string[] | null> {
		try {
			const log = await call<string[]>("accounts_share", { provider: a.provider, name: a.name, dryRun });
			if (!dryRun) await this.load();
			return log;
		} catch (e) {
			showError("Could not share the account", e);
			return null;
		}
	}

	async sortPref(provider: Provider): Promise<SortKey> {
		try {
			const v = await call<string>("accounts_pref", { provider, key: "sort", value: null });
			return v === "recent" || v === "tokens" ? v : "name";
		} catch {
			return "name";
		}
	}

	saveSort(provider: Provider, sort: SortKey) {
		call("accounts_pref", { provider, key: "sort", value: sort }).catch(() => {});
	}
}

export const accounts = new Accounts();

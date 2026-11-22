// Saved proxies (Settings → Network). The list is kept by the app in a
// private file (a URL can carry a password); conversations and terminals
// refer to a proxy by id. Which proxy a conversation uses, most specific
// first: its own choice, the project's default, the account's default,
// the app default.
import { call } from "$lib/ipc";
import { toast } from "svelte-sonner";
import type { Provider } from "$lib/types";

export interface Proxy {
	id: string;
	name: string;
	url: string;
	noProxy: string;
}

export interface ProxySettings {
	proxies: Proxy[];
	appDefault: string | null;
	/** "claude:work" → proxy id or "none". */
	byAccount: Record<string, string>;
	/** Project folder → proxy id or "none". */
	byProject: Record<string, string>;
	/** Terminals opened in Glaude use the proxy too. */
	terminals: boolean;
}

export interface Probe {
	target: string;
	ok: boolean;
	status?: number | null;
	ms?: number | null;
	error?: string | null;
}

export interface CheckResult {
	probes: Probe[];
	ip?: string | null;
	country?: string | null;
}

export type ProxySource = "conversation" | "project" | "account" | "app";

export interface Resolved {
	/** Proxy id, or null for none. */
	id: string | null;
	proxy: Proxy | null;
	/** Where the choice came from (null: nothing set anywhere). */
	source: ProxySource | null;
}

export interface ProxyTarget {
	provider?: Provider;
	account?: string;
	cwd?: string;
}

const EMPTY: ProxySettings = { proxies: [], appDefault: null, byAccount: {}, byProject: {}, terminals: true };

/** The URL with its password hidden, for lists and tooltips. */
export function maskUrl(url: string): string {
	return url.replace(/^([a-z0-9+.-]+:\/\/[^:@/]*):[^@/]*@/i, "$1:•••@");
}

export const isSocks = (url: string) => /^socks/i.test(url.trim());

export const sourceLabel: Record<ProxySource, string> = {
	conversation: "this conversation",
	project: "project default",
	account: "account default",
	app: "app default",
};

class Proxies {
	s = $state<ProxySettings>({ ...EMPTY });
	loaded = $state(false);
	private loading: Promise<void> | null = null;

	get list(): Proxy[] {
		return this.s.proxies;
	}

	/** Read the saved list once (and again after `reload`). */
	ready(): Promise<void> {
		this.loading ??= call<ProxySettings>("proxies_get")
			.then((v) => {
				this.s = { ...EMPTY, ...v };
			})
			.catch(() => {})
			.finally(() => (this.loaded = true));
		return this.loading;
	}

	async save(next: ProxySettings): Promise<boolean> {
		const prev = this.s;
		this.s = next;
		try {
			await call("proxies_set", { settings: next });
			return true;
		} catch (e) {
			this.s = prev;
			toast.error("Could not save the proxy settings", { description: e instanceof Error ? e.message : String(e) });
			return false;
		}
	}

	byId(id: string | null | undefined): Proxy | null {
		return (id && this.s.proxies.find((p) => p.id === id)) || null;
	}

	/** The project rule for a folder: the closest folder with one. */
	projectRule(cwd: string | undefined): string | undefined {
		if (!cwd) return undefined;
		let best: string | undefined;
		for (const dir of Object.keys(this.s.byProject)) {
			if ((cwd === dir || cwd.startsWith(dir.endsWith("/") ? dir : dir + "/")) && (!best || dir.length > best.length)) best = dir;
		}
		return best ? this.s.byProject[best] : undefined;
	}

	private pick(id: string, source: ProxySource): Resolved {
		const proxy = id === "none" ? null : this.byId(id);
		return { id: proxy?.id ?? null, proxy, source };
	}

	/** The default for a conversation (no choice of its own). */
	defaultFor(t: ProxyTarget): Resolved {
		const project = this.projectRule(t.cwd);
		if (project) return this.pick(project, "project");
		const account = t.provider && t.account ? this.s.byAccount[`${t.provider}:${t.account}`] : undefined;
		if (account) return this.pick(account, "account");
		if (this.s.appDefault) return this.pick(this.s.appDefault, "app");
		return { id: null, proxy: null, source: null };
	}

	/** `choice`: a proxy id, "none", or undefined for the default. */
	resolve(t: ProxyTarget, choice?: string | null): Resolved {
		if (choice) return this.pick(choice, "conversation");
		return this.defaultFor(t);
	}

	/** A plain shell in `cwd` (no agent): nothing when terminals are off. */
	forShell(cwd?: string): string | null {
		if (!this.s.terminals) return null;
		return this.defaultFor({ cwd }).id;
	}

	async check(url: string): Promise<CheckResult> {
		return call<CheckResult>("proxy_check", { url });
	}
}

export const proxies = new Proxies();

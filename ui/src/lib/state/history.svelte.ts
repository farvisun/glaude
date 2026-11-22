// Past sessions on disk (Claude transcripts, Codex threads) for the sidebar.
import { call } from "$lib/ipc";
import type { Provider } from "$lib/types";

export interface SessionSummary {
	provider: Provider;
	id: string;
	cwd: string;
	title: string;
	updated: number;
	path?: string;
	account?: string;
}

export interface ProjectGroup {
	cwd: string;
	name: string;
	updated: number;
	sessions: SessionSummary[];
}

const COLLAPSE_KEY = "glaude.sidebar.collapsed.v1";
/** Display names given in Glaude; the CLI's own transcript is not changed. */
const NAMES_KEY = "glaude.names.v1";

function loadNames(): Record<string, string> {
	try {
		return JSON.parse(localStorage.getItem(NAMES_KEY) ?? "{}") as Record<string, string>;
	} catch {
		return {};
	}
}

class History {
	sessions = $state.raw<SessionSummary[]>([]);
	loading = $state(true);
	error = $state<string | null>(null);
	collapsed = $state<Record<string, boolean>>({});
	/** "claude:<id>" → the name given in Glaude. */
	names = $state<Record<string, string>>(loadNames());
	/** The tool's own titles, for "reset". */
	private original = new Map<string, string>();
	private timer: ReturnType<typeof setTimeout> | null = null;

	constructor() {
		try {
			this.collapsed = JSON.parse(localStorage.getItem(COLLAPSE_KEY) ?? "{}");
		} catch {
			this.collapsed = {};
		}
	}

	toggle(cwd: string) {
		this.collapsed[cwd] = !this.collapsed[cwd];
		localStorage.setItem(COLLAPSE_KEY, JSON.stringify($state.snapshot(this.collapsed)));
	}

	get groups(): ProjectGroup[] {
		const map = new Map<string, ProjectGroup>();
		for (const s of this.sessions) {
			const key = s.cwd || "(unknown folder)";
			let g = map.get(key);
			if (!g) {
				g = { cwd: key, name: key.split(/[\\/]/).filter(Boolean).pop() ?? key, updated: s.updated, sessions: [] };
				map.set(key, g);
			}
			g.sessions.push(s);
			g.updated = Math.max(g.updated, s.updated);
		}
		return [...map.values()].sort((a, b) => b.updated - a.updated);
	}

	/** Claude and Codex lists load independently; Codex starts its
	 * app-server on first use and may take a few seconds. */
	async refresh() {
		const load = async (cmd: string, provider: Provider) => {
			try {
				const list = (await call<SessionSummary[]>(cmd)).map((s) => this.named(s));
				this.sessions = [...this.sessions.filter((s) => s.provider !== provider), ...list].sort(
					(a, b) => b.updated - a.updated,
				);
				if (provider === "claude") this.error = null;
			} catch (e) {
				// Codex may simply not be installed; only Claude errors are shown.
				if (provider === "claude") this.error = e instanceof Error ? e.message : String(e);
			} finally {
				if (provider === "claude") this.loading = false;
			}
		};
		await Promise.all([load("sessions_list", "claude"), load("codex_sessions_list", "codex")]);
	}

	private named(s: SessionSummary): SessionSummary {
		const k = `${s.provider}:${s.id}`;
		this.original.set(k, s.title);
		const n = this.names[k];
		return n ? { ...s, title: n } : s;
	}

	/** The title the CLI gave a session (before any rename). */
	originalTitle(s: { provider: string; id: string }): string | undefined {
		return this.original.get(`${s.provider}:${s.id}`);
	}

	/** Name a session in Glaude; an empty name goes back to the CLI's title. */
	rename(s: { provider: string; id: string }, title: string): string {
		const k = `${s.provider}:${s.id}`;
		const name = title.replace(/\s+/g, " ").trim();
		if (name && name !== this.original.get(k)) this.names[k] = name;
		else delete this.names[k];
		localStorage.setItem(NAMES_KEY, JSON.stringify($state.snapshot(this.names)));
		const shown = this.names[k] ?? this.original.get(k) ?? name;
		this.sessions = this.sessions.map((x) => (x.provider === s.provider && x.id === s.id ? { ...x, title: shown } : x));
		return shown;
	}

	/** Refresh soon, coalescing bursts (e.g. several turns ending). */
	refreshSoon(ms = 1500) {
		if (this.timer) clearTimeout(this.timer);
		this.timer = setTimeout(() => this.refresh(), ms);
	}
}

export const history = new History();

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

class History {
	sessions = $state.raw<SessionSummary[]>([]);
	loading = $state(true);
	error = $state<string | null>(null);
	collapsed = $state<Record<string, boolean>>({});
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
				const list = await call<SessionSummary[]>(cmd);
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

	/** Refresh soon, coalescing bursts (e.g. several turns ending). */
	refreshSoon(ms = 1500) {
		if (this.timer) clearTimeout(this.timer);
		this.timer = setTimeout(() => this.refresh(), ms);
	}
}

export const history = new History();

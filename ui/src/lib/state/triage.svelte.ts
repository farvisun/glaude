// Sidebar triage of past sessions: pin, snooze until a time, archive; and
// how often and how recently each was opened (frecency) for Cmd+K.

const KEY = "glaude.triage.v1";

interface Saved {
	pinned: Record<string, number>;
	snoozed: Record<string, number>;
	archived: Record<string, number>;
	visits: Record<string, { n: number; last: number }>;
}

export const sessionKey = (s: { provider: string; id: string }) => `${s.provider}:${s.id}`;

class Triage {
	pinned = $state<Record<string, number>>({});
	snoozed = $state<Record<string, number>>({});
	archived = $state<Record<string, number>>({});
	visits = $state<Record<string, { n: number; last: number }>>({});
	/** Advances every minute so snoozed sessions come back on time. */
	now = $state(Date.now());

	constructor() {
		try {
			const s = JSON.parse(localStorage.getItem(KEY) ?? "null") as Partial<Saved> | null;
			this.pinned = s?.pinned ?? {};
			this.snoozed = s?.snoozed ?? {};
			this.archived = s?.archived ?? {};
			this.visits = s?.visits ?? {};
		} catch {
			/* start empty */
		}
		if (typeof window !== "undefined") setInterval(() => (this.now = Date.now()), 60_000);
	}

	private save() {
		const data: Saved = {
			pinned: $state.snapshot(this.pinned),
			snoozed: $state.snapshot(this.snoozed),
			archived: $state.snapshot(this.archived),
			visits: $state.snapshot(this.visits),
		};
		localStorage.setItem(KEY, JSON.stringify(data));
	}

	isPinned = (k: string) => k in this.pinned;
	isArchived = (k: string) => k in this.archived;
	snoozedUntil = (k: string): number | null => {
		const t = this.snoozed[k];
		return t && t > this.now ? t : null;
	};
	/** Out of the main list: archived, or snoozed until later. */
	hidden = (k: string) => this.isArchived(k) || this.snoozedUntil(k) !== null;

	togglePin(k: string) {
		if (k in this.pinned) delete this.pinned[k];
		else this.pinned[k] = Date.now();
		this.save();
	}
	snooze(k: string, until: number) {
		this.snoozed[k] = until;
		delete this.pinned[k];
		this.save();
	}
	wake(k: string) {
		delete this.snoozed[k];
		this.save();
	}
	archive(k: string) {
		this.archived[k] = Date.now();
		delete this.pinned[k];
		delete this.snoozed[k];
		this.save();
	}
	unarchive(k: string) {
		delete this.archived[k];
		this.save();
	}
	/** Restore a session's earlier state (Undo). */
	restore(k: string, before: { pinned?: number; snoozed?: number; archived?: number }) {
		for (const f of ["pinned", "snoozed", "archived"] as const) {
			if (before[f] !== undefined) this[f][k] = before[f];
			else delete this[f][k];
		}
		this.save();
	}
	stateOf(k: string) {
		return { pinned: this.pinned[k], snoozed: this.snoozed[k], archived: this.archived[k] };
	}

	visit(k: string) {
		const v = this.visits[k] ?? { n: 0, last: 0 };
		this.visits[k] = { n: Math.min(v.n + 1, 1000), last: Date.now() };
		// Keep the most recent 500.
		const keys = Object.keys(this.visits);
		if (keys.length > 500) {
			keys.sort((a, b) => this.visits[a].last - this.visits[b].last);
			for (const old of keys.slice(0, keys.length - 500)) delete this.visits[old];
		}
		this.save();
	}

	/** Higher for sessions opened often and recently (and updated recently). */
	frecency(k: string, updated: number): number {
		const v = this.visits[k];
		const last = Math.max(v?.last ?? 0, updated);
		const hours = Math.max(0, (this.now - last) / 3_600_000);
		const recency = hours < 4 ? 100 : hours < 24 ? 70 : hours < 24 * 7 ? 50 : hours < 24 * 30 ? 30 : 10;
		return recency * (1 + Math.log2(1 + (v?.n ?? 0))) + (k in this.pinned ? 1000 : 0);
	}
}

export const triage = new Triage();

/** Snooze choices: label and the time they end. */
export function snoozeOptions(now = new Date()): { label: string; until: number }[] {
	const at = (d: Date, h: number) => {
		const x = new Date(d);
		x.setHours(h, 0, 0, 0);
		return x;
	};
	const tonight = at(now, 19);
	const tomorrow = at(new Date(now.getTime() + 86_400_000), 9);
	// The first Monday after tomorrow.
	const nextWeek = new Date(tomorrow);
	do nextWeek.setDate(nextWeek.getDate() + 1);
	while (nextWeek.getDay() !== 1);
	const opts = [{ label: "1 hour", until: now.getTime() + 3_600_000 }];
	if (tonight.getTime() - now.getTime() > 3_600_000) opts.push({ label: "This evening", until: tonight.getTime() });
	opts.push({ label: "Tomorrow morning", until: tomorrow.getTime() });
	opts.push({ label: "Next week", until: at(nextWeek, 9).getTime() });
	return opts;
}

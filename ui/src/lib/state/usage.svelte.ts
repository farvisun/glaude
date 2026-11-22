// A local log of finished turns (time, cost, tokens, files) for the Usage
// view. Only turns run in Glaude are counted.
import type { Provider, TurnTokens } from "$lib/types";
import type { Conversation, TurnStat } from "./conversations.svelte";

const KEY = "glaude.usage.v1";
const KEEP_DAYS = 120;
const MAX = 8000;

export interface TurnRecord {
	ts: number;
	convId: string;
	title: string;
	cwd: string;
	provider: Provider;
	account: string;
	durationMs?: number;
	costUsd?: number;
	tokens?: TurnTokens;
	files: number;
	error?: boolean;
}

function load(): TurnRecord[] {
	try {
		const all = JSON.parse(localStorage.getItem(KEY) ?? "[]") as TurnRecord[];
		return Array.isArray(all) ? all : [];
	} catch {
		return [];
	}
}

class UsageLog {
	records = $state.raw<TurnRecord[]>(load());

	record(conv: Conversation, stat: TurnStat) {
		const r: TurnRecord = {
			ts: stat.endedAt,
			convId: conv.id,
			title: conv.title,
			cwd: conv.cwd,
			provider: conv.provider,
			account: conv.account || "default",
			durationMs: stat.durationMs,
			costUsd: stat.costUsd,
			tokens: stat.tokens,
			files: stat.files.length,
			error: stat.error ? true : undefined,
		};
		const cutoff = Date.now() - KEEP_DAYS * 86_400_000;
		this.records = [...this.records.filter((x) => x.ts >= cutoff).slice(-(MAX - 1)), r];
		try {
			localStorage.setItem(KEY, JSON.stringify(this.records));
		} catch {
			/* storage full: keep in memory */
		}
	}

	clear() {
		this.records = [];
		localStorage.removeItem(KEY);
	}
}

export const usage = new UsageLog();

export interface Totals {
	turns: number;
	costUsd: number;
	tokens: number;
	output: number;
	durationMs: number;
	files: number;
}

export const tokensOf = (t?: TurnTokens) => (t ? t.input + t.output : 0);

export function totals(rs: TurnRecord[]): Totals {
	const out: Totals = { turns: 0, costUsd: 0, tokens: 0, output: 0, durationMs: 0, files: 0 };
	for (const r of rs) {
		out.turns++;
		out.costUsd += r.costUsd ?? 0;
		out.tokens += tokensOf(r.tokens);
		out.output += r.tokens?.output ?? 0;
		out.durationMs += r.durationMs ?? 0;
		out.files += r.files;
	}
	return out;
}

/** Local calendar day, "2026-11-22". */
export function dayKey(ts: number): string {
	const d = new Date(ts);
	return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

/** The last `n` days (oldest first) with their totals; empty days included. */
export function byDay(rs: TurnRecord[], n: number, now = Date.now()): { day: string; totals: Totals }[] {
	const groups = new Map<string, TurnRecord[]>();
	for (const r of rs) {
		const k = dayKey(r.ts);
		(groups.get(k) ?? groups.set(k, []).get(k)!).push(r);
	}
	const out: { day: string; totals: Totals }[] = [];
	const d = new Date(now);
	d.setHours(12, 0, 0, 0);
	for (let i = n - 1; i >= 0; i--) {
		const k = dayKey(d.getTime() - i * 86_400_000);
		out.push({ day: k, totals: totals(groups.get(k) ?? []) });
	}
	return out;
}

export function groupBy(rs: TurnRecord[], key: (r: TurnRecord) => string): { key: string; records: TurnRecord[]; totals: Totals }[] {
	const groups = new Map<string, TurnRecord[]>();
	for (const r of rs) {
		const k = key(r);
		(groups.get(k) ?? groups.set(k, []).get(k)!).push(r);
	}
	return [...groups.entries()].map(([k, list]) => ({ key: k, records: list, totals: totals(list) }));
}

export function since(rs: TurnRecord[], ms: number, now = Date.now()): TurnRecord[] {
	return rs.filter((r) => r.ts >= now - ms);
}

/** Start of today (local). */
export function startOfToday(now = Date.now()): number {
	const d = new Date(now);
	d.setHours(0, 0, 0, 0);
	return d.getTime();
}

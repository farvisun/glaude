// Consecutive tool calls fold into one row ("Read 6 files · Ran 3 commands"),
// expanded on click. Thinking between the calls stays inside the run.
import type { FileDiff, Item } from "$lib/types";

export type Row = { kind: "item"; id: string; item: Item } | { kind: "run"; id: string; items: Item[] };

const NOT_FOLDED = new Set(["Agent", "Task", "Workflow", "TodoWrite"]);

function foldable(it: Item, hasTask: (id: string) => boolean): boolean {
	if (it.kind === "command" || it.kind === "fileChange") return true;
	return it.kind === "tool" && !it.parent && !NOT_FOLDED.has(it.name) && !hasTask(it.id);
}

export function groupRuns(items: Item[], opts: { compact: boolean; hasTask: (id: string) => boolean; min?: number }): Row[] {
	const min = opts.min ?? 2;
	const rows: Row[] = [];
	if (!opts.compact) return items.map((item) => ({ kind: "item", id: item.id, item }));
	let buf: Item[] = [];
	let tools = 0;
	const flush = () => {
		const tail: Item[] = [];
		while (buf.length && buf[buf.length - 1].kind === "reasoning") tail.unshift(buf.pop()!);
		if (tools >= min) rows.push({ kind: "run", id: `run:${buf[0].id}`, items: buf });
		else for (const item of buf) rows.push({ kind: "item", id: item.id, item });
		for (const item of tail) rows.push({ kind: "item", id: item.id, item });
		buf = [];
		tools = 0;
	};
	for (const it of items) {
		if (foldable(it, opts.hasTask)) {
			buf.push(it);
			tools++;
		} else if (it.kind === "reasoning" && buf.length) buf.push(it);
		else {
			flush();
			rows.push({ kind: "item", id: it.id, item: it });
		}
	}
	flush();
	return rows;
}

export const lastId = (row: Row) => (row.kind === "run" ? row.items[row.items.length - 1].id : row.id);

type Kind = "read" | "edit" | "run" | "search" | "web" | "other";
function kindOf(it: Item): Kind | null {
	if (it.kind === "command") return "run";
	if (it.kind === "fileChange") return "edit";
	if (it.kind !== "tool") return null;
	switch (it.name) {
		case "Read":
		case "NotebookRead":
			return "read";
		case "Edit":
		case "MultiEdit":
		case "Write":
		case "NotebookEdit":
			return "edit";
		case "Bash":
		case "BashOutput":
			return "run";
		case "Grep":
		case "Glob":
		case "LS":
			return "search";
		case "WebFetch":
		case "WebSearch":
			return "web";
	}
	return "other";
}

/** "Read 3 files · Edited 2 files · Ran 4 commands". */
export function runSummary(items: Item[]): string {
	const n: Record<Kind, number> = { read: 0, edit: 0, run: 0, search: 0, web: 0, other: 0 };
	const reads = new Set<string>();
	for (const it of items) {
		const k = kindOf(it);
		if (!k) continue;
		if (k === "read" && it.kind === "tool") reads.add(it.title);
		else n[k]++;
	}
	n.read = reads.size;
	const edits = new Set(editedFiles(items).map((f) => f.path)).size;
	const plural = (c: number, one: string, many: string) => `${c} ${c === 1 ? one : many}`;
	const parts = [
		n.read && `Read ${plural(n.read, "file", "files")}`,
		edits && `Edited ${plural(edits, "file", "files")}`,
		n.run && `Ran ${plural(n.run, "command", "commands")}`,
		n.search && `Searched ${n.search === 1 ? "once" : `${n.search} times`}`,
		n.web && `Visited the web ${n.web === 1 ? "once" : `${n.web} times`}`,
		n.other && plural(n.other, "other tool", "other tools"),
	].filter(Boolean);
	return parts.join(" · ") || `${items.length} steps`;
}

/** Files edited in these items with lines added and removed. */
export function editedFiles(items: Item[]): { path: string; added: number; removed: number }[] {
	const out = new Map<string, { path: string; added: number; removed: number }>();
	const add = (d: FileDiff) => {
		const e = out.get(d.path) ?? { path: d.path, added: 0, removed: 0 };
		e.added += d.added;
		e.removed += d.removed;
		out.set(d.path, e);
	};
	for (const it of items) {
		if (it.kind === "fileChange") it.diffs.forEach(add);
		else if (it.kind === "tool" && kindOf(it) === "edit" && it.status !== "declined" && it.status !== "error") {
			if (it.diffs?.length) it.diffs.forEach(add);
			else if (it.title && !out.has(it.title)) out.set(it.title, { path: it.title, added: 0, removed: 0 });
		}
	}
	return [...out.values()];
}

/** Failed calls: errors and commands that exited non-zero. */
export function failures(items: Item[]): number {
	let n = 0;
	for (const it of items) {
		if ((it.kind === "tool" || it.kind === "command" || it.kind === "fileChange") && it.status === "error") n++;
		else if (it.kind === "command" && it.exitCode !== undefined && it.exitCode !== 0) n++;
	}
	return n;
}

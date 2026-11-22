// Find in a conversation (⌘F): which items match, and highlighting the
// matches on screen with the CSS Custom Highlight API (no DOM changes).
import type { Item } from "$lib/types";
import { plainText } from "./activity";

/** Case and Persian-letter insensitive (ي/ك, ZWNJ), same length as the input. */
export function norm(s: string): string {
	return s.toLowerCase().replace(/‌/g, " ").replace(/ي/g, "ی").replace(/ك/g, "ک");
}

export function itemText(it: Item): string {
	switch (it.kind) {
		case "user":
		case "assistant":
			return plainText(it.blocks);
		case "tool":
			return `${it.name} ${it.title}`;
		case "command":
			return it.command;
		case "fileChange":
			return it.diffs.map((d) => d.path).join(" ");
		case "plan":
			return it.steps.map((s) => s.text).join(" ");
		case "system":
			return it.text;
		case "permission":
			return `${it.tool} ${it.title}`;
		default:
			return "";
	}
}

/** Ids of the items containing the query, in order. */
export function findMatches(items: Item[], query: string): string[] {
	const q = norm(query.trim());
	if (!q) return [];
	return items.filter((it) => norm(itemText(it)).includes(q)).map((it) => it.id);
}

type HighlightRegistry = Map<string, unknown>;
const registry = (): HighlightRegistry | null =>
	typeof CSS !== "undefined" && "highlights" in CSS ? (CSS as unknown as { highlights: HighlightRegistry }).highlights : null;

function rangesIn(root: Element, q: string, limit: number): Range[] {
	const out: Range[] = [];
	const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
	for (let n = walker.nextNode(); n && out.length < limit; n = walker.nextNode()) {
		const text = norm(n.nodeValue ?? "");
		for (let at = text.indexOf(q); at >= 0 && out.length < limit; at = text.indexOf(q, at + q.length)) {
			const r = document.createRange();
			r.setStart(n, at);
			r.setEnd(n, at + q.length);
			out.push(r);
		}
	}
	return out;
}

/** Highlight every match under `root`, and the current item's more strongly. */
export function highlight(root: Element | null, query: string, current: Element | null) {
	const reg = registry();
	if (!reg) return;
	const q = norm(query.trim());
	if (!root || !q) {
		reg.delete("find");
		reg.delete("find-current");
		return;
	}
	const H = (globalThis as unknown as { Highlight: new (...r: Range[]) => unknown }).Highlight;
	reg.set("find", new H(...rangesIn(root, q, 2000)));
	if (current) reg.set("find-current", new H(...rangesIn(current, q, 200)));
	else reg.delete("find-current");
}

// Paths in replies become links: inline code like `src/a.ts:12`, relative
// markdown links and file:// links that exist on disk get `data-path`.
import { call } from "$lib/ipc";
import type { Editor } from "./prefs.svelte";

export interface PathHit {
	abs: string;
	dir: boolean;
	line?: number;
	col?: number;
}

export const editorNames: Record<Editor, string> = {
	default: "Default app",
	vscode: "VS Code",
	cursor: "Cursor",
	zed: "Zed",
};

// A path: optional ~/ ./ ../ or /, segments, an optional :line[:col] or #L12.
const PATHISH = /^(~\/|\.{1,2}\/|\/)?[\w.@+\- ]*[\w@+\-]([\\/][\w.@+\- ]*[\w@+\-])*\/?((:\d+){1,2}|#L\d+)?$/;
const HAS_EXT = /\.[A-Za-z][\w]{0,9}((:\d+){1,2}|#L\d+)?$/;

/** Text that could be a path (cheap check before asking the backend). */
export function looksLikePath(text: string): boolean {
	const t = text.trim();
	if (t.length < 2 || t.length > 400 || /\s{2}|\n/.test(t)) return false;
	if (/^[a-z]+:\/\//i.test(t)) return t.startsWith("file://");
	if (!PATHISH.test(t)) return false;
	// Needs a folder separator or a file extension: not every word is a path.
	return t.includes("/") || HAS_EXT.test(t);
}

const cache = new Map<string, PathHit | null>();

function candidateOf(el: Element): string | null {
	if (el.tagName === "A") {
		const href = el.getAttribute("href") ?? "";
		if (!href || /^(https?:|mailto:|#)/i.test(href)) return null;
		try {
			return href.startsWith("file://") ? href : decodeURIComponent(href);
		} catch {
			return href;
		}
	}
	if (el.closest("pre")) return null;
	const t = el.textContent ?? "";
	return looksLikePath(t) ? t.trim() : null;
}

/** Mark the paths under `root` that exist. */
export async function markPaths(root: HTMLElement, cwd: string) {
	const els = Array.from(root.querySelectorAll<HTMLElement>("code:not([data-scanned]), a[href]:not([data-scanned])"));
	const want: { el: HTMLElement; c: string }[] = [];
	for (const el of els) {
		el.dataset.scanned = "1";
		const c = candidateOf(el);
		if (c) want.push({ el, c });
	}
	if (!want.length) return;
	const ask = [...new Set(want.map((w) => w.c).filter((c) => !cache.has(`${cwd}\0${c}`)))];
	if (ask.length) {
		try {
			const hits = await call<(PathHit | null)[]>("paths_check", { cwd, candidates: ask });
			ask.forEach((c, i) => cache.set(`${cwd}\0${c}`, hits[i] ?? null));
		} catch {
			return;
		}
	}
	for (const { el, c } of want) {
		const hit = cache.get(`${cwd}\0${c}`);
		if (!hit) continue;
		el.dataset.path = hit.abs;
		if (hit.dir) el.dataset.dir = "1";
		if (hit.line) el.dataset.line = String(hit.line);
		if (hit.col) el.dataset.col = String(hit.col);
		el.classList.add("path-link");
		el.title = `${hit.abs}${hit.line ? `:${hit.line}` : ""} · click for options, ${navigator.platform.includes("Mac") ? "⌘" : "Ctrl+"}click to open`;
	}
}

export function hitOf(el: HTMLElement): PathHit | null {
	const abs = el.dataset.path;
	if (!abs) return null;
	return {
		abs,
		dir: el.dataset.dir === "1",
		line: el.dataset.line ? Number(el.dataset.line) : undefined,
		col: el.dataset.col ? Number(el.dataset.col) : undefined,
	};
}

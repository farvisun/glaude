// Saved prompts: reusable messages with fill-in parts like {{file}}, for
// every project or one. Used from the composer's / menu and the command
// palette; {{cwd}}, {{project}}, {{branch}} and {{date}} fill themselves.
import { uid } from "./layout.svelte";

const KEY = "glaude.prompts.v1";

export interface SavedPrompt {
	id: string;
	name: string;
	body: string;
	/** Project folder it belongs to; null = every project. */
	cwd: string | null;
	updated: number;
}

export { BUILT_INS, slugOf, variablesOf, fillPrompt } from "./prompt-text";

function load(): SavedPrompt[] {
	try {
		const v = JSON.parse(localStorage.getItem(KEY) ?? "null");
		if (Array.isArray(v)) return v as SavedPrompt[];
	} catch {
		/* corrupt: start over */
	}
	return [
		{
			id: "p-review",
			name: "Review changes",
			body: "Review the uncommitted changes in {{project}} for bugs, missed edge cases and risky changes. Report file:line and a fix for each, most severe first.",
			cwd: null,
			updated: 0,
		},
		{
			id: "p-explain",
			name: "Explain",
			body: "Explain how {{topic}} works in this codebase: the files involved, the flow, and anything surprising.",
			cwd: null,
			updated: 0,
		},
	];
}

class Prompts {
	list = $state<SavedPrompt[]>(load());

	private save() {
		localStorage.setItem(KEY, JSON.stringify(this.list));
	}

	/** Prompts for a project: its own first, then the shared ones. */
	forCwd(cwd: string | null | undefined): SavedPrompt[] {
		const mine = this.list.filter((p) => p.cwd && cwd && p.cwd === cwd);
		const shared = this.list.filter((p) => !p.cwd);
		return [...mine, ...shared].sort((a, b) => a.name.localeCompare(b.name));
	}

	upsert(p: Omit<SavedPrompt, "id" | "updated"> & { id?: string }) {
		const next: SavedPrompt = { ...p, id: p.id ?? uid("prompt"), updated: Date.now() };
		const i = this.list.findIndex((x) => x.id === next.id);
		this.list = i >= 0 ? this.list.map((x) => (x.id === next.id ? next : x)) : [...this.list, next];
		this.save();
		return next;
	}

	remove(id: string) {
		this.list = this.list.filter((p) => p.id !== id);
		this.save();
	}
}

export const prompts = new Prompts();

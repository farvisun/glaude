// Stashed composer drafts (Cmd/Ctrl+S), kept across restarts.
import type { Attachment } from "$lib/types";

export interface Draft {
	id: string;
	text: string;
	attachments: Attachment[];
	at: number;
}

const KEY = "glaude.stash.v1";
const MAX = 20;

class Stash {
	items = $state<Draft[]>([]);

	constructor() {
		try {
			this.items = JSON.parse(localStorage.getItem(KEY) ?? "[]") as Draft[];
		} catch {
			this.items = [];
		}
	}

	private save() {
		localStorage.setItem(KEY, JSON.stringify(this.items));
	}

	add(text: string, attachments: Attachment[]) {
		const d = { id: `d${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`, text, attachments, at: Date.now() };
		this.items = [d, ...this.items].slice(0, MAX);
		this.save();
	}

	take(id: string): Draft | undefined {
		const d = this.items.find((x) => x.id === id);
		this.items = this.items.filter((x) => x.id !== id);
		this.save();
		return d;
	}
}

export const stash = new Stash();

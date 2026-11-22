// The terminal panel at the bottom of the window (VS Code style): groups of
// terminals, each group one or more terminals side by side. The xterm
// instances and their shells live in terminals.ts, so a terminal keeps
// running while the panel is hidden.
import type { TermSpec } from "./layout.svelte";
import { uid } from "./layout.svelte";
import { dispose, focus, onTitle } from "./terminals";
import { tick } from "svelte";

const KEY = "glaude.termpanel.v1";

export interface TermGroup {
	id: string;
	terms: string[];
}

interface Saved {
	open: boolean;
	size: number;
	maximized: boolean;
}

function loadSaved(): Saved {
	try {
		return { open: false, size: 42, maximized: false, ...(JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<Saved>) };
	} catch {
		return { open: false, size: 42, maximized: false };
	}
}

class TermPanel {
	open = $state(false);
	/** Height in percent of the main area. */
	size = $state(42);
	maximized = $state(false);
	groups = $state<TermGroup[]>([]);
	active = $state<string | null>(null);
	focused = $state<string | null>(null);
	specs = $state.raw<Record<string, TermSpec>>({});
	titles = $state<Record<string, string>>({});
	/** Set by the app: the folder and proxy for a new terminal. */
	defaults: () => TermSpec = () => ({});

	/** Was open when the app last closed (restored with a fresh shell). */
	private wasOpen = false;

	constructor() {
		const s = loadSaved();
		this.wasOpen = s.open;
		this.size = s.size;
		this.maximized = s.maximized;
		onTitle((id, title) => {
			if (this.specs[id]) this.titles[id] = title;
		});
	}

	restore() {
		if (this.wasOpen && !this.groups.length) this.newTerminal();
		this.wasOpen = false;
	}

	save() {
		localStorage.setItem(KEY, JSON.stringify({ open: this.open, size: this.size, maximized: this.maximized }));
	}

	get count(): number {
		return this.groups.reduce((n, g) => n + g.terms.length, 0);
	}

	groupOf(termId: string): TermGroup | undefined {
		return this.groups.find((g) => g.terms.includes(termId));
	}

	private add(spec: TermSpec): string {
		const id = uid("pterm");
		this.specs = { ...this.specs, [id]: spec };
		return id;
	}

	/** A new terminal in its own group (⌘T, the + button). */
	newTerminal(spec?: TermSpec) {
		const id = this.add(spec ?? this.defaults());
		const g = { id: uid("tgroup"), terms: [id] };
		this.groups = [...this.groups, g];
		this.active = g.id;
		this.show(id);
	}

	/** Another terminal beside the focused one, same folder. */
	split() {
		const g = this.groups.find((x) => x.id === this.active);
		if (!g) return this.newTerminal();
		const base = (this.focused && this.specs[this.focused]) || this.defaults();
		const id = this.add({ cwd: base.cwd, proxy: base.proxy });
		this.groups = this.groups.map((x) => (x.id === g.id ? { ...x, terms: [...x.terms, id] } : x));
		this.show(id);
	}

	/** End a terminal's shell and remove it. */
	kill(termId: string | null = this.focused) {
		if (!termId) return;
		dispose(termId);
		const { [termId]: _gone, ...rest } = this.specs;
		this.specs = rest;
		delete this.titles[termId];
		this.groups = this.groups.map((g) => ({ ...g, terms: g.terms.filter((t) => t !== termId) })).filter((g) => g.terms.length);
		if (!this.groups.some((g) => g.id === this.active)) this.active = this.groups[this.groups.length - 1]?.id ?? null;
		const g = this.groups.find((x) => x.id === this.active);
		this.focused = g?.terms[g.terms.length - 1] ?? null;
		if (!this.groups.length) this.hide();
		else if (this.focused) void tick().then(() => focus(this.focused!));
	}

	activate(groupId: string, termId?: string) {
		const g = this.groups.find((x) => x.id === groupId);
		if (!g) return;
		this.active = g.id;
		this.focused = termId ?? g.terms[0];
		void tick().then(() => this.focused && focus(this.focused));
	}

	show(termId?: string) {
		this.open = true;
		if (termId) this.focused = termId;
		this.save();
		void tick().then(() => this.focused && focus(this.focused));
	}

	hide() {
		this.open = false;
		this.save();
	}

	/** Ctrl+`: show (starting a terminal if there is none) or hide. */
	toggle() {
		if (this.open) return this.hide();
		if (!this.groups.length) this.newTerminal();
		else this.show();
	}

	toggleMaximize() {
		this.maximized = !this.maximized;
		this.save();
	}
}

export const termPanel = new TermPanel();

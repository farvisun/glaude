// Pane tree + tabs. A binary split tree of at most MAX_PANES leaves; each
// leaf has its own tab strip. Persisted to localStorage so the window comes
// back the way it was left.

export type TabKind = "welcome" | "conversation" | "terminal" | "accounts" | "settings" | "search";

/** What a terminal tab runs: a login shell, or a CLI's own TUI. */
export interface TermSpec {
	cwd?: string;
	run?: { provider: "claude" | "codex"; account: string; resume?: string; login?: boolean };
}

export interface Tab {
	id: string;
	kind: TabKind;
	title: string;
	/** conversation id or terminal id, depending on kind */
	ref?: string;
	term?: TermSpec;
}

export interface Leaf {
	type: "leaf";
	id: string;
	tabs: string[];
	active: string | null;
}

export interface Split {
	type: "split";
	id: string;
	dir: "horizontal" | "vertical";
	a: PaneNode;
	b: PaneNode;
}

export type PaneNode = Leaf | Split;

export type Edge = "left" | "right" | "top" | "bottom";

const KEY = "glaude.layout.v1";
const SIDEBAR_KEY = "glaude.sidebar.v1";
export const MAX_PANES = 8;

let seq = 0;
export function uid(prefix: string): string {
	seq += 1;
	return `${prefix}-${Date.now().toString(36)}-${seq.toString(36)}`;
}

function leaf(tabs: string[] = [], active: string | null = null): Leaf {
	return { type: "leaf", id: uid("pane"), tabs, active };
}

interface Saved {
	root: PaneNode;
	tabs: Record<string, Tab>;
	focused: string;
}

interface SidebarState {
	size: number;
	collapsed: boolean;
}

function loadJSON<T>(key: string): T | null {
	try {
		const raw = localStorage.getItem(key);
		return raw ? (JSON.parse(raw) as T) : null;
	} catch {
		return null;
	}
}

class Layout {
	root = $state<PaneNode>(leaf());
	tabs = $state<Record<string, Tab>>({});
	focused = $state<string>("");
	sidebar = $state<SidebarState>({ size: 20, collapsed: false });
	/** Called when a tab is closed, so owners can release resources. */
	onClose: ((tab: Tab) => void) | null = null;

	constructor() {
		const saved = loadJSON<Saved>(KEY);
		if (saved?.root && saved.tabs) {
			this.root = saved.root;
			this.tabs = saved.tabs;
			this.focused = saved.focused;
		}
		const sb = loadJSON<SidebarState>(SIDEBAR_KEY);
		if (sb) this.sidebar = sb;
		if (this.leaves().length === 0) this.root = leaf();
		if (!this.findLeaf(this.focused)) this.focused = this.leaves()[0].id;
		const first = this.leaves()[0];
		if (Object.keys(this.tabs).length === 0) this.openTab({ kind: "welcome", title: "Start" }, first.id);
	}

	save() {
		const data: Saved = { root: $state.snapshot(this.root), tabs: $state.snapshot(this.tabs), focused: this.focused };
		localStorage.setItem(KEY, JSON.stringify(data));
	}

	saveSidebar() {
		localStorage.setItem(SIDEBAR_KEY, JSON.stringify($state.snapshot(this.sidebar)));
	}

	leaves(node: PaneNode = this.root): Leaf[] {
		return node.type === "leaf" ? [node] : [...this.leaves(node.a), ...this.leaves(node.b)];
	}

	findLeaf(id: string, node: PaneNode = this.root): Leaf | null {
		if (node.type === "leaf") return node.id === id ? node : null;
		return this.findLeaf(id, node.a) ?? this.findLeaf(id, node.b);
	}

	leafOfTab(tabId: string): Leaf | null {
		return this.leaves().find((l) => l.tabs.includes(tabId)) ?? null;
	}

	get focusedLeaf(): Leaf {
		return this.findLeaf(this.focused) ?? this.leaves()[0];
	}

	focus(paneId: string) {
		if (this.focused !== paneId) {
			this.focused = paneId;
			this.save();
		}
	}

	/** Open a tab, or focus an existing one with the same kind+ref. */
	openTab(spec: Omit<Tab, "id">, paneId?: string): string {
		const existing = Object.values(this.tabs).find(
			(t) => t.kind === spec.kind && (t.ref ?? null) === (spec.ref ?? null) && spec.kind !== "terminal",
		);
		if (existing) {
			this.activate(existing.id);
			return existing.id;
		}
		const id = uid("tab");
		this.tabs[id] = { id, ...spec };
		const target = (paneId && this.findLeaf(paneId)) || this.focusedLeaf;
		// A lone welcome tab gets replaced by the first real tab.
		const welcome = target.tabs.find((t) => this.tabs[t]?.kind === "welcome");
		target.tabs.push(id);
		target.active = id;
		this.focused = target.id;
		if (welcome && spec.kind !== "welcome") this.closeTab(welcome);
		this.save();
		return id;
	}

	activate(tabId: string) {
		const l = this.leafOfTab(tabId);
		if (!l) return;
		l.active = tabId;
		this.focused = l.id;
		this.save();
	}

	rename(tabId: string, title: string) {
		const t = this.tabs[tabId];
		if (t && t.title !== title) {
			t.title = title;
			this.save();
		}
	}

	closeTab(tabId: string) {
		const l = this.leafOfTab(tabId);
		const closing = this.tabs[tabId];
		if (closing) this.onClose?.($state.snapshot(closing) as Tab);
		delete this.tabs[tabId];
		if (!l) return this.save();
		const idx = l.tabs.indexOf(tabId);
		l.tabs.splice(idx, 1);
		if (l.active === tabId) l.active = l.tabs[Math.min(idx, l.tabs.length - 1)] ?? null;
		if (l.tabs.length === 0) {
			if (this.leaves().length > 1) this.removeLeaf(l.id);
			else this.openTab({ kind: "welcome", title: "Start" }, l.id);
		}
		this.save();
	}

	moveTab(tabId: string, toPane: string, index?: number) {
		const from = this.leafOfTab(tabId);
		const to = this.findLeaf(toPane);
		if (!from || !to) return;
		from.tabs.splice(from.tabs.indexOf(tabId), 1);
		if (from.active === tabId) from.active = from.tabs[0] ?? null;
		const at = index === undefined ? to.tabs.length : Math.min(index, to.tabs.length);
		to.tabs.splice(at, 0, tabId);
		to.active = tabId;
		this.focused = to.id;
		if (from.tabs.length === 0 && from.id !== to.id) this.removeLeaf(from.id);
		this.save();
	}

	/** Split `paneId` at `edge` and move `tabId` into the new pane. */
	splitWith(paneId: string, edge: Edge, tabId: string): boolean {
		if (this.leaves().length >= MAX_PANES) return false;
		const target = this.findLeaf(paneId);
		const from = this.leafOfTab(tabId);
		if (!target || !from) return false;
		if (from.id === target.id && from.tabs.length === 1) return false;
		from.tabs.splice(from.tabs.indexOf(tabId), 1);
		if (from.active === tabId) from.active = from.tabs[0] ?? null;
		const fresh = leaf([tabId], tabId);
		const dir: Split["dir"] = edge === "left" || edge === "right" ? "horizontal" : "vertical";
		const copy: Leaf = { ...target, tabs: [...target.tabs] };
		const split: Split = {
			type: "split",
			id: uid("split"),
			dir,
			a: edge === "left" || edge === "top" ? fresh : copy,
			b: edge === "left" || edge === "top" ? copy : fresh,
		};
		this.replace(target.id, split);
		this.focused = fresh.id;
		if (from.tabs.length === 0 && from.id !== target.id) this.removeLeaf(from.id);
		this.save();
		return true;
	}

	/** Split `paneId` at `edge` into a new pane holding a Start tab; the
	 *  next tab opened there replaces it. Returns the new pane's id. */
	splitNew(paneId: string, edge: Edge): string | null {
		if (this.leaves().length >= MAX_PANES) return null;
		const target = this.findLeaf(paneId);
		if (!target) return null;
		const tabId = uid("tab");
		this.tabs[tabId] = { id: tabId, kind: "welcome", title: "Start" };
		const fresh = leaf([tabId], tabId);
		const dir: Split["dir"] = edge === "left" || edge === "right" ? "horizontal" : "vertical";
		const copy: Leaf = { ...target, tabs: [...target.tabs] };
		const first = edge === "left" || edge === "top";
		this.replace(target.id, { type: "split", id: uid("split"), dir, a: first ? fresh : copy, b: first ? copy : fresh });
		this.focused = fresh.id;
		this.save();
		return fresh.id;
	}

	private replace(id: string, next: PaneNode) {
		const walk = (n: PaneNode): PaneNode => {
			if (n.id === id) return next;
			if (n.type === "split") return { ...n, a: walk(n.a), b: walk(n.b) };
			return n;
		};
		this.root = walk(this.root);
	}

	private removeLeaf(id: string) {
		const walk = (n: PaneNode): PaneNode => {
			if (n.type === "leaf") return n;
			if (n.a.id === id) return n.b;
			if (n.b.id === id) return n.a;
			return { ...n, a: walk(n.a), b: walk(n.b) };
		};
		this.root = walk(this.root);
		if (!this.findLeaf(this.focused)) this.focused = this.leaves()[0].id;
	}
}

export const layout = new Layout();

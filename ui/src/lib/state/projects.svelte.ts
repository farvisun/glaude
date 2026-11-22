// Projects the user creates in the sidebar: a name, one or more folders, a
// colour or icon, and defaults for new conversations. Sessions are listed
// under the project whose folder holds them (the closest one); the rest go
// to "Other folders". Kept in localStorage.
import type { Provider } from "$lib/types";
import { uid } from "./layout.svelte";

const KEY = "glaude.projects.v1";

export interface ProjectDefaults {
	provider?: Provider;
	account?: string;
	model?: string;
	effort?: string;
	mode?: string;
}

export interface Project {
	id: string;
	name: string;
	/** The first folder is where new conversations start. */
	folders: string[];
	color?: string;
	icon?: string;
	defaults: ProjectDefaults;
	created: number;
}

export type ProjectSort = "manual" | "recent" | "name";
export type SessionSort = "recent" | "created" | "name";

/** Colours that read on light and dark grounds. */
export const PROJECT_COLORS: { id: string; name: string; value: string }[] = [
	{ id: "gray", name: "Gray", value: "#8b8f9a" },
	{ id: "red", name: "Red", value: "#e5484d" },
	{ id: "orange", name: "Orange", value: "#f76b15" },
	{ id: "amber", name: "Amber", value: "#e2a336" },
	{ id: "green", name: "Green", value: "#30a46c" },
	{ id: "teal", name: "Teal", value: "#12a594" },
	{ id: "blue", name: "Blue", value: "#3e7bfa" },
	{ id: "violet", name: "Violet", value: "#8e4ec6" },
	{ id: "pink", name: "Pink", value: "#d6409f" },
];

export const colorOf = (p: Pick<Project, "color"> | null | undefined) => PROJECT_COLORS.find((c) => c.id === p?.color)?.value;

interface Saved {
	list: Project[];
	sort: ProjectSort;
	sessionSort: SessionSort;
}

function load(): Saved {
	try {
		const s = JSON.parse(localStorage.getItem(KEY) ?? "null") as Partial<Saved> | null;
		return { list: s?.list ?? [], sort: s?.sort ?? "manual", sessionSort: s?.sessionSort ?? "recent" };
	} catch {
		return { list: [], sort: "manual", sessionSort: "recent" };
	}
}

const under = (cwd: string, dir: string) => cwd === dir || cwd.startsWith(dir.endsWith("/") ? dir : dir + "/");

class Projects {
	list = $state<Project[]>([]);
	sort = $state<ProjectSort>("manual");
	sessionSort = $state<SessionSort>("recent");

	constructor() {
		const s = load();
		this.list = s.list;
		this.sort = s.sort;
		this.sessionSort = s.sessionSort;
	}

	save() {
		localStorage.setItem(KEY, JSON.stringify({ list: $state.snapshot(this.list), sort: this.sort, sessionSort: this.sessionSort }));
	}

	get(id: string): Project | undefined {
		return this.list.find((p) => p.id === id);
	}

	/** The project holding `cwd`: the one with the closest folder. */
	of(cwd: string | undefined): Project | null {
		if (!cwd) return null;
		let best: Project | null = null;
		let len = -1;
		for (const p of this.list)
			for (const f of p.folders)
				if (under(cwd, f) && f.length > len) {
					best = p;
					len = f.length;
				}
		return best;
	}

	add(p: Omit<Project, "id" | "created"> & { id?: string }): Project {
		const project: Project = { ...p, id: p.id ?? uid("project"), created: Date.now() };
		this.list = [...this.list, project];
		this.save();
		return project;
	}

	update(id: string, patch: Partial<Omit<Project, "id" | "created">>) {
		this.list = this.list.map((p) => (p.id === id ? { ...p, ...patch } : p));
		this.save();
	}

	remove(id: string): Project | undefined {
		const gone = this.get(id);
		this.list = this.list.filter((p) => p.id !== id);
		this.save();
		return gone;
	}

	/** Put a removed project back where it was (Undo). */
	restore(p: Project, index: number) {
		const list = this.list.filter((x) => x.id !== p.id);
		list.splice(Math.min(index, list.length), 0, p);
		this.list = list;
		this.save();
	}

	/** Manual order: move `id` to position `to`. */
	move(id: string, to: number) {
		const from = this.list.findIndex((p) => p.id === id);
		if (from < 0) return;
		const list = [...this.list];
		const [p] = list.splice(from, 1);
		list.splice(to > from ? to - 1 : to, 0, p);
		this.list = list;
		this.sort = "manual";
		this.save();
	}

	setSort(s: ProjectSort) {
		this.sort = s;
		this.save();
	}

	setSessionSort(s: SessionSort) {
		this.sessionSort = s;
		this.save();
	}
}

export const projects = new Projects();

/** Order sessions by the chosen sort. */
export function sortSessions<T extends { title: string; updated: number; created?: number }>(list: T[], by: SessionSort): T[] {
	const out = [...list];
	if (by === "name") out.sort((a, b) => a.title.localeCompare(b.title, undefined, { sensitivity: "base" }));
	else if (by === "created") out.sort((a, b) => (b.created ?? b.updated) - (a.created ?? a.updated));
	else out.sort((a, b) => b.updated - a.updated);
	return out;
}

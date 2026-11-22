// Opening paths from replies: editor, default app, file manager, terminal,
// or the read-only viewer tab.
import { toast } from "svelte-sonner";
import { call } from "$lib/ipc";
import { basename } from "$lib/format";
import { layout } from "./layout.svelte";
import { prefs } from "./prefs.svelte";
import { ui } from "./ui.svelte";
import { editorNames, type PathHit } from "./paths";

const ua = typeof navigator !== "undefined" ? navigator.userAgent : "";
export const fileManager = /Mac/i.test(ua) ? "Finder" : /Windows/i.test(ua) ? "Explorer" : "Files";

function parentOf(p: string): string {
	const i = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
	return i > 0 ? p.slice(0, i) : p;
}

async function open(hit: PathHit, how: "editor" | "default" | "reveal") {
	try {
		const done = await call<string>("open_path", { path: hit.abs, how, editor: prefs.editor, line: hit.line ?? null, col: hit.col ?? null });
		if (done === "revealed") toast(`Shown in ${fileManager}`, { description: "Programs are never started from a link." });
	} catch (e) {
		toast.error("Could not open it", { description: e instanceof Error ? e.message : String(e) });
	}
}

export const pathActions = {
	/** The main action: files in the chosen editor (or the viewer), folders in the file manager. */
	primary(hit: PathHit) {
		if (hit.dir) return open(hit, "default");
		if (prefs.editor === "default") return pathActions.view(hit);
		return open(hit, "editor");
	},
	editor: (hit: PathHit) => open(hit, "editor"),
	system: (hit: PathHit) => open(hit, "default"),
	reveal: (hit: PathHit) => open(hit, "reveal"),
	view(hit: PathHit) {
		layout.openTab({ kind: "file", title: basename(hit.abs), ref: hit.abs, line: hit.line });
	},
	terminal(hit: PathHit) {
		const dir = hit.dir ? hit.abs : parentOf(hit.abs);
		ui.openTerminal({ cwd: dir }, basename(dir) || "Terminal");
	},
	async copy(hit: PathHit) {
		await navigator.clipboard.writeText(hit.line ? `${hit.abs}:${hit.line}` : hit.abs);
		toast("Path copied");
	},
	editorLabel: () => editorNames[prefs.editor],
};

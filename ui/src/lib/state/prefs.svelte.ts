// UI preferences kept in localStorage.

export type Palette = "lajevard" | "dood";
/** Where file links from replies open. */
export type Editor = "default" | "vscode" | "cursor" | "zed";

const KEY = "glaude.prefs.v1";

interface Prefs {
	palette: Palette;
	progressPanel: boolean;
	statuslineScript: boolean;
	editor: Editor;
}

class PrefStore {
	palette = $state<Palette>("lajevard");
	progressPanel = $state(true);
	/** Show the output of the user's Claude statusLine.command instead of the built-in line. */
	statuslineScript = $state(false);
	editor = $state<Editor>("default");

	constructor() {
		try {
			const saved = JSON.parse(localStorage.getItem(KEY) ?? "null") as Partial<Prefs> | null;
			if (saved?.palette) this.palette = saved.palette;
			if (typeof saved?.progressPanel === "boolean") this.progressPanel = saved.progressPanel;
			if (typeof saved?.statuslineScript === "boolean") this.statuslineScript = saved.statuslineScript;
			if (saved?.editor) this.editor = saved.editor;
		} catch {
			/* ignore corrupt prefs */
		}
	}

	save() {
		const data: Prefs = {
			palette: this.palette,
			progressPanel: this.progressPanel,
			statuslineScript: this.statuslineScript,
			editor: this.editor,
		};
		localStorage.setItem(KEY, JSON.stringify(data));
	}

	setPalette(p: Palette) {
		this.palette = p;
		this.save();
	}
}

export const prefs = new PrefStore();

export const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent);

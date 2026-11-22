// UI preferences kept in localStorage.

export type Palette = "lajevard" | "dood";

const KEY = "glaude.prefs.v1";

interface Prefs {
	palette: Palette;
	progressPanel: boolean;
	statuslineScript: boolean;
}

class PrefStore {
	palette = $state<Palette>("lajevard");
	progressPanel = $state(true);
	/** Show the output of the user's Claude statusLine.command instead of the built-in line. */
	statuslineScript = $state(false);

	constructor() {
		try {
			const saved = JSON.parse(localStorage.getItem(KEY) ?? "null") as Partial<Prefs> | null;
			if (saved?.palette) this.palette = saved.palette;
			if (typeof saved?.progressPanel === "boolean") this.progressPanel = saved.progressPanel;
			if (typeof saved?.statuslineScript === "boolean") this.statuslineScript = saved.statuslineScript;
		} catch {
			/* ignore corrupt prefs */
		}
	}

	save() {
		const data: Prefs = { palette: this.palette, progressPanel: this.progressPanel, statuslineScript: this.statuslineScript };
		localStorage.setItem(KEY, JSON.stringify(data));
	}

	setPalette(p: Palette) {
		this.palette = p;
		this.save();
	}
}

export const prefs = new PrefStore();

export const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent);

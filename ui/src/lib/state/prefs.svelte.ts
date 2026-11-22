// UI preferences kept in localStorage.

export type Palette = "lajevard" | "dood";

const KEY = "glaude.prefs.v1";

interface Prefs {
	palette: Palette;
	progressPanel: boolean;
}

class PrefStore {
	palette = $state<Palette>("lajevard");
	progressPanel = $state(true);

	constructor() {
		try {
			const saved = JSON.parse(localStorage.getItem(KEY) ?? "null") as Partial<Prefs> | null;
			if (saved?.palette) this.palette = saved.palette;
			if (typeof saved?.progressPanel === "boolean") this.progressPanel = saved.progressPanel;
		} catch {
			/* ignore corrupt prefs */
		}
	}

	save() {
		const data: Prefs = { palette: this.palette, progressPanel: this.progressPanel };
		localStorage.setItem(KEY, JSON.stringify(data));
	}

	setPalette(p: Palette) {
		this.palette = p;
		this.save();
	}
}

export const prefs = new PrefStore();

export const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent);

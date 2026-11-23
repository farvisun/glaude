// The user's themes and which theme each part of the window uses.

import { mode } from "mode-watcher";
import { prefs } from "$lib/state/prefs.svelte";
import { PRESETS } from "./presets";
import type { Kind, Theme } from "./types";
import { themeVars } from "./vars";

const KEY = "glaude.themes.v1";

/** "app" (same as the window), "app-light" / "app-dark" (the window's light
 *  or dark theme), or a theme id. Used for chat and terminals. */
export type ScopeSetting = string;

class ThemeStore {
	custom = $state<Theme[]>([]);

	constructor() {
		try {
			const saved = JSON.parse(localStorage.getItem(KEY) ?? "null") as Theme[] | null;
			if (Array.isArray(saved)) this.custom = saved.filter((t) => t && t.id && t.bg && t.fg);
		} catch {
			/* ignore */
		}
	}

	private save() {
		localStorage.setItem(KEY, JSON.stringify($state.snapshot(this.custom)));
	}

	get all(): Theme[] {
		return [...PRESETS, ...this.custom];
	}

	get(id: string | undefined | null): Theme | undefined {
		if (!id) return undefined;
		return PRESETS.find((t) => t.id === id) ?? this.custom.find((t) => t.id === id);
	}

	/** The window's theme for a mode (Sade when the saved one is gone). */
	forKind(kind: Kind): Theme {
		const t = this.get(kind === "dark" ? prefs.darkTheme : prefs.lightTheme);
		return t ?? this.get(`sade-${kind}`)!;
	}

	/** The window's current theme. */
	get app(): Theme {
		return this.forKind(mode.current === "dark" ? "dark" : "light");
	}

	resolve(setting: ScopeSetting): Theme {
		if (setting === "app-light") return this.forKind("light");
		if (setting === "app-dark") return this.forKind("dark");
		return (setting !== "app" && this.get(setting)) || this.app;
	}

	/** Chat or terminal theme when it differs from the window's, else null. */
	island(setting: ScopeSetting): { theme: Theme; style: string; kind: Kind } | null {
		if (setting === "app") return null;
		const t = this.resolve(setting);
		if (t.id === this.app.id) return null;
		const style = Object.entries(themeVars(t, prefs.accent))
			.map(([k, v]) => `${k}: ${v}`)
			.join("; ");
		return { theme: t, style, kind: t.kind };
	}

	get chat() {
		return this.island(prefs.chatTheme);
	}
	get terminal(): Theme {
		return this.resolve(prefs.termTheme);
	}
	get terminalIsland() {
		return this.island(prefs.termTheme);
	}

	/** Copy of a theme the user can change (exact palette values dropped so
	 *  every colour in the editor takes effect). */
	draft(from: Theme, name?: string): Theme {
		const copy = JSON.parse(JSON.stringify(from)) as Theme;
		return { ...copy, id: "", name: name ?? `${from.name} (yours)`, family: "Yours", custom: true, vars: undefined };
	}

	/** Add or replace; returns the saved theme. */
	put(t: Theme): Theme {
		const theme = { ...t, family: "Yours", custom: true, id: t.id || `custom-${Date.now().toString(36)}` };
		const i = this.custom.findIndex((x) => x.id === theme.id);
		if (i >= 0) this.custom[i] = theme;
		else this.custom.push(theme);
		this.save();
		return theme;
	}

	remove(id: string) {
		const t = this.get(id);
		this.custom = this.custom.filter((x) => x.id !== id);
		this.save();
		// Anything that used it falls back to Sade or to the window's theme.
		if (prefs.lightTheme === id) prefs.set("lightTheme", "sade-light");
		if (prefs.darkTheme === id) prefs.set("darkTheme", "sade-dark");
		if (prefs.chatTheme === id) prefs.set("chatTheme", "app");
		if (prefs.termTheme === id) prefs.set("termTheme", "app");
		return t;
	}

	/** Put a removed theme back (Undo). */
	restore(t: Theme) {
		this.put(t);
	}
}

export const themes = new ThemeStore();

/** Settings value → label, for menus. */
export function scopeLabel(setting: ScopeSetting): string {
	if (setting === "app") return "Same as the app";
	if (setting === "app-light") return `App's light theme (${themes.forKind("light").name})`;
	if (setting === "app-dark") return `App's dark theme (${themes.forKind("dark").name})`;
	return themes.get(setting)?.name ?? "Same as the app";
}

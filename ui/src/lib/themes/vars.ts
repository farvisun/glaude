// Theme → CSS variables (the app's tokens) and → xterm colours.

import type { ITheme } from "@xterm/xterm";
import { alpha, legible, mix, norm, textOn } from "./color";
import { ACCENTS } from "./presets";
import { ANSI_NAMES, SYNTAX_KEYS, type Theme } from "./types";

/** "theme" (the theme's own), an accent id from ACCENTS, or "#rrggbb". */
export function accentOf(t: Theme, setting = "theme"): string {
	if (setting.startsWith("#")) return norm(setting, t.accent);
	const a = ACCENTS.find((x) => x.id === setting);
	return norm(a ? a[t.kind] : t.accent);
}

/** Every token the app reads, with "--" names. Missing pieces are derived
 *  from the theme's ground and text, and status colours are nudged until they
 *  read on the ground (imported themes vary a lot). */
export function themeVars(t: Theme, accentSetting = "theme"): Record<string, string> {
	const dark = t.kind === "dark";
	const bg = norm(t.bg);
	const fg = norm(t.fg, dark ? "#ececf1" : "#0d0f14");
	const tint = (a: number) => alpha(fg, a);
	const card = norm(t.card, dark ? mix(bg, fg, 0.035) : mix(bg, "#ffffff", 0.7));
	const popover = norm(t.popover, dark ? mix(bg, fg, 0.06) : card);
	const sidebar = norm(t.sidebar, dark ? mix(bg, "#000000", 0.15) : mix(bg, fg, 0.025));
	const muted = legible(t.muted ?? mix(fg, bg, 0.38), bg, 4, fg);
	const codeBg = norm(t.codeBg, dark ? mix(bg, "#000000", 0.18) : mix(bg, fg, 0.03));
	const accent = accentOf(t, accentSetting);
	const onAccent = textOn(accent, "#ffffff", dark ? bg : fg);
	const red = legible(t.red, bg, 3.5, fg);
	const ok = legible(t.green, bg, 3, fg);
	const attention = legible(t.yellow, bg, 3, fg);
	const link = legible(t.blue, bg, 4.5, fg);
	const ring = mix(fg, bg, 0.45);

	const v: Record<string, string> = {
		background: bg,
		foreground: fg,
		card,
		"card-foreground": fg,
		popover,
		"popover-foreground": fg,
		primary: accent,
		"primary-foreground": onAccent,
		secondary: tint(dark ? 0.06 : 0.05),
		"secondary-foreground": fg,
		muted: tint(dark ? 0.05 : 0.045),
		"muted-foreground": muted,
		accent: tint(dark ? 0.075 : 0.06),
		"accent-foreground": fg,
		destructive: red,
		border: tint(0.1),
		input: tint(dark ? 0.15 : 0.14),
		ring,
		sidebar,
		"sidebar-foreground": fg,
		"sidebar-primary": accent,
		"sidebar-primary-foreground": onAccent,
		"sidebar-accent": tint(dark ? 0.07 : 0.06),
		"sidebar-accent-foreground": fg,
		"sidebar-border": tint(0.08),
		"sidebar-ring": ring,
		attention,
		"attention-foreground": textOn(attention, "#ffffff", "#1f1605"),
		ok,
		link,
		"code-bg": codeBg,
		"term-bg": norm(t.termBg, codeBg),
		"user-bubble": tint(dark ? 0.07 : 0.055),
		claude: dark ? "#e58a5f" : "#c8663d",
		codex: dark ? "#7aa2ff" : "#3d6fd8",
		"float-shadow": dark
			? "0 1px 2px rgb(0 0 0 / 0.3), 0 10px 28px -6px rgb(0 0 0 / 0.55)"
			: "0 1px 2px rgb(0 0 0 / 0.04), 0 8px 24px -6px rgb(0 0 0 / 0.14)",
		"dialog-shadow": dark
			? "0 2px 8px rgb(0 0 0 / 0.35), 0 28px 56px -12px rgb(0 0 0 / 0.7)"
			: "0 2px 6px rgb(0 0 0 / 0.05), 0 24px 48px -12px rgb(0 0 0 / 0.22)",
		"skeleton-hi": dark ? "9%" : "55%",
		"shimmer-lift": dark ? "55%" : "30%",
	};
	for (const k of SYNTAX_KEYS) v[`syn-${k}`] = legible(t.syntax?.[k] ?? fg, codeBg, k === "comment" ? 2.3 : 2.8, fg);
	ANSI_NAMES.forEach((_, i) => (v[`ansi-${i}`] = norm(t.ansi?.[i], i === 0 ? "#000000" : fg)));

	// Exact values of the ported palettes win; a chosen accent wins over them.
	Object.assign(v, t.vars ?? {});
	if (accentSetting !== "theme") {
		v.primary = v["sidebar-primary"] = accent;
		v["primary-foreground"] = v["sidebar-primary-foreground"] = onAccent;
	}
	return Object.fromEntries(Object.entries(v).map(([k, x]) => [`--${k}`, x]));
}

export function xtermTheme(t: Theme, accentSetting = "theme"): ITheme {
	const bg = themeVars(t, accentSetting)["--term-bg"];
	const accent = accentOf(t, accentSetting);
	const out: ITheme = {
		background: bg,
		foreground: norm(t.fg),
		cursor: accent,
		cursorAccent: bg,
		selectionBackground: accent + "55",
	};
	const o = out as Record<string, string>;
	ANSI_NAMES.forEach((name, i) => (o[name] = norm(t.ansi?.[i], i === 0 ? "#000000" : norm(t.fg))));
	return out;
}

// Applying ---------------------------------------------------------------------

const applied = new WeakMap<HTMLElement, string[]>();

/** Set (or clear, with null) a theme's variables on an element. Islands
 *  (chat, terminal) also get `dark`/`light` so `dark:` utilities follow them. */
export function applyVars(el: HTMLElement, vars: Record<string, string> | null, island?: "dark" | "light" | null) {
	for (const k of applied.get(el) ?? []) if (!vars || !(k in vars)) el.style.removeProperty(k);
	if (vars) for (const [k, x] of Object.entries(vars)) el.style.setProperty(k, x);
	applied.set(el, vars ? Object.keys(vars) : []);
	if (island !== undefined) {
		el.classList.toggle("dark", island === "dark");
		el.classList.toggle("light", island === "light");
		el.classList.toggle("theme-island", !!island);
	}
}

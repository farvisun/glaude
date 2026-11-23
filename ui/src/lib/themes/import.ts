// Read theme files from other apps into a Glaude theme:
// VS Code (.json, comments allowed), iTerm2 (.itermcolors), Ghostty, kitty,
// Windows Terminal (a scheme or settings.json) and Glaude's own JSON.

import { isColor, isDark, mix, norm } from "./color";
import { ANSI_NAMES, SYNTAX_KEYS, type Kind, type Syntax, type Theme } from "./types";

export class ThemeImportError extends Error {}

const baseName = (file: string) =>
	file
		.replace(/^.*[\\/]/, "")
		.replace(/\.(json|jsonc|itermcolors|conf|toml|txt|theme)$/i, "")
		.replace(/[-_]+/g, " ")
		.trim() || "Imported theme";

export function parseThemeFile(fileName: string, text: string): Theme {
	const t = text.trim();
	if (!t) throw new ThemeImportError("The file is empty.");
	if (t.includes("<plist") || t.includes("Ansi 0 Color")) return fromIterm(baseName(fileName), t);
	if (t.startsWith("{") || t.startsWith("[")) {
		let json: unknown;
		try {
			json = JSON.parse(stripJsonc(t));
		} catch {
			throw new ThemeImportError("The file isn't valid JSON.");
		}
		return fromJson(baseName(fileName), json);
	}
	if (/^\s*palette\s*=\s*\d+\s*=/m.test(t)) return fromGhostty(baseName(fileName), t);
	if (/^\s*color\d+\s+#?[0-9a-f]{6}/im.test(t)) return fromKitty(baseName(fileName), t);
	throw new ThemeImportError(
		"Glaude can't read this file. Use a VS Code theme (.json), an iTerm2 scheme (.itermcolors), or a Ghostty, kitty or Windows Terminal scheme.",
	);
}

/** JSON with comments and trailing commas (VS Code's format) → JSON. */
export function stripJsonc(s: string): string {
	let out = "";
	let inStr = false;
	for (let i = 0; i < s.length; i++) {
		const c = s[i];
		if (inStr) {
			out += c;
			if (c === "\\") out += s[++i] ?? "";
			else if (c === '"') inStr = false;
		} else if (c === '"') {
			inStr = true;
			out += c;
		} else if (c === "/" && s[i + 1] === "/") {
			while (i < s.length && s[i] !== "\n") i++;
			out += "\n";
		} else if (c === "/" && s[i + 1] === "*") {
			i += 2;
			while (i < s.length && !(s[i] === "*" && s[i + 1] === "/")) i++;
			i++;
		} else out += c;
	}
	return out.replace(/,(\s*[}\]])/g, "$1");
}

function fromJson(name: string, json: unknown): Theme {
	if (Array.isArray(json)) json = json[0];
	if (!json || typeof json !== "object") throw new ThemeImportError("The file has no theme in it.");
	const o = json as Record<string, unknown>;
	// Glaude's own export.
	if (Array.isArray(o.ansi) && o.syntax && typeof o.bg === "string") return glaudeTheme(o, name);
	// VS Code theme.
	if (o.colors || o.tokenColors) return fromVscode(name, o);
	// Windows Terminal settings.json (first scheme) or a single scheme.
	if (Array.isArray(o.schemes) && o.schemes.length) return fromWindowsTerminal(name, o.schemes[0] as Record<string, string>);
	if (typeof o.background === "string" && (typeof o.brightBlack === "string" || typeof o.purple === "string"))
		return fromWindowsTerminal(name, o as Record<string, string>);
	throw new ThemeImportError("This JSON isn't a theme Glaude knows (VS Code, Windows Terminal or Glaude).");
}

function glaudeTheme(o: Record<string, unknown>, name: string): Theme {
	const t = o as unknown as Theme;
	if (!isColor(t.bg) || !isColor(t.fg)) throw new ThemeImportError("The theme has no background or text colour.");
	const kind: Kind = t.kind === "light" || t.kind === "dark" ? t.kind : isDark(t.bg) ? "dark" : "light";
	const ansi = ANSI_NAMES.map((_, i) => norm(t.ansi[i], i ? t.fg : "#000000"));
	const syntax = Object.fromEntries(SYNTAX_KEYS.map((k) => [k, norm(t.syntax?.[k], t.fg)])) as Syntax;
	return {
		...fromTerminal(typeof t.name === "string" ? t.name : name, t.bg, t.fg, ansi),
		...pickColors(t),
		kind,
		ansi,
		syntax,
	};
}

/** Optional surface and status colours that are valid. */
function pickColors(t: Partial<Theme>): Partial<Theme> {
	const out: Partial<Theme> = {};
	for (const k of ["card", "popover", "sidebar", "muted", "accent", "red", "green", "yellow", "blue", "codeBg", "termBg"] as const)
		if (isColor(t[k])) out[k] = norm(t[k]);
	return out;
}

/** A theme from terminal colours alone: the rest comes from the 16 colours. */
export function fromTerminal(name: string, bg: string, fg: string, ansi: string[], cursor?: string): Theme {
	const kind: Kind = isDark(bg) ? "dark" : "light";
	const a = ANSI_NAMES.map((_, i) => norm(ansi[i], i ? fg : "#000000"));
	const comment = a[8] && a[8] !== norm(bg) ? a[8] : mix(fg, bg, 0.45);
	return {
		id: "",
		name,
		family: "Yours",
		kind,
		bg: norm(bg),
		fg: norm(fg),
		accent: cursor && isColor(cursor) && cursor.toLowerCase() !== fg.toLowerCase() ? norm(cursor) : a[4],
		red: a[1],
		green: a[2],
		yellow: a[3],
		blue: a[4],
		ansi: a,
		syntax: {
			comment,
			keyword: a[5],
			string: a[2],
			number: a[3],
			function: a[4],
			type: a[6],
			variable: a[1],
			tag: a[1],
			attribute: a[3],
			operator: a[6],
		},
		custom: true,
	};
}

function fromIterm(name: string, xml: string): Theme {
	// Top-level <key>Name</key><dict>…components…</dict> pairs; no DOM needed.
	const colors: Record<string, string> = {};
	for (const m of xml.matchAll(/<key>([^<]+)<\/key>\s*<dict>([\s\S]*?)<\/dict>/g)) {
		const part = (c: string) => {
			const v = new RegExp(`<key>${c} Component</key>\\s*<(?:real|integer)>([^<]+)</`).exec(m[2]);
			return Math.round(Math.max(0, Math.min(1, Number(v?.[1] ?? 0))) * 255);
		};
		colors[m[1].trim()] = norm(`#${[part("Red"), part("Green"), part("Blue")].map((n) => n.toString(16).padStart(2, "0")).join("")}`);
	}
	const bg = colors["Background Color"];
	const fg = colors["Foreground Color"];
	if (!bg || !fg) throw new ThemeImportError("The iTerm2 scheme has no background or text colour.");
	const ansi = ANSI_NAMES.map((_, i) => colors[`Ansi ${i} Color`]);
	return fromTerminal(name, bg, fg, ansi, colors["Cursor Color"]);
}

function keyValues(text: string, sep: RegExp): Record<string, string> {
	const out: Record<string, string> = {};
	for (const line of text.split("\n")) {
		const l = line.replace(/#(?![0-9a-f]{3,8}\b).*$/i, "").trim();
		const m = sep.exec(l);
		if (m) out[m[1].trim().toLowerCase()] = m[2].trim();
	}
	return out;
}

function fromGhostty(name: string, text: string): Theme {
	const kv = keyValues(text, /^([\w-]+)\s*=\s*(.+)$/);
	const ansi: string[] = [];
	for (const line of text.split("\n")) {
		const m = /^\s*palette\s*=\s*(\d+)\s*=\s*(#?[0-9a-f]{6})/i.exec(line);
		if (m && Number(m[1]) < 16) ansi[Number(m[1])] = norm(m[2]);
	}
	if (!kv.background || !kv.foreground) throw new ThemeImportError("The Ghostty theme has no background or foreground.");
	return fromTerminal(name, norm(kv.background), norm(kv.foreground), ansi, kv["cursor-color"]);
}

function fromKitty(name: string, text: string): Theme {
	const kv = keyValues(text, /^([\w-]+)\s+(#?[0-9a-f]{3,8})\b/i);
	const ansi = ANSI_NAMES.map((_, i) => (kv[`color${i}`] ? norm(kv[`color${i}`]) : ""));
	if (!kv.background || !kv.foreground) throw new ThemeImportError("The kitty theme has no background or foreground.");
	return fromTerminal(name, norm(kv.background), norm(kv.foreground), ansi, kv.cursor);
}

function fromWindowsTerminal(name: string, s: Record<string, string>): Theme {
	const order = ["black", "red", "green", "yellow", "blue", "purple", "cyan", "white"];
	const ansi = [...order, ...order.map((k) => "bright" + k[0].toUpperCase() + k.slice(1))].map((k) => s[k] ?? "");
	if (!isColor(s.background) || !isColor(s.foreground)) throw new ThemeImportError("The scheme has no background or foreground.");
	return fromTerminal(s.name || name, s.background, s.foreground, ansi, s.cursorColor);
}

// VS Code ---------------------------------------------------------------------

type TokenRule = { scope?: string | string[]; settings?: { foreground?: string } };

const SCOPES: Record<keyof Syntax, string[]> = {
	comment: ["comment"],
	keyword: ["keyword.control", "keyword", "storage.type", "storage"],
	string: ["string.quoted", "string"],
	number: ["constant.numeric", "constant.language", "constant"],
	function: ["entity.name.function", "support.function", "meta.function-call"],
	type: ["entity.name.type", "support.type", "entity.name.class", "support.class"],
	variable: ["variable.parameter", "variable.other", "variable"],
	tag: ["entity.name.tag"],
	attribute: ["entity.other.attribute-name"],
	operator: ["keyword.operator"],
};

/** The colour TextMate would use for a scope: an exact rule, then the closest
 *  more general one (`keyword` for `keyword.control`). */
export function scopeColor(rules: TokenRule[], wanted: string[]): string | undefined {
	for (const want of wanted) {
		let best: { len: number; color: string } | null = null;
		for (const r of rules) {
			const color = r.settings?.foreground;
			if (!color || !isColor(color)) continue;
			const scopes = (Array.isArray(r.scope) ? r.scope : (r.scope ?? "").split(",")).map((x) => x.trim());
			for (const sc of scopes) {
				if (!sc || sc.includes(" ")) continue;
				if (want === sc || want.startsWith(sc + ".")) {
					if (!best || sc.length > best.len) best = { len: sc.length, color };
				}
			}
		}
		if (best) return norm(best.color);
	}
	return undefined;
}

function fromVscode(fileName: string, o: Record<string, unknown>): Theme {
	const c = (o.colors ?? {}) as Record<string, string>;
	const rules = (Array.isArray(o.tokenColors) ? o.tokenColors : []) as TokenRule[];
	const get = (...keys: string[]) => {
		for (const k of keys) if (isColor(c[k])) return norm(c[k]);
		return undefined;
	};
	const kindHint = typeof o.type === "string" ? o.type : "";
	const bg = get("editor.background") ?? (kindHint.includes("light") ? "#ffffff" : "#1e1e1e");
	const fg = get("editor.foreground", "foreground") ?? (isDark(bg) ? "#d4d4d4" : "#1f1f1f");
	const kind: Kind = kindHint.includes("light") ? "light" : kindHint === "dark" || kindHint.startsWith("hc") ? "dark" : isDark(bg) ? "dark" : "light";
	const ansi = ANSI_NAMES.map((n) => get(`terminal.ansi${n[0].toUpperCase()}${n.slice(1)}`) ?? "");
	const syntax = Object.fromEntries(SYNTAX_KEYS.map((k) => [k, scopeColor(rules, SCOPES[k]) ?? ""])) as Syntax;
	// Fill gaps from each other: terminal colours from code colours and back.
	const base = fromTerminal(
		typeof o.name === "string" && o.name.trim() ? o.name.trim() : fileName,
		bg,
		fg,
		ansi.map((x, i) =>
			x || [mix(bg, fg, 0.15), syntax.tag, syntax.string, syntax.number, syntax.function, syntax.keyword, syntax.type, fg][i % 8] || "",
		),
	);
	for (const k of SYNTAX_KEYS) if (syntax[k]) base.syntax[k] = syntax[k];
	return {
		...base,
		kind,
		card: get("editorWidget.background", "sideBar.background"),
		popover: get("menu.background", "dropdown.background", "editorWidget.background"),
		sidebar: get("sideBar.background", "activityBar.background"),
		muted: get("descriptionForeground", "editorLineNumber.activeForeground"),
		accent: get("button.background", "focusBorder", "textLink.foreground") ?? base.accent,
		red: get("errorForeground", "editorError.foreground") ?? base.red,
		green: get("gitDecoration.addedResourceForeground", "terminal.ansiGreen") ?? base.green,
		yellow: get("editorWarning.foreground", "terminal.ansiYellow") ?? base.yellow,
		blue: get("textLink.foreground", "terminal.ansiBlue") ?? base.blue,
		codeBg: get("textCodeBlock.background"),
		termBg: get("terminal.background"),
	};
}

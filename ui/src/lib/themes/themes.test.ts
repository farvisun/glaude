import { describe, expect, it } from "vitest";
import { contrast, legible, mix, norm, parse, textOn } from "./color";
import { PRESETS, ACCENTS, FAMILIES } from "./presets";
import { themeVars, xtermTheme } from "./vars";
import { parseThemeFile, scopeColor, stripJsonc, ThemeImportError } from "./import";
import { SYNTAX_KEYS } from "./types";

describe("colour helpers", () => {
	it("parses and mixes", () => {
		expect(parse("#abc")).toEqual([170, 187, 204]);
		expect(parse("282c34")).toEqual([40, 44, 52]);
		expect(parse("#282c34cc")).toEqual([40, 44, 52]);
		expect(norm("rgb(1 2 3)")).toBe("#010203");
		expect(norm("nope", "#ffffff")).toBe("#ffffff");
		expect(mix("#000000", "#ffffff", 0.5)).toBe("#808080");
	});
	it("keeps text readable", () => {
		expect(contrast("#000000", "#ffffff")).toBeCloseTo(21, 0);
		expect(textOn("#ffd700")).toBe("#0b0c0f");
		expect(textOn("#1e40af")).toBe("#ffffff");
		const nudged = legible("#f1fa8c", "#ffffff", 3, "#000000");
		expect(contrast(nudged, "#ffffff")).toBeGreaterThanOrEqual(3);
	});
});

describe("presets", () => {
	it("has unique ids, known families and full colour sets", () => {
		expect(new Set(PRESETS.map((t) => t.id)).size).toBe(PRESETS.length);
		for (const t of PRESETS) {
			expect(FAMILIES).toContain(t.family);
			expect(t.ansi, t.id).toHaveLength(16);
			for (const c of t.ansi) expect(parse(c), `${t.id} ${c}`).not.toBeNull();
			for (const k of SYNTAX_KEYS) expect(parse(t.syntax[k]), `${t.id} ${k}`).not.toBeNull();
		}
		expect(PRESETS.filter((t) => t.kind === "light").length).toBeGreaterThan(8);
		expect(PRESETS.filter((t) => t.kind === "dark").length).toBeGreaterThan(10);
	});

	it("gives every theme readable text", () => {
		for (const t of PRESETS) {
			const v = themeVars(t);
			expect(contrast(v["--foreground"], v["--background"]), t.id).toBeGreaterThanOrEqual(4.5);
			expect(contrast(v["--muted-foreground"], v["--background"]), t.id).toBeGreaterThanOrEqual(3.9);
			expect(contrast(v["--link"], v["--background"]), t.id).toBeGreaterThanOrEqual(4.4);
			expect(contrast(v["--primary-foreground"], v["--primary"]), t.id).toBeGreaterThanOrEqual(2.5);
		}
	});

	it("keeps Sade's exact values and applies a chosen accent", () => {
		const sade = PRESETS.find((t) => t.id === "sade-light")!;
		expect(themeVars(sade)["--border"]).toBe("rgb(13 15 20 / 0.1)");
		expect(themeVars(sade)["--primary"]).toBe("#111318");
		const blue = ACCENTS.find((a) => a.id === "blue")!;
		expect(themeVars(sade, "blue")["--primary"]).toBe(blue.light);
		expect(themeVars(sade, "#ff00aa")["--primary"]).toBe("#ff00aa");
		const dracula = PRESETS.find((t) => t.id === "dracula")!;
		expect(themeVars(dracula, "blue")["--primary"]).toBe(blue.dark);
	});

	it("builds xterm colours", () => {
		const x = xtermTheme(PRESETS.find((t) => t.id === "nord")!);
		expect(x.red).toBe("#bf616a");
		expect(x.brightWhite).toBe("#eceff4");
		expect(x.background).toBe("#292e39");
	});
});

describe("import", () => {
	it("reads a VS Code theme with comments and trailing commas", () => {
		const text = `{
			// My theme
			"name": "Midnight",
			"type": "dark",
			"colors": {
				"editor.background": "#101010", /* ground */
				"editor.foreground": "#e0e0e0",
				"button.background": "#ff8800",
				"terminal.ansiRed": "#ff0000",
				"terminal.ansiBrightWhite": "#ffffff",
			},
			"tokenColors": [
				{ "scope": "comment", "settings": { "foreground": "#777777" } },
				{ "scope": ["keyword", "storage"], "settings": { "foreground": "#c792ea" } },
				{ "scope": "keyword.operator", "settings": { "foreground": "#89ddff" } },
				{ "scope": "string, string.quoted", "settings": { "foreground": "#c3e88d" } },
				{ "scope": "entity.name.function", "settings": { "foreground": "#82aaff" } },
			],
		}`;
		const t = parseThemeFile("midnight-color-theme.json", text);
		expect(t.name).toBe("Midnight");
		expect(t.kind).toBe("dark");
		expect(t.bg).toBe("#101010");
		expect(t.accent).toBe("#ff8800");
		expect(t.ansi[1]).toBe("#ff0000");
		expect(t.ansi[15]).toBe("#ffffff");
		expect(t.syntax.comment).toBe("#777777");
		expect(t.syntax.keyword).toBe("#c792ea");
		expect(t.syntax.operator).toBe("#89ddff");
		expect(t.syntax.string).toBe("#c3e88d");
		expect(t.syntax.function).toBe("#82aaff");
	});

	it("matches the closest general scope", () => {
		const rules = [
			{ scope: "keyword", settings: { foreground: "#111111" } },
			{ scope: "keyword.control.flow", settings: { foreground: "#222222" } },
		];
		expect(scopeColor(rules, ["keyword.control"])).toBe("#111111");
		expect(stripJsonc(`{"a": "http://x", // c\n "b": [1,],}`)).toBe(`{"a": "http://x", \n "b": [1]}`);
	});

	it("reads iTerm2, Ghostty, kitty and Windows Terminal schemes", () => {
		const comp = (r: number, g: number, b: number) =>
			`<dict><key>Blue Component</key><real>${b}</real><key>Green Component</key><real>${g}</real><key>Red Component</key><real>${r}</real></dict>`;
		const plist = `<?xml version="1.0"?><plist version="1.0"><dict>
			<key>Ansi 1 Color</key>${comp(1, 0, 0)}
			<key>Background Color</key>${comp(0, 0, 0)}
			<key>Foreground Color</key>${comp(1, 1, 1)}
		</dict></plist>`;
		const it2 = parseThemeFile("Snazzy.itermcolors", plist);
		expect(it2.name).toBe("Snazzy");
		expect(it2.bg).toBe("#000000");
		expect(it2.ansi[1]).toBe("#ff0000");
		expect(it2.kind).toBe("dark");

		const gh = parseThemeFile("Builtin Light", "background = fafafa\nforeground = #383a42\npalette = 1=#e45649\npalette = 4=#4078f2\n# comment\n");
		expect(gh.kind).toBe("light");
		expect(gh.ansi[4]).toBe("#4078f2");
		expect(gh.accent).toBe("#4078f2");

		const kitty = parseThemeFile("gruvbox.conf", "background #282828\nforeground #ebdbb2\ncolor1 #cc241d\ncursor #fe8019\n");
		expect(kitty.ansi[1]).toBe("#cc241d");
		expect(kitty.accent).toBe("#fe8019");

		const wt = parseThemeFile("x.json", JSON.stringify({ name: "Campbell", background: "#0C0C0C", foreground: "#CCCCCC", red: "#C50F1F", purple: "#881798", brightBlack: "#767676" }));
		expect(wt.name).toBe("Campbell");
		expect(wt.ansi[5]).toBe("#881798");
		expect(wt.ansi[8]).toBe("#767676");
	});

	it("round-trips Glaude's own JSON and rejects other files", () => {
		const { id: _i, vars: _v, ...dracula } = PRESETS.find((t) => t.id === "dracula")!;
		const back = parseThemeFile("dracula.json", JSON.stringify(dracula));
		expect(back.name).toBe("Dracula");
		expect(back.ansi).toEqual(dracula.ansi);
		expect(back.syntax).toEqual(dracula.syntax);
		expect(back.card).toBe(dracula.card);
		expect(() => parseThemeFile("a.txt", "hello world")).toThrow(ThemeImportError);
		expect(() => parseThemeFile("a.json", "{ nope")).toThrow(ThemeImportError);
	});
});

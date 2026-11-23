// Ready-made themes. Sade, Lajevard and Dood are Glaude's own (exact CSS
// values in `vars`); the rest follow the published palettes of each theme.

import type { Syntax, Theme } from "./types";

const syn = (
	comment: string,
	keyword: string,
	string: string,
	number: string,
	fn: string,
	type: string,
	variable: string,
	tag: string,
	attribute: string,
	operator: string,
): Syntax => ({ comment, keyword, string, number, function: fn, type, variable, tag, attribute, operator });

const ansi = (s: string) => s.trim().split(/\s+/);

// Glaude's palettes (moved from app.css) ------------------------------------

const ANSI_GLAUDE_LIGHT = ansi(
	"#1c2333 #c4372f #2c7a3a #946200 #2459b8 #8a3fb8 #0f7f76 #c9cfda #5d6880 #d9534c #3a9a4b #b07a00 #3b74d6 #a259d1 #159a90 #ffffff",
);
const ANSI_GLAUDE_DARK = ansi(
	"#1b2233 #f0716b #7fd18b #f2c66d #6fa8f5 #c79cf2 #5cc8bd #d5dbe6 #5d6880 #ff8f88 #9ee3a8 #ffd98a #92bfff #dcb8ff #84e0d6 #f4f7fb",
);

function glaude(
	id: string,
	name: string,
	kind: "light" | "dark",
	v: Record<string, string>,
	syntax: Syntax,
	link: string,
): Theme {
	return {
		id,
		name,
		family: name.split(" ")[0],
		kind,
		bg: v["background"],
		fg: v["foreground"],
		card: v["card"],
		popover: v["popover"],
		sidebar: v["sidebar"],
		muted: v["muted-foreground"],
		accent: v["primary"],
		red: v["destructive"],
		green: v["ok"],
		yellow: v["attention"],
		blue: link,
		codeBg: v["code-bg"],
		termBg: v["code-bg"],
		ansi: kind === "dark" ? ANSI_GLAUDE_DARK : ANSI_GLAUDE_LIGHT,
		syntax,
		vars: v,
	};
}

/** Lines of `name: value;` → record. */
const css = (s: string) =>
	Object.fromEntries(
		s
			.split(";")
			.map((l) => l.trim())
			.filter(Boolean)
			.map((l) => {
				const i = l.indexOf(":");
				return [l.slice(0, i).trim(), l.slice(i + 1).trim()];
			}),
	);

const SADE_LIGHT = css(`
	background: #fcfcfd; foreground: #0d0f14; card: #ffffff; card-foreground: #0d0f14; popover: #ffffff; popover-foreground: #0d0f14;
	primary: #111318; primary-foreground: #ffffff; secondary: rgb(13 15 20 / 0.05); secondary-foreground: #0d0f14;
	muted: rgb(13 15 20 / 0.045); muted-foreground: #5c6270; accent: rgb(13 15 20 / 0.06); accent-foreground: #0d0f14;
	destructive: #d92d20; border: rgb(13 15 20 / 0.1); input: rgb(13 15 20 / 0.14); ring: #8a8f9c;
	sidebar: #f7f7f9; sidebar-foreground: #0d0f14; sidebar-primary: #111318; sidebar-primary-foreground: #ffffff;
	sidebar-accent: rgb(13 15 20 / 0.06); sidebar-accent-foreground: #0d0f14; sidebar-border: rgb(13 15 20 / 0.08); sidebar-ring: #8a8f9c;
	attention: #b45309; attention-foreground: #fffaf0; ok: #15803d; code-bg: #f5f6f8; user-bubble: #f0f1f4; claude: #c8663d; codex: #3d6fd8`);
const SADE_DARK = css(`
	background: #0b0c0f; foreground: #ececf1; card: #111216; card-foreground: #ececf1; popover: #15161a; popover-foreground: #ececf1;
	primary: #f1f2f5; primary-foreground: #0b0c0f; secondary: rgb(236 236 241 / 0.06); secondary-foreground: #ececf1;
	muted: rgb(236 236 241 / 0.05); muted-foreground: #9a9ca8; accent: rgb(236 236 241 / 0.075); accent-foreground: #ececf1;
	destructive: #f97066; border: rgb(236 236 241 / 0.1); input: rgb(236 236 241 / 0.15); ring: #6f7380;
	sidebar: #0e0f12; sidebar-foreground: #dcdde3; sidebar-primary: #f1f2f5; sidebar-primary-foreground: #0b0c0f;
	sidebar-accent: rgb(236 236 241 / 0.07); sidebar-accent-foreground: #ececf1; sidebar-border: rgb(236 236 241 / 0.08); sidebar-ring: #6f7380;
	attention: #f5b041; attention-foreground: #1f1605; ok: #4ade80; code-bg: #111216; user-bubble: rgb(236 236 241 / 0.07); claude: #e58a5f; codex: #7aa2ff`);
const LAJEVARD_LIGHT = css(`
	background: #f4f6fa; foreground: #15203a; card: #ffffff; card-foreground: #15203a; popover: #ffffff; popover-foreground: #15203a;
	primary: #0f7f76; primary-foreground: #f4fbfa; secondary: #e6ebf3; secondary-foreground: #15203a; muted: #e9edf4; muted-foreground: #5b6b86;
	accent: #e1e8f2; accent-foreground: #15203a; destructive: #c2413b; border: #d7deea; input: #cfd7e5; ring: #0f7f76;
	sidebar: #eaeef5; sidebar-foreground: #15203a; sidebar-primary: #0f7f76; sidebar-primary-foreground: #f4fbfa; sidebar-accent: #dbe3ef;
	sidebar-accent-foreground: #15203a; sidebar-border: #d3dbe8; sidebar-ring: #0f7f76;
	attention: #b7791f; attention-foreground: #fff8ea; ok: #2f855a; code-bg: #eef2f8; user-bubble: #e3ecf6; claude: #c8663d; codex: #3d6fd8`);
const LAJEVARD_DARK = css(`
	background: #101a2e; foreground: #e4eaf4; card: #16233b; card-foreground: #e4eaf4; popover: #1a2943; popover-foreground: #e4eaf4;
	primary: #5cc8bd; primary-foreground: #0b1a1f; secondary: #1e2e4a; secondary-foreground: #e4eaf4; muted: #1a2944; muted-foreground: #8d9eba;
	accent: #22355a; accent-foreground: #e4eaf4; destructive: #ef6b6b; border: #26385a; input: #2c4066; ring: #5cc8bd;
	sidebar: #0c1526; sidebar-foreground: #d5ddeb; sidebar-primary: #5cc8bd; sidebar-primary-foreground: #0b1a1f; sidebar-accent: #172540;
	sidebar-accent-foreground: #e4eaf4; sidebar-border: #1d2c48; sidebar-ring: #5cc8bd;
	attention: #f2b544; attention-foreground: #1f1605; ok: #63c98b; code-bg: #0d1628; user-bubble: #1b2b48; claude: #e58a5f; codex: #7aa2ff`);
const DOOD_LIGHT = css(`
	background: #f6f6f7; foreground: #1c1d21; card: #ffffff; card-foreground: #1c1d21; popover: #ffffff; popover-foreground: #1c1d21;
	primary: #4f5bd5; primary-foreground: #f7f7ff; secondary: #ececef; secondary-foreground: #1c1d21; muted: #efeff2; muted-foreground: #6a6c75;
	accent: #e8e8ec; accent-foreground: #1c1d21; destructive: #c4372f; border: #e0e0e5; input: #d6d6dd; ring: #4f5bd5;
	sidebar: #efeff1; sidebar-foreground: #1c1d21; sidebar-primary: #4f5bd5; sidebar-primary-foreground: #f7f7ff; sidebar-accent: #e4e4e9;
	sidebar-accent-foreground: #1c1d21; sidebar-border: #e0e0e5; sidebar-ring: #4f5bd5;
	attention: #b45309; attention-foreground: #fff7ed; ok: #2f7d4f; code-bg: #f0f0f3; user-bubble: #ebebf0; claude: #c8663d; codex: #4f6fd8`);
const DOOD_DARK = css(`
	background: #18181b; foreground: #e7e7ea; card: #1f1f23; card-foreground: #e7e7ea; popover: #232327; popover-foreground: #e7e7ea;
	primary: #9aa4ff; primary-foreground: #14152a; secondary: #26262b; secondary-foreground: #e7e7ea; muted: #232327; muted-foreground: #9a9ba3;
	accent: #2b2b31; accent-foreground: #e7e7ea; destructive: #f07167; border: #2e2e34; input: #35353c; ring: #9aa4ff;
	sidebar: #141417; sidebar-foreground: #d9d9de; sidebar-primary: #9aa4ff; sidebar-primary-foreground: #14152a; sidebar-accent: #222226;
	sidebar-accent-foreground: #e7e7ea; sidebar-border: #26262b; sidebar-ring: #9aa4ff;
	attention: #f5b041; attention-foreground: #1f1605; ok: #6cc48f; code-bg: #131316; user-bubble: #26262c; claude: #e58a5f; codex: #8aa6ff`);

// Everything else ------------------------------------------------------------

export const PRESETS: Theme[] = [
	glaude("sade-light", "Sade Light", "light", SADE_LIGHT, syn("#8a8f9c", "#3d6fd8", "#15803d", "#b45309", "#c8663d", "#8250df", "#9a3412", "#d92d20", "#b45309", "#3d6fd8"), "#2563eb"),
	glaude("sade-dark", "Sade Dark", "dark", SADE_DARK, syn("#6f7380", "#7aa2ff", "#4ade80", "#f5b041", "#e58a5f", "#c4a7ff", "#f0a37a", "#f97066", "#f5b041", "#7aa2ff"), "#7aa7ff"),
	glaude("lajevard-light", "Lajevard Light", "light", LAJEVARD_LIGHT, syn("#7d8aa3", "#3d6fd8", "#2f855a", "#b7791f", "#c8663d", "#0f7f76", "#8a3fb8", "#c2413b", "#b7791f", "#0f7f76"), "#2459b8"),
	glaude("lajevard-dark", "Lajevard Dark", "dark", LAJEVARD_DARK, syn("#6f7f9c", "#7aa2ff", "#63c98b", "#f2b544", "#e58a5f", "#5cc8bd", "#c79cf2", "#ef6b6b", "#f2b544", "#5cc8bd"), "#7aa7ff"),
	glaude("dood-light", "Dood Light", "light", DOOD_LIGHT, syn("#8b8d96", "#4f5bd5", "#2f7d4f", "#b45309", "#c8663d", "#8a3fb8", "#9a3412", "#c4372f", "#b45309", "#4f5bd5"), "#4f5bd5"),
	glaude("dood-dark", "Dood Dark", "dark", DOOD_DARK, syn("#75767f", "#9aa4ff", "#6cc48f", "#f5b041", "#e58a5f", "#c79cf2", "#f0a37a", "#f07167", "#f5b041", "#9aa4ff"), "#9aa4ff"),

	{
		id: "github-light",
		name: "GitHub Light",
		family: "GitHub",
		kind: "light",
		bg: "#ffffff",
		fg: "#1f2328",
		card: "#ffffff",
		sidebar: "#f6f8fa",
		muted: "#59636e",
		accent: "#0969da",
		red: "#cf222e",
		green: "#1a7f37",
		yellow: "#9a6700",
		blue: "#0969da",
		codeBg: "#f6f8fa",
		ansi: ansi("#24292f #cf222e #116329 #4d2d00 #0969da #8250df #1b7c83 #6e7781 #57606a #a40e26 #1a7f37 #633c01 #218bff #a475f9 #3192aa #8c959f"),
		syntax: syn("#59636e", "#cf222e", "#0a3069", "#0550ae", "#8250df", "#953800", "#953800", "#116329", "#0550ae", "#cf222e"),
		vars: { border: "#d1d9e0", "sidebar-border": "#d1d9e0" },
	},
	{
		id: "github-dark",
		name: "GitHub Dark",
		family: "GitHub",
		kind: "dark",
		bg: "#0d1117",
		fg: "#e6edf3",
		card: "#151b23",
		popover: "#151b23",
		sidebar: "#010409",
		muted: "#9198a1",
		accent: "#4493f8",
		red: "#f85149",
		green: "#3fb950",
		yellow: "#d29922",
		blue: "#4493f8",
		codeBg: "#151b23",
		ansi: ansi("#484f58 #ff7b72 #3fb950 #d29922 #58a6ff #bc8cff #39c5cf #b1bac4 #6e7681 #ffa198 #56d364 #e3b341 #79c0ff #d2a8ff #56d4dd #ffffff"),
		syntax: syn("#9198a1", "#ff7b72", "#a5d6ff", "#79c0ff", "#d2a8ff", "#ffa657", "#ffa657", "#7ee787", "#79c0ff", "#ff7b72"),
		vars: { border: "#3d444d", "sidebar-border": "#3d444d" },
	},
	{
		id: "one-light",
		name: "One Light",
		family: "One",
		kind: "light",
		bg: "#fafafa",
		fg: "#383a42",
		card: "#ffffff",
		sidebar: "#eaeaeb",
		muted: "#696c77",
		accent: "#4078f2",
		red: "#e45649",
		green: "#50a14f",
		yellow: "#c18401",
		blue: "#4078f2",
		codeBg: "#f0f0f1",
		ansi: ansi("#383a42 #e45649 #50a14f #c18401 #4078f2 #a626a4 #0184bc #a0a1a7 #696c77 #e45649 #50a14f #c18401 #4078f2 #a626a4 #0184bc #ffffff"),
		syntax: syn("#a0a1a7", "#a626a4", "#50a14f", "#986801", "#4078f2", "#c18401", "#e45649", "#e45649", "#986801", "#0184bc"),
	},
	{
		id: "one-dark",
		name: "One Dark",
		family: "One",
		kind: "dark",
		bg: "#282c34",
		fg: "#abb2bf",
		card: "#2c313a",
		popover: "#2c313a",
		sidebar: "#21252b",
		muted: "#7f848e",
		accent: "#61afef",
		red: "#e06c75",
		green: "#98c379",
		yellow: "#e5c07b",
		blue: "#61afef",
		codeBg: "#21252b",
		ansi: ansi("#3f4451 #e05561 #8cc265 #d18f52 #4aa5f0 #c162de #42b3c2 #d7dae0 #4f5666 #ff616e #a5e075 #f0a45d #4dc4ff #de73ff #4cd1e0 #e6e6e6"),
		syntax: syn("#5c6370", "#c678dd", "#98c379", "#d19a66", "#61afef", "#e5c07b", "#e06c75", "#e06c75", "#d19a66", "#56b6c2"),
	},
	{
		id: "solarized-light",
		name: "Solarized Light",
		family: "Solarized",
		kind: "light",
		bg: "#fdf6e3",
		fg: "#586e75",
		card: "#fffbef",
		sidebar: "#eee8d5",
		muted: "#657b83",
		accent: "#268bd2",
		red: "#dc322f",
		green: "#859900",
		yellow: "#b58900",
		blue: "#268bd2",
		codeBg: "#f5efdc",
		ansi: ansi("#073642 #dc322f #859900 #b58900 #268bd2 #d33682 #2aa198 #eee8d5 #586e75 #cb4b16 #859900 #b58900 #268bd2 #6c71c4 #2aa198 #fdf6e3"),
		syntax: syn("#93a1a1", "#859900", "#2aa198", "#d33682", "#268bd2", "#b58900", "#268bd2", "#268bd2", "#93a1a1", "#859900"),
	},
	{
		id: "solarized-dark",
		name: "Solarized Dark",
		family: "Solarized",
		kind: "dark",
		bg: "#002b36",
		fg: "#93a1a1",
		card: "#073642",
		popover: "#073642",
		sidebar: "#00212b",
		muted: "#839496",
		accent: "#268bd2",
		red: "#dc322f",
		green: "#859900",
		yellow: "#b58900",
		blue: "#268bd2",
		codeBg: "#00212b",
		ansi: ansi("#073642 #dc322f #859900 #b58900 #268bd2 #d33682 #2aa198 #eee8d5 #586e75 #cb4b16 #859900 #b58900 #268bd2 #6c71c4 #2aa198 #fdf6e3"),
		syntax: syn("#586e75", "#859900", "#2aa198", "#d33682", "#268bd2", "#b58900", "#268bd2", "#268bd2", "#93a1a1", "#859900"),
	},
	{
		id: "dracula",
		name: "Dracula",
		family: "Dracula",
		kind: "dark",
		bg: "#282a36",
		fg: "#f8f8f2",
		card: "#2f3141",
		popover: "#343746",
		sidebar: "#21222c",
		muted: "#a4a9c6",
		accent: "#bd93f9",
		red: "#ff5555",
		green: "#50fa7b",
		yellow: "#f1fa8c",
		blue: "#8be9fd",
		codeBg: "#21222c",
		ansi: ansi("#21222c #ff5555 #50fa7b #f1fa8c #bd93f9 #ff79c6 #8be9fd #f8f8f2 #6272a4 #ff6e6e #69ff94 #ffffa5 #d6acff #ff92df #a4ffff #ffffff"),
		syntax: syn("#6272a4", "#ff79c6", "#f1fa8c", "#bd93f9", "#50fa7b", "#8be9fd", "#ffb86c", "#ff79c6", "#50fa7b", "#ff79c6"),
	},
	{
		id: "tokyo-night",
		name: "Tokyo Night",
		family: "Tokyo Night",
		kind: "dark",
		bg: "#1a1b26",
		fg: "#c0caf5",
		card: "#1f2335",
		popover: "#1f2335",
		sidebar: "#16161e",
		muted: "#787c99",
		accent: "#7aa2f7",
		red: "#f7768e",
		green: "#9ece6a",
		yellow: "#e0af68",
		blue: "#7aa2f7",
		codeBg: "#16161e",
		ansi: ansi("#15161e #f7768e #9ece6a #e0af68 #7aa2f7 #bb9af7 #7dcfff #a9b1d6 #414868 #ff899d #9fe044 #faba4a #8db0ff #c7a9ff #a4daff #c0caf5"),
		syntax: syn("#565f89", "#bb9af7", "#9ece6a", "#ff9e64", "#7aa2f7", "#2ac3de", "#e0af68", "#f7768e", "#bb9af7", "#89ddff"),
	},
	{
		id: "tokyo-night-storm",
		name: "Tokyo Night Storm",
		family: "Tokyo Night",
		kind: "dark",
		bg: "#24283b",
		fg: "#c0caf5",
		card: "#292e42",
		popover: "#292e42",
		sidebar: "#1f2335",
		muted: "#7f85a8",
		accent: "#7aa2f7",
		red: "#f7768e",
		green: "#9ece6a",
		yellow: "#e0af68",
		blue: "#7aa2f7",
		codeBg: "#1f2335",
		ansi: ansi("#1d202f #f7768e #9ece6a #e0af68 #7aa2f7 #bb9af7 #7dcfff #a9b1d6 #414868 #ff899d #9fe044 #faba4a #8db0ff #c7a9ff #a4daff #c0caf5"),
		syntax: syn("#565f89", "#bb9af7", "#9ece6a", "#ff9e64", "#7aa2f7", "#2ac3de", "#e0af68", "#f7768e", "#bb9af7", "#89ddff"),
	},
	{
		id: "nord",
		name: "Nord",
		family: "Nord",
		kind: "dark",
		bg: "#2e3440",
		fg: "#d8dee9",
		card: "#3b4252",
		popover: "#3b4252",
		sidebar: "#2a2f3a",
		muted: "#8d96a8",
		accent: "#88c0d0",
		red: "#bf616a",
		green: "#a3be8c",
		yellow: "#ebcb8b",
		blue: "#81a1c1",
		codeBg: "#292e39",
		ansi: ansi("#3b4252 #bf616a #a3be8c #ebcb8b #81a1c1 #b48ead #88c0d0 #e5e9f0 #4c566a #bf616a #a3be8c #ebcb8b #81a1c1 #b48ead #8fbcbb #eceff4"),
		syntax: syn("#616e88", "#81a1c1", "#a3be8c", "#b48ead", "#88c0d0", "#8fbcbb", "#d8dee9", "#81a1c1", "#8fbcbb", "#81a1c1"),
	},
	{
		id: "catppuccin-latte",
		name: "Catppuccin Latte",
		family: "Catppuccin",
		kind: "light",
		bg: "#eff1f5",
		fg: "#4c4f69",
		card: "#f5f6f9",
		sidebar: "#e6e9ef",
		muted: "#6c6f85",
		accent: "#8839ef",
		red: "#d20f39",
		green: "#40a02b",
		yellow: "#df8e1d",
		blue: "#1e66f5",
		codeBg: "#e6e9ef",
		ansi: ansi("#5c5f77 #d20f39 #40a02b #df8e1d #1e66f5 #ea76cb #179299 #acb0be #6c6f85 #de293e #49af3d #eea02d #456eff #fe85d8 #2d9fa8 #bcc0cc"),
		syntax: syn("#7c7f93", "#8839ef", "#40a02b", "#fe640b", "#1e66f5", "#df8e1d", "#e64553", "#1e66f5", "#df8e1d", "#04a5e5"),
	},
	{
		id: "catppuccin-mocha",
		name: "Catppuccin Mocha",
		family: "Catppuccin",
		kind: "dark",
		bg: "#1e1e2e",
		fg: "#cdd6f4",
		card: "#232334",
		popover: "#262637",
		sidebar: "#181825",
		muted: "#a6adc8",
		accent: "#cba6f7",
		red: "#f38ba8",
		green: "#a6e3a1",
		yellow: "#f9e2af",
		blue: "#89b4fa",
		codeBg: "#181825",
		ansi: ansi("#45475a #f38ba8 #a6e3a1 #f9e2af #89b4fa #f5c2e7 #94e2d5 #bac2de #585b70 #f38ba8 #a6e3a1 #f9e2af #89b4fa #f5c2e7 #94e2d5 #a6adc8"),
		syntax: syn("#9399b2", "#cba6f7", "#a6e3a1", "#fab387", "#89b4fa", "#f9e2af", "#eba0ac", "#89b4fa", "#f9e2af", "#89dceb"),
	},
	{
		id: "rose-pine",
		name: "Rosé Pine",
		family: "Rosé Pine",
		kind: "dark",
		bg: "#191724",
		fg: "#e0def4",
		card: "#1f1d2e",
		popover: "#1f1d2e",
		sidebar: "#16141f",
		muted: "#908caa",
		accent: "#c4a7e7",
		red: "#eb6f92",
		green: "#9ccfd8",
		yellow: "#f6c177",
		blue: "#9ccfd8",
		codeBg: "#1f1d2e",
		ansi: ansi("#26233a #eb6f92 #31748f #f6c177 #9ccfd8 #c4a7e7 #ebbcba #e0def4 #6e6a86 #eb6f92 #31748f #f6c177 #9ccfd8 #c4a7e7 #ebbcba #e0def4"),
		syntax: syn("#6e6a86", "#31748f", "#f6c177", "#ebbcba", "#ebbcba", "#9ccfd8", "#c4a7e7", "#9ccfd8", "#c4a7e7", "#908caa"),
	},
	{
		id: "rose-pine-moon",
		name: "Rosé Pine Moon",
		family: "Rosé Pine",
		kind: "dark",
		bg: "#232136",
		fg: "#e0def4",
		card: "#2a273f",
		popover: "#2a273f",
		sidebar: "#1f1d30",
		muted: "#908caa",
		accent: "#c4a7e7",
		red: "#eb6f92",
		green: "#9ccfd8",
		yellow: "#f6c177",
		blue: "#9ccfd8",
		codeBg: "#2a273f",
		ansi: ansi("#393552 #eb6f92 #3e8fb0 #f6c177 #9ccfd8 #c4a7e7 #ea9a97 #e0def4 #6e6a86 #eb6f92 #3e8fb0 #f6c177 #9ccfd8 #c4a7e7 #ea9a97 #e0def4"),
		syntax: syn("#6e6a86", "#3e8fb0", "#f6c177", "#ea9a97", "#ea9a97", "#9ccfd8", "#c4a7e7", "#9ccfd8", "#c4a7e7", "#908caa"),
	},
	{
		id: "rose-pine-dawn",
		name: "Rosé Pine Dawn",
		family: "Rosé Pine",
		kind: "light",
		bg: "#faf4ed",
		fg: "#575279",
		card: "#fffaf3",
		popover: "#fffaf3",
		sidebar: "#f2e9e1",
		muted: "#797593",
		accent: "#907aa9",
		red: "#b4637a",
		green: "#286983",
		yellow: "#ea9d34",
		blue: "#286983",
		codeBg: "#f4ede8",
		ansi: ansi("#f2e9e1 #b4637a #286983 #ea9d34 #56949f #907aa9 #d7827e #575279 #9893a5 #b4637a #286983 #ea9d34 #56949f #907aa9 #d7827e #575279"),
		syntax: syn("#9893a5", "#286983", "#ea9d34", "#d7827e", "#d7827e", "#56949f", "#907aa9", "#56949f", "#907aa9", "#797593"),
	},
	{
		id: "gruvbox-light",
		name: "Gruvbox Light",
		family: "Gruvbox",
		kind: "light",
		bg: "#fbf1c7",
		fg: "#3c3836",
		card: "#f9f5d7",
		popover: "#f9f5d7",
		sidebar: "#f2e5bc",
		muted: "#7c6f64",
		accent: "#af3a03",
		red: "#9d0006",
		green: "#79740e",
		yellow: "#b57614",
		blue: "#076678",
		codeBg: "#f2e5bc",
		ansi: ansi("#fbf1c7 #cc241d #98971a #d79921 #458588 #b16286 #689d6a #7c6f64 #928374 #9d0006 #79740e #b57614 #076678 #8f3f71 #427b58 #3c3836"),
		syntax: syn("#928374", "#9d0006", "#79740e", "#8f3f71", "#427b58", "#b57614", "#076678", "#9d0006", "#b57614", "#af3a03"),
	},
	{
		id: "gruvbox-dark",
		name: "Gruvbox Dark",
		family: "Gruvbox",
		kind: "dark",
		bg: "#282828",
		fg: "#ebdbb2",
		card: "#32302f",
		popover: "#3c3836",
		sidebar: "#1d2021",
		muted: "#a89984",
		accent: "#fe8019",
		red: "#fb4934",
		green: "#b8bb26",
		yellow: "#fabd2f",
		blue: "#83a598",
		codeBg: "#1d2021",
		ansi: ansi("#282828 #cc241d #98971a #d79921 #458588 #b16286 #689d6a #a89984 #928374 #fb4934 #b8bb26 #fabd2f #83a598 #d3869b #8ec07c #ebdbb2"),
		syntax: syn("#928374", "#fb4934", "#b8bb26", "#d3869b", "#8ec07c", "#fabd2f", "#83a598", "#fb4934", "#fabd2f", "#fe8019"),
	},
	{
		id: "everforest-light",
		name: "Everforest Light",
		family: "Everforest",
		kind: "light",
		bg: "#fdf6e3",
		fg: "#5c6a72",
		card: "#fffbef",
		popover: "#fffbef",
		sidebar: "#f4f0d9",
		muted: "#829181",
		accent: "#35a77c",
		red: "#f85552",
		green: "#8da101",
		yellow: "#dfa000",
		blue: "#3a94c5",
		codeBg: "#f4f0d9",
		ansi: ansi("#5c6a72 #f85552 #8da101 #dfa000 #3a94c5 #df69ba #35a77c #e6e2cc #829181 #f85552 #8da101 #dfa000 #3a94c5 #df69ba #35a77c #fdf6e3"),
		syntax: syn("#939f91", "#f85552", "#8da101", "#df69ba", "#35a77c", "#dfa000", "#3a94c5", "#f57d26", "#dfa000", "#f57d26"),
	},
	{
		id: "everforest-dark",
		name: "Everforest Dark",
		family: "Everforest",
		kind: "dark",
		bg: "#2d353b",
		fg: "#d3c6aa",
		card: "#343f44",
		popover: "#343f44",
		sidebar: "#232a2e",
		muted: "#9da9a0",
		accent: "#a7c080",
		red: "#e67e80",
		green: "#a7c080",
		yellow: "#dbbc7f",
		blue: "#7fbbb3",
		codeBg: "#232a2e",
		ansi: ansi("#475258 #e67e80 #a7c080 #dbbc7f #7fbbb3 #d699b6 #83c092 #d3c6aa #7a8478 #e67e80 #a7c080 #dbbc7f #7fbbb3 #d699b6 #83c092 #d3c6aa"),
		syntax: syn("#859289", "#e67e80", "#a7c080", "#d699b6", "#83c092", "#dbbc7f", "#7fbbb3", "#e69875", "#dbbc7f", "#e69875"),
	},
];

/** Gallery order of the families. */
export const FAMILIES = ["Sade", "Lajevard", "Dood", "GitHub", "One", "Solarized", "Dracula", "Tokyo Night", "Nord", "Catppuccin", "Rosé Pine", "Gruvbox", "Everforest"];

/** Accent colours on top of any theme: [light, dark]. */
export const ACCENTS: { id: string; name: string; light: string; dark: string }[] = [
	{ id: "blue", name: "Blue", light: "#2563eb", dark: "#60a5fa" },
	{ id: "violet", name: "Violet", light: "#7c3aed", dark: "#a78bfa" },
	{ id: "pink", name: "Pink", light: "#db2777", dark: "#f472b6" },
	{ id: "red", name: "Red", light: "#dc2626", dark: "#f87171" },
	{ id: "orange", name: "Orange", light: "#ea580c", dark: "#fb923c" },
	{ id: "amber", name: "Amber", light: "#b45309", dark: "#fbbf24" },
	{ id: "green", name: "Green", light: "#16a34a", dark: "#4ade80" },
	{ id: "teal", name: "Teal", light: "#0d9488", dark: "#2dd4bf" },
	{ id: "graphite", name: "Graphite", light: "#111318", dark: "#f1f2f5" },
];

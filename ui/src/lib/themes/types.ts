// A theme colours the app, code (chat, files, diffs) and terminals.

export type Kind = "light" | "dark";

export const SYNTAX_KEYS = ["comment", "keyword", "string", "number", "function", "type", "variable", "tag", "attribute", "operator"] as const;
export type SyntaxKey = (typeof SYNTAX_KEYS)[number];
export type Syntax = Record<SyntaxKey, string>;

/** xterm order: black red green yellow blue magenta cyan white, then the bright ones. */
export const ANSI_NAMES = [
	"black",
	"red",
	"green",
	"yellow",
	"blue",
	"magenta",
	"cyan",
	"white",
	"brightBlack",
	"brightRed",
	"brightGreen",
	"brightYellow",
	"brightBlue",
	"brightMagenta",
	"brightCyan",
	"brightWhite",
] as const;

export interface Theme {
	id: string;
	name: string;
	/** Group in the gallery ("GitHub", "Catppuccin", "Yours"). */
	family: string;
	kind: Kind;
	/** Window and editor ground, and text. */
	bg: string;
	fg: string;
	/** Raised surfaces; derived from bg when missing. */
	card?: string;
	popover?: string;
	sidebar?: string;
	/** Secondary text; derived when missing. */
	muted?: string;
	/** Buttons, focus, selection. */
	accent: string;
	/** Status colours (errors, success, attention, links). */
	red: string;
	green: string;
	yellow: string;
	blue: string;
	/** Code blocks and terminal panel ground; derived when missing. */
	codeBg?: string;
	/** Terminal ground; bg when missing. */
	termBg?: string;
	ansi: string[];
	syntax: Syntax;
	/** Exact CSS variables that win over the derived ones (ported palettes). */
	vars?: Record<string, string>;
	/** Made or imported by the user. */
	custom?: boolean;
}

// xterm.js instances live here, not in the tab component, so a terminal
// keeps its PTY and scrollback when its tab moves to another pane or is
// hidden. The component only lends it a host element.

import { Terminal, type ITheme } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { FitAddon } from "@xterm/addon-fit";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { call, channel, inTauri } from "$lib/ipc";
import { openLink, showError } from "./sessions";
import type { TermSpec } from "./layout.svelte";
import { prefs } from "./prefs.svelte";
import { proxies } from "./proxies.svelte";
import { themes } from "$lib/themes/themes.svelte";
import { xtermTheme } from "$lib/themes/vars";

interface Entry {
	term: Terminal;
	fit: FitAddon;
	el: HTMLDivElement;
	spec: TermSpec;
	/** PTY id; changes on restart. */
	ptyId: string | null;
	generation: number;
	exited: boolean;
	cols: number;
	rows: number;
}

const entries = new Map<string, Entry>();

/** Told when a shell sets its title (for terminal names in the panel). */
let titleHandler: ((id: string, title: string) => void) | null = null;
export function onTitle(fn: (id: string, title: string) => void) {
	titleHandler = fn;
}

/** Colours from Settings → Appearance → Terminal theme. */
function theme(): ITheme {
	return xtermTheme(themes.terminal, prefs.accent);
}

/** Font and cursor from Settings → Terminal. */
function termOptions() {
	return {
		fontFamily: `${prefs.fontStack(prefs.termFont, prefs.termFontCustom)}, "Estedad Variable"`,
		fontSize: prefs.termFontSize,
		cursorStyle: prefs.termCursor,
		cursorBlink: prefs.termCursorBlink,
	};
}

/** Apply changed terminal settings to every open terminal. */
export function reconfigure() {
	const o = termOptions();
	for (const [id, e] of entries) {
		Object.assign(e.term.options, o);
		refit(id);
	}
}

/** Re-theme every terminal (mode, theme or accent change). */
export function retheme() {
	const t = theme();
	for (const e of entries.values()) e.term.options.theme = t;
}

function create(tabId: string, spec: TermSpec): Entry {
	const el = document.createElement("div");
	el.className = "h-full w-full";
	const term = new Terminal({
		allowProposedApi: true,
		...termOptions(),
		lineHeight: 1.15,
		scrollback: 10000,
		macOptionIsMeta: true,
		theme: theme(),
	});
	const fit = new FitAddon();
	term.loadAddon(fit);
	term.loadAddon(new Unicode11Addon());
	term.unicode.activeVersion = "11";
	term.loadAddon(new WebLinksAddon((_e, uri) => openLink(uri)));
	term.onTitleChange((title) => titleHandler?.(tabId, title));
	const entry: Entry = { term, fit, el, spec, ptyId: null, generation: 0, exited: false, cols: 80, rows: 24 };
	term.onData((data) => {
		if (entry.exited) {
			if (data === "\r") void start(tabId, entry);
			return;
		}
		if (entry.ptyId) call("pty_write", { id: entry.ptyId, data }).catch(() => {});
	});
	entries.set(tabId, entry);
	return entry;
}

async function start(tabId: string, entry: Entry) {
	if (entry.ptyId) await call("pty_close", { id: entry.ptyId }).catch(() => {});
	entry.generation += 1;
	const id = `${tabId}:${entry.generation}`;
	entry.ptyId = id;
	entry.exited = false;
	await proxies.ready();
	// Undefined: the defaults (an agent's account and project, or the shell
	// default); "none" or null: no proxy.
	const s = entry.spec;
	const proxy =
		s.proxy !== undefined
			? s.proxy && s.proxy !== "none"
				? s.proxy
				: null
			: s.run
				? proxies.defaultFor({ provider: s.run.provider, account: s.run.account, cwd: s.cwd }).id
				: proxies.forShell(s.cwd);
	const onData = await channel<ArrayBuffer | number[] | Uint8Array>((chunk) => {
		if (entry.ptyId !== id) return;
		entry.term.write(chunk instanceof Uint8Array ? chunk : new Uint8Array(chunk as ArrayBuffer));
	});
	const onExit = await channel<number | null>((code) => {
		if (entry.ptyId !== id) return;
		entry.exited = true;
		const note = code == null || code === 0 ? "Process exited" : `Process exited with code ${code}`;
		entry.term.write(`\r\n\x1b[2m${note}. Press Enter to start it again.\x1b[0m\r\n`);
	});
	try {
		await call("pty_open", {
			id,
			spec: {
				cwd: entry.spec.cwd ?? null,
				cols: entry.cols,
				rows: entry.rows,
				run: entry.spec.run ?? null,
				shell: prefs.termShell.trim() || null,
				proxy,
			},
			onData: onData.raw,
			onExit: onExit.raw,
		});
		// A project action: type its command into the new shell (once).
		if (entry.spec.command && entry.generation === 1) await call("pty_write", { id, data: entry.spec.command + "\r" });
	} catch (e) {
		entry.exited = true;
		showError("Could not start the terminal", e);
	}
}

/** Mount the terminal for `tabId` into `host`, creating it on first use. */
export function attach(tabId: string, spec: TermSpec, host: HTMLElement): void {
	let entry = entries.get(tabId);
	const fresh = !entry;
	if (!entry) entry = create(tabId, spec);
	host.appendChild(entry.el);
	if (fresh) entry.term.open(entry.el);
	refit(tabId);
	if (fresh) {
		void enableWebgl(tabId);
		void start(tabId, entry);
	} else entry.term.refresh(0, entry.term.rows - 1);
}

/** Take the element out of the DOM without ending the process. */
export function detach(tabId: string): void {
	entries.get(tabId)?.el.remove();
}

export function refit(tabId: string): void {
	const e = entries.get(tabId);
	if (!e || !e.el.isConnected || e.el.clientWidth === 0) return;
	try {
		e.fit.fit();
	} catch {
		return;
	}
	const { cols, rows } = e.term;
	if (cols === e.cols && rows === e.rows) return;
	e.cols = cols;
	e.rows = rows;
	if (e.ptyId && !e.exited) call("pty_resize", { id: e.ptyId, cols, rows }).catch(() => {});
}

export function focus(tabId: string): void {
	entries.get(tabId)?.term.focus();
}

/** End the process and free the terminal (tab closed). */
export function dispose(tabId: string): void {
	const e = entries.get(tabId);
	if (!e) return;
	entries.delete(tabId);
	if (e.ptyId) call("pty_close", { id: e.ptyId }).catch(() => {});
	e.term.dispose();
	e.el.remove();
}

/** WebGL renderer for heavy output; falls back to the DOM renderer. */
async function enableWebgl(tabId: string): Promise<void> {
	const e = entries.get(tabId);
	if (!e || !inTauri) return;
	try {
		const { WebglAddon } = await import("@xterm/addon-webgl");
		const gl = new WebglAddon();
		gl.onContextLoss(() => gl.dispose());
		e.term.loadAddon(gl);
	} catch {
		/* DOM renderer is fine */
	}
}

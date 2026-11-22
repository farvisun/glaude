// App commands: run from the macOS menu bar (Rust emits "menu" events) and,
// off macOS or in the browser, from keyboard shortcuts handled here.
import { setMode } from "mode-watcher";
import { call, inTauri } from "$lib/ipc";
import { layout } from "./layout.svelte";
import { conversations } from "./conversations.svelte";
import { ui } from "./ui.svelte";
import { prefs, isMac } from "./prefs.svelte";
import { convActions } from "./sessions";
import { badges } from "./badges.svelte";
import { stepOf, pendingOf, byUrgency } from "./activity";
import { fillPrompt, variablesOf, type SavedPrompt } from "./prompts.svelte";
import type { Conversation } from "./conversations.svelte";
import { toast } from "svelte-sonner";

/** The conversation in the focused pane's active tab. */
export function currentConv() {
	const tab = layout.focusedLeaf.active ? layout.tabs[layout.focusedLeaf.active] : undefined;
	return tab?.kind === "conversation" && tab.ref ? conversations.get(tab.ref) : undefined;
}

/** Show a conversation's tab (and scroll to an item in it). */
export function showConversation(convId: string, itemId?: string) {
	const tab = Object.values(layout.tabs).find((t) => t.kind === "conversation" && t.ref === convId);
	if (!tab) return;
	layout.activate(tab.id);
	if (itemId) ui.reveal = { convId, itemId };
}

/** Put a saved prompt into a conversation's composer; ask for its parts first. */
export function usePrompt(p: SavedPrompt, conv: Conversation) {
	if (variablesOf(p.body).length) {
		ui.promptFill = { prompt: p, convId: conv.id };
		return;
	}
	conv.draft = fillPrompt(p.body, {}, { cwd: conv.cwd, branch: conv.session.gitBranch });
	showConversation(conv.id);
}

/** Jump to the next agent waiting for you (longest waiting first). */
export function nextWaiting() {
	const waiting = Object.values(conversations.byId)
		.map((conv) => ({ conv, step: stepOf(conv) }))
		.filter((r) => r.step.tone === "waiting")
		.sort(byUrgency);
	if (!waiting.length) {
		toast("No agent is waiting for you");
		return;
	}
	const cur = currentConv()?.id;
	const i = waiting.findIndex((r) => r.conv.id === cur);
	const next = waiting[(i + 1) % waiting.length].conv;
	showConversation(next.id, pendingOf(next)?.id);
}

function cycleTab(step: number) {
	const leaf = layout.focusedLeaf;
	if (!leaf.tabs.length) return;
	const i = Math.max(0, leaf.tabs.indexOf(leaf.active ?? ""));
	layout.activate(leaf.tabs[(i + step + leaf.tabs.length) % leaf.tabs.length]);
}

function split(edge: "right" | "bottom") {
	const leaf = layout.focusedLeaf;
	const tab = leaf.active;
	if (!tab) return;
	if (leaf.tabs.length === 1) layout.splitNew(leaf.id, edge);
	else layout.splitWith(leaf.id, edge, tab);
}

function openSettings(section?: string) {
	if (section) ui.settingsSection = section;
	layout.openTab({ kind: "settings", title: "Settings" });
}

export const actions: Record<string, () => void> = {
	settings: () => openSettings(),
	agents: () => layout.openTab({ kind: "agents", title: "Agents" }),
	usage: () => layout.openTab({ kind: "usage", title: "Usage" }),
	"next-waiting": () => nextWaiting(),
	find: () => {
		const c = currentConv();
		if (c) {
			ui.findOpen = c.id;
			ui.findSeq++;
		}
	},
	palette: () => (ui.paletteOpen = !ui.paletteOpen),
	review: () => {
		const c = currentConv();
		if (c) void convActions.crossReview(c);
		else toast("Open a conversation first");
	},
	"new-prompt": () => (ui.promptEdit = "new"),
	prompts: () => openSettings("prompts"),
	shortcuts: () => openSettings("keys"),
	"new-conversation": () => ui.openNewConversation(),
	"new-terminal": () => ui.openTerminal(),
	"open-folder": () => {
		call<string | null>("pick_folder")
			.then((dir) => dir && ui.openNewConversation(dir))
			.catch(() => {});
	},
	"close-tab": () => {
		const active = layout.focusedLeaf.active;
		if (active) layout.closeTab(active);
	},
	search: () => (ui.searchOpen = !ui.searchOpen),
	"toggle-sidebar": () => ui.toggleSidebar(),
	"toggle-progress": () => prefs.set("progressPanel", !prefs.progressPanel),
	"toggle-terminal": () => {
		const c = currentConv();
		if (c) c.drawer = !c.drawer;
	},
	"zoom-in": () => prefs.zoom(0.1),
	"zoom-out": () => prefs.zoom(-0.1),
	"zoom-reset": () => prefs.zoom(0),
	"theme-system": () => setMode("system"),
	"theme-light": () => setMode("light"),
	"theme-dark": () => setMode("dark"),
	changes: () => {
		const c = currentConv();
		if (c) ui.openChanges(c.cwd);
	},
	"side-chat": () => {
		const c = currentConv();
		if (c) void convActions.sideChat(c);
	},
	"session-details": () => {
		const c = currentConv();
		if (c) ui.sessionPanel = { convId: c.id };
	},
	stop: () => {
		const c = currentConv();
		if (c?.running) void convActions.interrupt(c);
	},
	"split-right": () => split("right"),
	"split-down": () => split("bottom"),
	"next-tab": () => cycleTab(1),
	"prev-tab": () => cycleTab(-1),
};

const k = isMac ? { mod: "⌘", shift: "⇧", alt: "⌥", ctrl: "⌃" } : { mod: "Ctrl+", shift: "Shift+", alt: "Alt+", ctrl: "Ctrl+" };

/** Every action with its label and shortcut, for the command palette. */
export const commandList: { id: string; label: string; group: string; keys?: string; words?: string }[] = [
	{ id: "new-conversation", label: "New conversation", group: "Conversation", keys: `${k.mod}N` },
	{ id: "new-terminal", label: "New terminal", group: "Conversation", keys: `${k.mod}T` },
	{ id: "open-folder", label: "Open folder…", group: "Conversation", keys: `${k.mod}O` },
	{ id: "find", label: "Find in conversation", group: "Conversation", keys: `${k.mod}F`, words: "search" },
	{ id: "search", label: "Search history", group: "Conversation", keys: `${k.mod}K` },
	{ id: "changes", label: "Changes", group: "Conversation", keys: `${k.shift}${k.mod}G`, words: "diff git commit" },
	{ id: "review", label: "Ask the other tool to review the changes", group: "Conversation", keys: `${k.shift}${k.mod}R`, words: "codex claude cross review" },
	{ id: "side-chat", label: "Side chat", group: "Conversation", words: "btw fork" },
	{ id: "session-details", label: "Session details", group: "Conversation", keys: `${k.mod}I` },
	{ id: "stop", label: "Stop agent", group: "Conversation", keys: `${k.mod}.`, words: "interrupt" },
	{ id: "close-tab", label: "Close tab", group: "Conversation", keys: `${k.mod}W` },
	{ id: "agents", label: "Agents board", group: "Agents", keys: `${k.alt}${k.mod}A`, words: "dashboard" },
	{ id: "next-waiting", label: "Next agent waiting for you", group: "Agents", keys: `${k.shift}${k.mod}A`, words: "approval inbox" },
	{ id: "usage", label: "Usage and cost", group: "Agents", words: "tokens limits" },
	{ id: "new-prompt", label: "New saved prompt…", group: "Prompts" },
	{ id: "prompts", label: "Manage saved prompts", group: "Prompts" },
	{ id: "split-right", label: "Split right", group: "View", keys: `${k.mod}\\` },
	{ id: "split-down", label: "Split down", group: "View", keys: `${k.shift}${k.mod}\\` },
	{ id: "next-tab", label: "Next tab", group: "View", keys: `${k.ctrl}Tab` },
	{ id: "prev-tab", label: "Previous tab", group: "View", keys: `${k.shift}${k.ctrl}Tab` },
	{ id: "toggle-sidebar", label: "Toggle sidebar", group: "View", keys: `${k.mod}B` },
	{ id: "toggle-progress", label: "Toggle progress panel", group: "View", keys: `${k.alt}${k.mod}P` },
	{ id: "toggle-terminal", label: "Toggle terminal", group: "View", keys: `${k.ctrl}\`` },
	{ id: "zoom-in", label: "Zoom in", group: "View", keys: `${k.mod}+` },
	{ id: "zoom-out", label: "Zoom out", group: "View", keys: `${k.mod}−` },
	{ id: "zoom-reset", label: "Actual size", group: "View", keys: `${k.mod}0` },
	{ id: "theme-system", label: "Theme: system", group: "View" },
	{ id: "theme-light", label: "Theme: light", group: "View" },
	{ id: "theme-dark", label: "Theme: dark", group: "View" },
	{ id: "settings", label: "Settings", group: "Glaude", keys: `${k.mod},`, words: "preferences" },
	{ id: "shortcuts", label: "Keyboard shortcuts", group: "Glaude" },
];

export function run(id: string) {
	if (id.startsWith("open-conv:")) {
		const convId = id.slice("open-conv:".length);
		const conv = conversations.get(convId);
		showConversation(convId, conv ? pendingOf(conv)?.id : undefined);
		return;
	}
	actions[id]?.();
}

/** Shortcuts the macOS menu owns; the page handles them elsewhere. */
const menuOwned = isMac && inTauri;

/** Keyboard shortcuts. Returns true when handled. */
export function onKey(e: KeyboardEvent): boolean {
	const mod = isMac ? e.metaKey : e.ctrlKey;
	const key = e.key.toLowerCase();
	// Cmd/Ctrl+1…9: tabs of the focused pane (9 is the last). Not in the menu.
	if (mod && !e.shiftKey && !e.altKey && /^[1-9]$/.test(e.key)) {
		const tabs = layout.focusedLeaf.tabs;
		const n = Number(e.key);
		const id = n === 9 ? tabs[tabs.length - 1] : tabs[n - 1];
		if (id) layout.activate(id);
		return true;
	}
	if (menuOwned) return false;
	const combos: [boolean, string][] = [
		[mod && !e.shiftKey && key === "n", "new-conversation"],
		[mod && !e.shiftKey && key === "t", "new-terminal"],
		[mod && !e.shiftKey && key === "o", "open-folder"],
		[mod && !e.shiftKey && key === "w", "close-tab"],
		[mod && key === "k", "search"],
		[mod && e.shiftKey && key === "a", "next-waiting"],
		[mod && e.altKey && !e.shiftKey && (key === "a" || e.code === "KeyA"), "agents"],
		[mod && e.shiftKey && key === "p", "palette"],
		[mod && !e.shiftKey && !e.altKey && key === "f", "find"],
		[mod && !e.shiftKey && key === "b", "toggle-sidebar"],
		[mod && e.shiftKey && key === "g", "changes"],
		[mod && e.shiftKey && key === "r", "review"],
		[mod && key === ",", "settings"],
		[mod && (key === "=" || key === "+"), "zoom-in"],
		[mod && key === "-", "zoom-out"],
		[mod && key === "0", "zoom-reset"],
		[mod && key === ".", "stop"],
		[mod && !e.shiftKey && key === "\\", "split-right"],
		[mod && e.shiftKey && (key === "\\" || key === "|"), "split-down"],
		[e.ctrlKey && !e.shiftKey && key === "tab", "next-tab"],
		[e.ctrlKey && e.shiftKey && key === "tab", "prev-tab"],
	];
	const hit = combos.find(([ok]) => ok);
	if (!hit) return false;
	actions[hit[1]]();
	return true;
}

/** Menu bar events from Rust, and the status icon's list of agents. */
export function startCommands() {
	if (!inTauri) return;
	import("@tauri-apps/api/event").then(({ listen }) => {
		void listen<string>("menu", (e) => run(e.payload));
	});
	let timer: ReturnType<typeof setTimeout> | undefined;
	let last = "";
	$effect.root(() => {
		$effect(() => {
			const convs = Object.values(conversations.byId).map((c) => {
				const b = badges.byRef[c.id];
				const state = b === "attention" ? "waiting" : b === "running" ? "working" : b === "unread" ? "unread" : "idle";
				return { id: c.id, title: c.title, state, step: state === "idle" ? null : stepOf(c).text };
			});
			const json = JSON.stringify(convs);
			if (json === last) return;
			clearTimeout(timer);
			timer = setTimeout(() => {
				last = json;
				call("tray_update", { convs }).catch(() => {});
			}, 400);
		});
	});
}

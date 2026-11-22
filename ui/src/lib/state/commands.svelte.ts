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

/** The conversation in the focused pane's active tab. */
export function currentConv() {
	const tab = layout.focusedLeaf.active ? layout.tabs[layout.focusedLeaf.active] : undefined;
	return tab?.kind === "conversation" && tab.ref ? conversations.get(tab.ref) : undefined;
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

export function run(id: string) {
	if (id.startsWith("open-conv:")) {
		const convId = id.slice("open-conv:".length);
		const tab = Object.values(layout.tabs).find((t) => t.kind === "conversation" && t.ref === convId);
		if (tab) layout.activate(tab.id);
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
		[mod && !e.shiftKey && key === "b", "toggle-sidebar"],
		[mod && e.shiftKey && key === "g", "changes"],
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
				return { id: c.id, title: c.title, state };
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

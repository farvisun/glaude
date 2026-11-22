// OS notifications and the dock badge. Fires only when the user can't see
// the conversation: window in the background or the tab not visible.
import { inTauri } from "$lib/ipc";
import type { Conversation } from "./conversations.svelte";
import { conversations } from "./conversations.svelte";
import { layout } from "./layout.svelte";
import { prefs, isMac } from "./prefs.svelte";

let permissionGranted: boolean | null = null;

async function canNotify(): Promise<boolean> {
	if (!inTauri) return false;
	if (permissionGranted !== null) return permissionGranted;
	const n = await import("@tauri-apps/plugin-notification");
	permissionGranted = await n.isPermissionGranted();
	if (!permissionGranted) permissionGranted = (await n.requestPermission()) === "granted";
	return permissionGranted;
}

export function visible(conv: Conversation): boolean {
	if (!document.hasFocus()) return false;
	return layout.leaves().some((l) => l.active && layout.tabs[l.active]?.ref === conv.id);
}

async function send(title: string, body: string) {
	if (!(await canNotify())) return;
	const n = await import("@tauri-apps/plugin-notification");
	n.sendNotification({ title, body, ...(prefs.notifySound ? { sound: isMac ? "Glass" : "Default" } : {}) });
}

function label(conv: Conversation) {
	return `${conv.provider === "claude" ? "Claude" : "Codex"} · ${conv.title}`;
}

export function notifyTurnDone(conv: Conversation, error?: string) {
	updateBadge();
	if (visible(conv) || !(error ? prefs.notifyErrors : prefs.notifyDone)) return;
	void send(label(conv), error ? `Stopped with an error: ${error.slice(0, 120)}` : "Finished. Your turn.");
}

export function notifyPermission(conv: Conversation, tool: string) {
	updateBadge();
	if (visible(conv) || !prefs.notifyWaiting) return;
	const body =
		tool === "question" ? "Has a question for you." : tool === "plan" ? "Finished a plan for you to review." : `Needs your approval to use ${tool}.`;
	void send(label(conv), body);
	if (inTauri)
		import("@tauri-apps/api/window").then(({ getCurrentWindow }) =>
			getCurrentWindow()
				.requestUserAttention(2)
				.catch(() => {}),
		);
}

let lastBadge = -1;
export function updateBadge() {
	if (!inTauri) return;
	const n = Object.values(conversations.byId).reduce((sum, c) => sum + c.pendingPermissions, 0);
	if (n === lastBadge) return;
	lastBadge = n;
	import("@tauri-apps/api/window").then(({ getCurrentWindow }) =>
		getCurrentWindow()
			.setBadgeCount(n > 0 ? n : undefined)
			.catch(() => {}),
	);
}

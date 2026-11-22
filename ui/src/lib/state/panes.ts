// Splitting a pane from its context menu: move a tab into the new pane, or
// open a conversation or terminal there that starts from the same folder.
import { conversations } from "./conversations.svelte";
import { layout, MAX_PANES, type Edge } from "./layout.svelte";
import { startConversation } from "./sessions";
import { ui } from "./ui.svelte";

export type SplitWhat = "move" | "conversation" | "terminal";

export const canSplit = () => layout.leaves().length < MAX_PANES;

/** The tab can move into a new pane (its pane keeps at least one tab). */
export function canMove(tabId: string | null): boolean {
	const l = tabId ? layout.leafOfTab(tabId) : null;
	return !!l && l.tabs.length > 1;
}

/** Folder (and the conversation, if any) behind a tab. */
function sourceOf(tabId: string | null) {
	const tab = tabId ? layout.tabs[tabId] : undefined;
	const conv = tab?.kind === "conversation" && tab.ref ? conversations.get(tab.ref) : undefined;
	return { conv, cwd: conv?.cwd ?? tab?.term?.cwd };
}

export function splitPane(paneId: string, edge: Edge, what: SplitWhat, tabId: string | null) {
	if (what === "move") {
		if (tabId && canMove(tabId)) layout.splitWith(paneId, edge, tabId);
		return;
	}
	const { conv, cwd } = sourceOf(tabId);
	const pane = layout.splitNew(paneId, edge);
	if (!pane) return;
	if (what === "terminal") {
		ui.openTerminal(cwd ? { cwd } : {}, "Terminal", pane);
	} else if (conv) {
		void startConversation(
			{
				provider: conv.provider,
				account: conv.account,
				cwd: conv.cwd,
				model: conv.session.model,
				permissionMode: conv.session.permissionMode,
			},
			pane,
		);
	} else {
		// Nothing to copy from: ask, and open it in the new pane.
		ui.openNewConversation(cwd);
	}
}

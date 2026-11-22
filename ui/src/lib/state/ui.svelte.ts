// Transient UI state: dialogs, palettes, loading flags.
import { layout, type TermSpec } from "./layout.svelte";
import { conversations } from "./conversations.svelte";

class UiState {
	newConversationOpen = $state(false);
	/** Folder to preselect in the new-conversation dialog. */
	newConversationCwd = $state<string | null>(null);
	searchOpen = $state(false);
	sessionsLoading = $state(false);
	/** Output viewer of a background shell or monitor. */
	taskOutput = $state<{ convId: string; taskId: string } | null>(null);
	/** Session panel (context, MCP, skills, agents, hooks, memory, style). */
	sessionPanel = $state<{ convId: string; tab?: string } | null>(null);
	/** Rewind dialog for a user message. */
	rewind = $state<{ convId: string; itemId: string } | null>(null);
	/** Settings section to show next (from the menu). */
	settingsSection = $state<string | null>(null);
	/** Set by the app shell: show or hide the sidebar. */
	toggleSidebar: () => void = () => {};
	/** Actions for a path clicked in a reply. */
	pathMenu = $state<{ hit: import("./paths").PathHit; x: number; y: number } | null>(null);

	/** Tool and account to preselect (from the Accounts page). */
	newConversationPreset = $state<{ provider: "claude" | "codex"; account: string } | null>(null);

	openNewConversation(cwd?: string, preset?: { provider: "claude" | "codex"; account: string }) {
		this.newConversationCwd = cwd ?? null;
		this.newConversationPreset = preset ?? null;
		this.newConversationOpen = true;
	}

	openSearch() {
		this.searchOpen = true;
	}

	/** A new terminal starts in the folder of the conversation in focus. */
	/** The Changes tab (diffs, commit, push, PR) of a project folder. */
	openChanges(cwd: string, paneId?: string) {
		const name = cwd.split(/[\\/]/).filter(Boolean).pop() ?? cwd;
		layout.openTab({ kind: "changes", title: `Changes · ${name}`, ref: cwd }, paneId);
	}

	openTerminal(term: TermSpec = {}, title = "Terminal", paneId?: string) {
		if (!term.cwd) {
			const tab = layout.focusedLeaf.active ? layout.tabs[layout.focusedLeaf.active] : undefined;
			const conv = tab?.kind === "conversation" && tab.ref ? conversations.get(tab.ref) : undefined;
			if (conv) term = { ...term, cwd: conv.cwd };
		}
		layout.openTab({ kind: "terminal", title, term }, paneId);
	}
}

export const ui = new UiState();

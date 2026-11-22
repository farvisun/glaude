// Transient UI state: dialogs, palettes, loading flags.
import { layout, type TermSpec } from "./layout.svelte";
import { conversations } from "./conversations.svelte";

class UiState {
	newConversationOpen = $state(false);
	/** Folder to preselect in the new-conversation dialog. */
	newConversationCwd = $state<string | null>(null);
	searchOpen = $state(false);
	sessionsLoading = $state(false);

	openNewConversation(cwd?: string) {
		this.newConversationCwd = cwd ?? null;
		this.newConversationOpen = true;
	}

	openSearch() {
		this.searchOpen = true;
	}

	/** A new terminal starts in the folder of the conversation in focus. */
	openTerminal(term: TermSpec = {}, title = "Terminal") {
		if (!term.cwd) {
			const tab = layout.focusedLeaf.active ? layout.tabs[layout.focusedLeaf.active] : undefined;
			const conv = tab?.kind === "conversation" && tab.ref ? conversations.get(tab.ref) : undefined;
			if (conv) term = { ...term, cwd: conv.cwd };
		}
		layout.openTab({ kind: "terminal", title, term });
	}
}

export const ui = new UiState();

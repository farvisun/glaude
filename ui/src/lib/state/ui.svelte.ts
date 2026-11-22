// Transient UI state: dialogs, palettes, loading flags.
import { layout } from "./layout.svelte";

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

	openTerminal() {
		layout.openTab({ kind: "terminal", title: "Terminal" });
	}
}

export const ui = new UiState();
